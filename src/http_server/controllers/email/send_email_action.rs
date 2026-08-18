use std::sync::Arc;

use serde::{Deserialize, Serialize};

use my_http_server::macros::*;
use my_http_server::*;

use crate::app::AppContext;

#[http_route(
    method: "POST",
    route: "/api/email/v1/send",
    summary: "Sends an email",
    description: "Builds the MIME message and submits it to the mail server of the container. The mail server resolves the MX record of each recipient and delivers the message to it.",
    controller: "Email",
    input_data: "SendEmailHttpInput",

    result:[
        {status_code: 200, description: "Email is accepted by the mail server", model: SendEmailHttpResponse},
        {status_code: 400, description: "Email model is invalid"},
        {status_code: 500, description: "Mail server did not accept the email"},
    ]
)]
pub struct SendEmailAction {
    app: Arc<AppContext>,
}

impl SendEmailAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

async fn handle_request(
    action: &SendEmailAction,
    input_data: SendEmailHttpInput,
    _ctx: &HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let email: SendEmailHttpModel = input_data.body.deserialize_json()?;

    let result = crate::flows::send_email(&action.app, email.try_into()?).await?;

    let response = SendEmailHttpResponse {
        queue_id: result.queue_id,
        smtp_response: result.smtp_response,
    };

    HttpOutput::as_json(response).into_ok_result(true)
}

#[derive(MyHttpInput)]
pub struct SendEmailHttpInput {
    #[http_body_raw(description = "Email to send")]
    pub body: RawDataTyped<SendEmailHttpModel>,
}

#[derive(Serialize, Deserialize, Debug, Clone, MyHttpObjectStructure)]
pub struct SendEmailHttpModel {
    /// Sender email. When it is not set - default_from_email from the settings is used.
    #[serde(default)]
    pub from_email: Option<String>,
    #[serde(default)]
    pub from_name: Option<String>,
    pub to: Vec<String>,
    #[serde(default)]
    pub cc: Option<Vec<String>>,
    #[serde(default)]
    pub bcc: Option<Vec<String>>,
    pub subject: String,
    pub body: String,
    /// When true - the body is sent as text/html, otherwise as text/plain.
    #[serde(default)]
    pub is_html: Option<bool>,
    #[serde(default)]
    pub attachments: Option<Vec<EmailAttachmentHttpModel>>,
    /// Optional. 'relay' - hand the message over to the relay, 'direct' - deliver it to the
    /// mail server of the recipient bypassing the relay. Empty means what the settings say.
    #[serde(default)]
    pub delivery_mode: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, MyHttpObjectStructure)]
pub struct EmailAttachmentHttpModel {
    pub file_name: String,
    pub content_type: String,
    pub base64_content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, MyHttpObjectStructure)]
pub struct SendEmailHttpResponse {
    /// Id of the message inside the mail server queue.
    pub queue_id: Option<String>,
    pub smtp_response: String,
}
