use std::sync::Arc;

use my_http_server::macros::*;
use my_http_server::*;
use serde::{Deserialize, Serialize};

use crate::app::AppContext;

#[http_route(
    method: "GET",
    route: "/api/checkup/v1/status",
    summary: "Checks everything the delivery depends on",
    description: "Resolves the dns records the mail delivery depends on - PTR, SPF, DKIM, DMARC - compares them with what this service is actually configured with, and reports what is in place and what is missing. Nothing is changed.",
    controller: "Checkup",

    result:[
        {status_code: 200, description: "The check up result", model: CheckupHttpResponse},
    ]
)]
pub struct GetCheckupAction {
    app: Arc<AppContext>,
}

impl GetCheckupAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &GetCheckupAction,
    _ctx: &HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let report = crate::flows::get_checkup(&action.app).await;

    let response = CheckupHttpResponse {
        status: report.get_status().as_str().to_string(),
        public_ip: report.public_ip.clone(),
        items: report
            .items
            .iter()
            .map(|item| CheckItemHttpModel {
                title: item.title.clone(),
                status: item.status.as_str().to_string(),
                message: item.message.clone(),
                expected: item.expected.clone(),
                actual: item.actual.clone(),
            })
            .collect(),
    };

    HttpOutput::as_json(response).into_ok_result(false)
}

#[derive(Serialize, Deserialize, Debug, Clone, MyHttpObjectStructure)]
pub struct CheckItemHttpModel {
    pub title: String,
    /// ok, warning or failed
    pub status: String,
    pub message: String,
    /// The value which has to be published or configured.
    pub expected: Option<String>,
    /// The value which is there right now.
    pub actual: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, MyHttpObjectStructure)]
pub struct CheckupHttpResponse {
    /// The worst status of the items: ok, warning or failed.
    pub status: String,
    /// The ip address the recipients see the mail coming from.
    pub public_ip: Option<String>,
    pub items: Vec<CheckItemHttpModel>,
}
