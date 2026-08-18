use tokio::process::Command;

pub struct OsCommandResult {
    pub exit_code: Option<i32>,
    pub std_out: String,
    pub std_err: String,
}

impl OsCommandResult {
    pub fn is_success(&self) -> bool {
        self.exit_code == Some(0)
    }

    pub fn get_output(&self) -> String {
        let std_out = self.std_out.trim();
        let std_err = self.std_err.trim();

        if std_err.is_empty() {
            return std_out.to_string();
        }

        if std_out.is_empty() {
            return std_err.to_string();
        }

        format!("{}. {}", std_out, std_err)
    }
}

pub async fn execute_os_command(command: &str, args: &[String]) -> Result<OsCommandResult, String> {
    let result = Command::new(command).args(args).output().await;

    match result {
        Ok(output) => Ok(OsCommandResult {
            exit_code: output.status.code(),
            std_out: String::from_utf8_lossy(&output.stdout).to_string(),
            std_err: String::from_utf8_lossy(&output.stderr).to_string(),
        }),
        Err(err) => Err(format!(
            "Can not execute the command '{}'. Err: {}",
            command, err
        )),
    }
}
