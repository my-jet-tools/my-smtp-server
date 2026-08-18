use std::sync::Arc;

use my_http_server::macros::*;
use my_http_server::*;
use serde::{Deserialize, Serialize};

use crate::app::{APP_NAME, APP_VERSION, AppContext};

#[http_route(
    method: "GET",
    route: "/api/isalive",
    summary: "Returns the state of the service",
    description: "Cheap health check: the service answers and tells whether the mail server process of the container is alive.",
    controller: "Monitoring",

    result:[
        {status_code: 200, description: "The service is alive", model: IsAliveHttpResponse},
    ]
)]
pub struct IsAliveAction {
    app: Arc<AppContext>,
}

impl IsAliveAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &IsAliveAction,
    _ctx: &HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let response = IsAliveHttpResponse {
        name: APP_NAME.to_string(),
        version: APP_VERSION.to_string(),
        mail_server_running: action.app.kumo_mta.is_running().await,
    };

    HttpOutput::as_json(response).into_ok_result(false)
}

#[derive(Serialize, Deserialize, Debug, Clone, MyHttpObjectStructure)]
pub struct IsAliveHttpResponse {
    pub name: String,
    pub version: String,
    /// Whether the mail server process of this container is alive.
    pub mail_server_running: bool,
}
