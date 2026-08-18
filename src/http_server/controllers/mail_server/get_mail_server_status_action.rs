use std::sync::Arc;

use serde::{Deserialize, Serialize};

use my_http_server::macros::*;
use my_http_server::*;

use crate::app::AppContext;

#[http_route(
    method: "GET",
    route: "/api/mail-server/v1/status",
    summary: "Returns the status of the mail server",
    description: "Tells whether the KumoMTA process of this container is running and returns the summary of its queues.",
    controller: "MailServer",

    result:[
        {status_code: 200, description: "Mail server status", model: MailServerStatusHttpResponse},
    ]
)]
pub struct GetMailServerStatusAction {
    app: Arc<AppContext>,
}

impl GetMailServerStatusAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &GetMailServerStatusAction,
    _ctx: &HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let settings = action.app.settings_reader.get_settings().await;

    let status = action.app.kumo_mta.get_status(&settings).await;

    let response = MailServerStatusHttpResponse {
        running: status.running,
        dkim_enabled: status.dkim_enabled,
        queue: status.queue,
    };

    HttpOutput::as_json(response).into_ok_result(true)
}

#[derive(Serialize, Deserialize, Debug, Clone, MyHttpObjectStructure)]
pub struct MailServerStatusHttpResponse {
    /// Whether the KumoMTA process of this container is alive.
    pub running: bool,
    pub dkim_enabled: bool,
    /// Summary of the KumoMTA queues.
    pub queue: String,
}
