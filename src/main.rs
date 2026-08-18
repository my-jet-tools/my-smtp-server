use std::{sync::Arc, time::Duration};

use rust_extensions::MyTimer;

use crate::app::AppContext;

mod app;
mod background;
mod flows;
mod http_server;
mod kumo_mta;
mod mappers;
mod mcp;
mod models;
mod scripts;
mod settings;
mod smtp_client;

pub const SETTINGS_FILE_NAME: &str = "~/.my-smtp-sender";

const MAIL_SERVER_WATCHDOG_INTERVAL: Duration = Duration::from_secs(15);

#[tokio::main]
async fn main() {
    let settings_reader = crate::settings::SettingsReader::new(SETTINGS_FILE_NAME).await;
    let settings_reader = Arc::new(settings_reader);

    let app = Arc::new(AppContext::new(settings_reader).await);

    // Settings are compiled into the kumod configuration and the mail server is started
    // before the http server accepts the very first send request.
    crate::kumo_mta::start_kumo_mta(&app).await;

    crate::http_server::setup_server(&app);

    let mut watchdog_timer = MyTimer::new(MAIL_SERVER_WATCHDOG_INTERVAL);

    watchdog_timer.register_timer(
        "KumoMtaWatchdog",
        Arc::new(crate::background::KumoMtaWatchdog::new(app.clone())),
    );

    watchdog_timer.start(app.states.clone(), my_logger::LOGGER.clone());

    app.states.wait_until_shutdown().await;
}
