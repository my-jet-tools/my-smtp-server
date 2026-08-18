use std::sync::Arc;

use mcp_server_middleware::*;

use serde::*;

use crate::{
    app::AppContext,
    kumo_mta::{POLICY_FILE, redact_policy_secrets},
};

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetMailServerPolicyInputData {}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetMailServerPolicyResponse {
    #[property(description = "Path of the policy file")]
    pub file_name: String,

    #[property(description = "Content of the policy the mail server was started with")]
    pub content: String,
}

pub struct GetMailServerPolicyHandler {
    _app: Arc<AppContext>,
}

impl GetMailServerPolicyHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { _app: app }
    }
}

impl ToolDefinition for GetMailServerPolicyHandler {
    const FUNC_NAME: &'static str = "get_mail_server_policy";
    const DESCRIPTION: &'static str = "Returns the configuration (lua policy) which was compiled out of the settings model and which the mail server is running with. Use it to see what the settings actually turned into.";
}

#[async_trait::async_trait]
impl McpToolCall<GetMailServerPolicyInputData, GetMailServerPolicyResponse>
    for GetMailServerPolicyHandler
{
    async fn execute_tool_call(
        &self,
        _model: GetMailServerPolicyInputData,
    ) -> Result<GetMailServerPolicyResponse, String> {
        match tokio::fs::read_to_string(POLICY_FILE).await {
            Ok(content) => Ok(GetMailServerPolicyResponse {
                file_name: POLICY_FILE.to_string(),
                content,
            }),
            Err(err) => Err(format!(
                "Can not read the policy file '{}'. Err: {}",
                POLICY_FILE, err
            )),
        }
    }
}
