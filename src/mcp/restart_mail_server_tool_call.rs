use std::sync::Arc;

use mcp_server_middleware::*;

use serde::*;

use crate::app::AppContext;

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct RestartMailServerInputData {}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct RestartMailServerResponse {
    #[property(description = "Whether the mail server is up and accepting the messages again")]
    pub running: bool,

    #[property(description = "Last lines of the output of the mail server after the restart")]
    pub output: Vec<String>,
}

pub struct RestartMailServerHandler {
    app: Arc<AppContext>,
}

impl RestartMailServerHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for RestartMailServerHandler {
    const FUNC_NAME: &'static str = "restart_mail_server";
    const DESCRIPTION: &'static str = "Compiles the configuration out of the current settings again and restarts the mail server with it. Use it after the settings have been changed - it does not need the container to be redeployed. The mail which is already in the queue stays in the queue.";
}

#[async_trait::async_trait]
impl McpToolCall<RestartMailServerInputData, RestartMailServerResponse>
    for RestartMailServerHandler
{
    async fn execute_tool_call(
        &self,
        _model: RestartMailServerInputData,
    ) -> Result<RestartMailServerResponse, String> {
        let settings = self.app.settings_reader.get_settings().await;

        let restart_result = self.app.kumo_mta.restart(&settings).await;

        let output = self.app.kumo_mta.get_output(100).await;

        if let Err(err) = restart_result {
            return Err(format!("{}. Output: {}", err, output.join("\n")));
        }

        Ok(RestartMailServerResponse {
            running: self.app.kumo_mta.is_running().await,
            output,
        })
    }
}
