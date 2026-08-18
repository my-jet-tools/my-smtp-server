use std::sync::Arc;

use crate::app::AppContext;

/// Reads the settings, compiles the kumod configuration out of them and starts kumod.
/// Panics when kumod can not be started - a container which can not send mail has to die
/// loudly instead of accepting the requests it can not fulfill.
pub async fn start_kumo_mta(app: &Arc<AppContext>) {
    let settings = app.settings_reader.get_settings().await;

    if let Err(err) = app.kumo_mta.init_and_start(&settings).await {
        panic!("Can not start KumoMTA. Err: {}", err);
    }
}
