use std::sync::Arc;

use mcp_server_middleware::*;

use serde::*;

use crate::app::AppContext;

const DEFAULT_AMOUNT_OF_LINES: usize = 200;

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetMailServerOutputInputData {
    #[property(description = "How many last lines to return. Default is 200")]
    pub amount_of_lines: Option<u32>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetMailServerOutputResponse {
    #[property(description = "Last lines the mail server has written to its stdout and stderr")]
    pub lines: Vec<String>,
}

pub struct GetMailServerOutputHandler {
    app: Arc<AppContext>,
}

impl GetMailServerOutputHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for GetMailServerOutputHandler {
    const FUNC_NAME: &'static str = "get_mail_server_output";
    const DESCRIPTION: &'static str = "Returns the last lines the mail server (KumoMTA) has written to its stdout and stderr. This is where the reason of a failed start up, a rejected message or a deferred delivery is visible.";
}

#[async_trait::async_trait]
impl McpToolCall<GetMailServerOutputInputData, GetMailServerOutputResponse>
    for GetMailServerOutputHandler
{
    async fn execute_tool_call(
        &self,
        model: GetMailServerOutputInputData,
    ) -> Result<GetMailServerOutputResponse, String> {
        let amount_of_lines = match model.amount_of_lines {
            Some(amount_of_lines) => amount_of_lines as usize,
            None => DEFAULT_AMOUNT_OF_LINES,
        };

        let lines = self.app.kumo_mta.get_output(amount_of_lines).await;

        Ok(GetMailServerOutputResponse { lines })
    }
}
