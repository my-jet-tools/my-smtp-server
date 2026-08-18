use std::sync::Arc;

use rust_extensions::AppStates;

use crate::{kumo_mta::KumoMta, settings::SettingsReader, smtp_client::SmtpClient};

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct AppContext {
    pub kumo_mta: KumoMta,
    pub smtp_client: SmtpClient,
    pub settings_reader: Arc<SettingsReader>,
    pub states: Arc<AppStates>,
}

impl AppContext {
    pub async fn new(settings_reader: Arc<SettingsReader>) -> Self {
        Self {
            kumo_mta: KumoMta::new(),
            smtp_client: SmtpClient::new(),
            settings_reader,
            states: Arc::new(AppStates::create_initialized()),
        }
    }
}
