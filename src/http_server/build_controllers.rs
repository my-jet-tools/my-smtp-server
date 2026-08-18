use std::sync::Arc;

use my_http_server::controllers::ControllersMiddleware;

use crate::app::AppContext;

pub fn build_controllers(app: &Arc<AppContext>) -> ControllersMiddleware {
    let mut result = ControllersMiddleware::new(None, None);

    result.register_post_action(Arc::new(super::controllers::email::SendEmailAction::new(
        app.clone(),
    )));

    result.register_get_action(Arc::new(
        super::controllers::mail_server::GetMailServerStatusAction::new(app.clone()),
    ));

    result.register_get_action(Arc::new(
        super::controllers::monitoring::IsAliveAction::new(app.clone()),
    ));

    result
}
