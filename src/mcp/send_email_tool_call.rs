use std::sync::Arc;

use mcp_server_middleware::*;

use serde::*;

use crate::{
    app::AppContext,
    models::{DeliveryMode, SendEmailError, SendEmailModel},
};

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct SendEmailInputData {
    #[property(
        description = "Recipients. Both 'user@domain.com' and 'User Name <user@domain.com>' are accepted"
    )]
    pub to: Vec<String>,

    #[property(description = "Subject of the email")]
    pub subject: String,

    #[property(description = "Body of the email")]
    pub body: String,

    #[property(
        description = "When true - the body is sent as text/html, otherwise as text/plain. Default is false"
    )]
    pub is_html: Option<bool>,

    #[property(
        description = "Sender email. When it is not set - the default one from the settings is used"
    )]
    pub from_email: Option<String>,

    #[property(description = "Display name of the sender")]
    pub from_name: Option<String>,

    #[property(description = "Copy recipients")]
    pub cc: Option<Vec<String>>,

    #[property(description = "Blind copy recipients")]
    pub bcc: Option<Vec<String>>,

    #[property(
        description = "'relay' - hand the message over to the relay, 'direct' - deliver it straight to the mail server of the recipient, bypassing the relay. Empty means what the settings say. Use it to test both routes on the same installation"
    )]
    pub delivery_mode: Option<String>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct SendEmailToolResponse {
    #[property(description = "Id of the message inside the queue of the mail server")]
    pub queue_id: Option<String>,

    #[property(description = "Answer of the mail server which accepted the message")]
    pub smtp_response: String,
}

pub struct SendEmailToolHandler {
    app: Arc<AppContext>,
}

impl SendEmailToolHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for SendEmailToolHandler {
    const FUNC_NAME: &'static str = "send_email";
    const DESCRIPTION: &'static str = "Sends an email through the mail server of this container. The answer means the message is accepted and queued - the delivery to the recipient happens afterwards and is visible in the output of the mail server.";
}

#[async_trait::async_trait]
impl McpToolCall<SendEmailInputData, SendEmailToolResponse> for SendEmailToolHandler {
    async fn execute_tool_call(
        &self,
        model: SendEmailInputData,
    ) -> Result<SendEmailToolResponse, String> {
        let model: SendEmailModel = model.try_into()?;

        let result = crate::flows::send_email(&self.app, model).await?;

        Ok(SendEmailToolResponse {
            queue_id: result.queue_id,
            smtp_response: result.smtp_response,
        })
    }
}

impl TryFrom<SendEmailInputData> for SendEmailModel {
    type Error = SendEmailError;

    fn try_from(src: SendEmailInputData) -> Result<Self, Self::Error> {
        let delivery_mode = DeliveryMode::parse(src.delivery_mode.as_deref())
            .map_err(SendEmailError::InvalidEmailModel)?;

        Ok(Self {
            from_email: src.from_email,
            from_name: src.from_name,
            to: src.to,
            cc: src.cc.unwrap_or_default(),
            bcc: src.bcc.unwrap_or_default(),
            subject: src.subject,
            body: src.body,
            is_html: src.is_html.unwrap_or(false),
            attachments: vec![],
            delivery_mode,
        })
    }
}
