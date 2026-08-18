use std::sync::Arc;

use my_http_server::macros::*;
use my_http_server::*;

use crate::app::{APP_NAME, APP_VERSION, AppContext};

/// The page is a single self contained html - no build step, no static files, nothing to
/// fetch from the internet. It asks /api/checkup/v1/status and renders it.
const INDEX_HTML: &str = include_str!("index.html");

#[http_route(
    method: "GET",
    route: "/",
    summary: "The check up page",
    description: "Shows which dns records have to be published and which of them are in place already.",
    controller: "Checkup",

    result:[
        {status_code: 200, description: "The page"},
    ]
)]
pub struct IndexAction {
    _app: Arc<AppContext>,
}

impl IndexAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { _app: app }
    }
}

async fn handle_request(
    _action: &IndexAction,
    _ctx: &HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let html = INDEX_HTML
        .replace("{{APP_NAME}}", APP_NAME)
        .replace("{{APP_VERSION}}", APP_VERSION);

    HttpOutput::as_html(html).into_ok_result(false)
}
