use std::{sync::Arc, time::Duration};

use tokio::{net::TcpStream, process::Child, sync::Mutex};

use crate::settings::SettingsModel;

use super::*;

const SMTP_READY_TIMEOUT: Duration = Duration::from_secs(30);
const SMTP_READY_CHECK_INTERVAL: Duration = Duration::from_millis(250);

/// Owns the kumod process of the container: compiles its configuration out of the
/// settings model, starts it and keeps it alive.
pub struct KumoMta {
    process: Mutex<Option<Child>>,
    output: Arc<KumoMtaOutput>,
}

impl KumoMta {
    pub fn new() -> Self {
        Self {
            process: Mutex::new(None),
            output: Arc::new(KumoMtaOutput::new()),
        }
    }

    /// Writes the settings into the kumod configuration files and starts kumod.
    /// Returns after kumod is ready to accept the messages on the loopback smtp port.
    pub async fn init_and_start(&self, settings: &SettingsModel) -> Result<(), String> {
        write_kumo_mta_config(settings).await?;

        self.validate_policy().await;

        self.start_process().await?;

        self.wait_until_smtp_is_ready().await?;

        Ok(())
    }

    /// Cheap pre-flight: kumod compiles the policy, runs its init event and exits without
    /// binding anything. It is not treated as fatal on purpose - the verdict of the real
    /// start up is the one which counts - but it puts the reason into the log before the
    /// process is started for real.
    async fn validate_policy(&self) {
        let args = vec![
            "--policy".to_string(),
            POLICY_FILE.to_string(),
            "--user".to_string(),
            KUMOD_USER.to_string(),
            "--validate".to_string(),
        ];

        let result = execute_os_command(KUMOD_EXECUTABLE, &args).await;

        let message = match result {
            Ok(result) => {
                if result.is_success() {
                    return;
                }

                format!(
                    "The generated KumoMTA policy did not pass the validation. {}",
                    result.get_output()
                )
            }
            Err(err) => format!("Can not validate the generated KumoMTA policy. {}", err),
        };

        my_logger::LOGGER.write_error(
            "KumoMta::validate_policy",
            message.as_str(),
            my_logger::LogEventCtx::new(),
        );

        self.output.add_line(message).await;
    }

    /// Compiles the configuration out of the current settings and restarts kumod with it.
    pub async fn restart(&self, settings: &SettingsModel) -> Result<(), String> {
        self.kill_process().await;

        self.init_and_start(settings).await
    }

    /// Last lines kumod has written to its stdout and stderr. The same output goes to the
    /// log of the container - this is the way to read it without an access to the host.
    pub async fn get_output(&self, amount_of_lines: usize) -> Vec<String> {
        self.output.get_last_lines(amount_of_lines).await
    }

    /// Restarts kumod when it is not running anymore. Called by the background timer.
    pub async fn check_and_restore(&self, settings: &SettingsModel) {
        if self.is_running().await {
            return;
        }

        my_logger::LOGGER.write_error(
            "KumoMta::check_and_restore",
            "KumoMTA process is not running. Restarting it",
            my_logger::LogEventCtx::new(),
        );

        // The settings could have been changed while the process was down.
        if let Err(err) = write_kumo_mta_config(settings).await {
            my_logger::LOGGER.write_error(
                "KumoMta::check_and_restore",
                format!("Can not write the KumoMTA config. Err: {}", err),
                my_logger::LogEventCtx::new(),
            );

            return;
        }

        if let Err(err) = self.start_process().await {
            my_logger::LOGGER.write_error(
                "KumoMta::check_and_restore",
                format!("Can not restart KumoMTA. Err: {}", err),
                my_logger::LogEventCtx::new(),
            );
        }
    }

    pub async fn get_status(&self, settings: &SettingsModel) -> KumoMtaStatus {
        KumoMtaStatus {
            running: self.is_running().await,
            dkim_enabled: settings.get_dkim_enabled(),
            queue: get_queue_summary().await,
        }
    }

    pub async fn is_running(&self) -> bool {
        let mut write_access = self.process.lock().await;
        is_process_alive(&mut write_access)
    }

    /// kumod needs a moment to compile the policy and to bind the port - the http server
    /// of the service must not start accepting the requests before that. A policy which
    /// does not compile makes kumod exit immediately, and that has to be reported as such
    /// instead of as a timeout.
    async fn wait_until_smtp_is_ready(&self) -> Result<(), String> {
        let started_at = tokio::time::Instant::now();

        loop {
            if TcpStream::connect((LOCAL_SMTP_HOST, LOCAL_SMTP_PORT))
                .await
                .is_ok()
            {
                return Ok(());
            }

            if !self.is_running().await {
                return Err(format!(
                    "KumoMTA process is gone right after the start. Check the policy '{}' and the KumoMTA output above",
                    POLICY_FILE
                ));
            }

            if started_at.elapsed() > SMTP_READY_TIMEOUT {
                return Err(format!(
                    "KumoMTA is not listening on {}:{} after {} seconds",
                    LOCAL_SMTP_HOST,
                    LOCAL_SMTP_PORT,
                    SMTP_READY_TIMEOUT.as_secs()
                ));
            }

            tokio::time::sleep(SMTP_READY_CHECK_INTERVAL).await;
        }
    }

    async fn start_process(&self) -> Result<(), String> {
        let mut write_access = self.process.lock().await;

        if is_process_alive(&mut write_access) {
            return Ok(());
        }

        // Both streams are piped and pumped into the in-memory buffer, from where they are
        // printed to our own stdout - so the output is both in the log of the container
        // and readable remotely.
        let child = tokio::process::Command::new(KUMOD_EXECUTABLE)
            .arg("--policy")
            .arg(POLICY_FILE)
            .arg("--user")
            .arg(KUMOD_USER)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();

        let mut child = match child {
            Ok(child) => child,
            Err(err) => return Err(format!("Can not start KumoMTA. Err: {}", err)),
        };

        if let Some(std_out) = child.stdout.take() {
            start_reading_std_out(self.output.clone(), std_out);
        }

        if let Some(std_err) = child.stderr.take() {
            start_reading_std_err(self.output.clone(), std_err);
        }

        *write_access = Some(child);

        Ok(())
    }

    async fn kill_process(&self) {
        let mut write_access = self.process.lock().await;

        let Some(process) = write_access.as_mut() else {
            return;
        };

        let _ = process.kill().await;

        *write_access = None;
    }
}

fn is_process_alive(process: &mut Option<Child>) -> bool {
    let Some(process) = process.as_mut() else {
        return false;
    };

    matches!(process.try_wait(), Ok(None))
}

pub async fn get_queue_summary() -> String {
    let args = vec![
        "--endpoint".to_string(),
        format!("http://{}", KUMO_HTTP_LISTENER),
        "queue-summary".to_string(),
    ];

    match execute_os_command(KCLI_EXECUTABLE, &args).await {
        Ok(result) => result.get_output(),
        Err(err) => err,
    }
}
