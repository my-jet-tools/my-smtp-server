use std::sync::Arc;

use mcp_server_middleware::*;

use serde::*;

use crate::app::AppContext;

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetCheckupInputData {}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct CheckItemModel {
    #[property(description = "What is checked")]
    pub title: String,

    #[property(description = "ok, warning or failed")]
    pub status: String,

    #[property(description = "What it means")]
    pub message: String,

    #[property(description = "The value which has to be published or configured")]
    pub expected: Option<String>,

    #[property(description = "The value which is there right now")]
    pub actual: Option<String>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetCheckupResponse {
    #[property(description = "The worst status of the items: ok, warning or failed")]
    pub status: String,

    #[property(description = "The ip address the recipients see the mail coming from")]
    pub public_ip: Option<String>,

    #[property(description = "One entry per check")]
    pub items: Vec<CheckItemModel>,
}

pub struct GetCheckupHandler {
    app: Arc<AppContext>,
}

impl GetCheckupHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for GetCheckupHandler {
    const FUNC_NAME: &'static str = "get_checkup";
    const DESCRIPTION: &'static str = "Checks everything the delivery depends on: the mail server process, the outgoing smtp port, and the dns records - PTR, SPF, DKIM, DMARC - compared with what this service is actually configured with. This is the first tool to call when the mail is not arriving.";
}

#[async_trait::async_trait]
impl McpToolCall<GetCheckupInputData, GetCheckupResponse> for GetCheckupHandler {
    async fn execute_tool_call(
        &self,
        _model: GetCheckupInputData,
    ) -> Result<GetCheckupResponse, String> {
        let report = crate::flows::get_checkup(&self.app).await;

        Ok(GetCheckupResponse {
            status: report.get_status().as_str().to_string(),
            public_ip: report.public_ip.clone(),
            items: report
                .items
                .iter()
                .map(|item| CheckItemModel {
                    title: item.title.clone(),
                    status: item.status.as_str().to_string(),
                    message: item.message.clone(),
                    expected: item.expected.clone(),
                    actual: item.actual.clone(),
                })
                .collect(),
        })
    }
}
