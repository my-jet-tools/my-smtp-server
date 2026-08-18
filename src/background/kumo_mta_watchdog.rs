use std::sync::Arc;

use rust_extensions::{MyTimerTick, RepeatTimerIteration};

use crate::app::AppContext;

/// kumod is a separate process of the container - if it dies, the service compiles the
/// configuration out of the current settings again and brings it back.
pub struct KumoMtaWatchdog {
    app: Arc<AppContext>,
}

impl KumoMtaWatchdog {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl MyTimerTick for KumoMtaWatchdog {
    async fn tick(&self) -> RepeatTimerIteration {
        let settings = self.app.settings_reader.get_settings().await;
        self.app.kumo_mta.check_and_restore(&settings).await;
        RepeatTimerIteration::WithInterval
    }
}
