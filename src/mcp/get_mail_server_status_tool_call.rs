use std::sync::Arc;

use mcp_server_middleware::*;

use serde::*;

use crate::app::AppContext;

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetMailServerStatusInputData {}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetMailServerStatusResponse {
    #[property(description = "Whether the KumoMTA process of this container is alive")]
    pub running: bool,

    #[property(description = "Whether at least one dkim key is configured")]
    pub dkim_enabled: bool,

    #[property(
        description = "Domains the mail is dkim signed for, and the selector of each of them"
    )]
    pub dkim_domains: Vec<String>,

    #[property(
        description = "Host name the mail server introduces itself with. It has to match the PTR record of the outgoing ip address"
    )]
    pub my_hostname: String,

    #[property(
        description = "Smtp server the mail is handed over to, when the direct delivery is not used"
    )]
    pub relay_host: Option<String>,

    #[property(description = "Summary of the queues of the mail server")]
    pub queue: String,
}

pub struct GetMailServerStatusHandler {
    app: Arc<AppContext>,
}

impl GetMailServerStatusHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for GetMailServerStatusHandler {
    const FUNC_NAME: &'static str = "get_mail_server_status";
    const DESCRIPTION: &'static str = "Tells whether the mail server (KumoMTA) of this container is running, how it is configured to sign and to deliver the mail, and what is in its queues right now.";
}

#[async_trait::async_trait]
impl McpToolCall<GetMailServerStatusInputData, GetMailServerStatusResponse>
    for GetMailServerStatusHandler
{
    async fn execute_tool_call(
        &self,
        _model: GetMailServerStatusInputData,
    ) -> Result<GetMailServerStatusResponse, String> {
        let settings = self.app.settings_reader.get_settings().await;

        let status = self.app.kumo_mta.get_status(&settings).await;

        let dkim_domains = settings
            .dkim
            .iter()
            .map(|dkim| format!("{} (selector: {})", dkim.domain, dkim.selector))
            .collect();

        Ok(GetMailServerStatusResponse {
            running: status.running,
            dkim_enabled: status.dkim_enabled,
            dkim_domains,
            my_hostname: settings.smtp.my_hostname.clone(),
            relay_host: settings
                .smtp
                .relay
                .as_ref()
                .map(|relay| format!("{}:{}", relay.host, relay.get_port())),
            queue: status.queue,
        })
    }
}
