use std::sync::Arc;

use mcp_server_middleware::*;

use serde::*;

use crate::app::AppContext;

/// One of the mail servers of gmail - it answers to everybody and is the simplest way to
/// find out whether the outgoing port 25 works at all from this host.
const DEFAULT_HOST: &str = "aspmx.l.google.com";
const DEFAULT_PORT: u16 = 25;

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct CheckOutboundSmtpInputData {
    #[property(description = "Mail server to connect to. Default is aspmx.l.google.com")]
    pub host: Option<String>,

    #[property(description = "Port to connect to. Default is 25")]
    pub port: Option<u32>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct CheckOutboundSmtpResponse {
    #[property(description = "Whether the tcp connection was established")]
    pub connected: bool,

    #[property(
        description = "Greeting of the mail server. Its presence proves the port is not blocked on the way out"
    )]
    pub banner: Option<String>,

    #[property(description = "Why the check did not succeed")]
    pub error: Option<String>,
}

pub struct CheckOutboundSmtpHandler {
    _app: Arc<AppContext>,
}

impl CheckOutboundSmtpHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { _app: app }
    }
}

impl ToolDefinition for CheckOutboundSmtpHandler {
    const FUNC_NAME: &'static str = "check_outbound_smtp";
    const DESCRIPTION: &'static str = "Opens a tcp connection from this container to a mail server and reads its greeting - no mail is sent. This is the way to find out whether the provider of the host blocks the outgoing smtp port, which is the usual reason for the mail to be stuck in the queue.";
}

#[async_trait::async_trait]
impl McpToolCall<CheckOutboundSmtpInputData, CheckOutboundSmtpResponse>
    for CheckOutboundSmtpHandler
{
    async fn execute_tool_call(
        &self,
        model: CheckOutboundSmtpInputData,
    ) -> Result<CheckOutboundSmtpResponse, String> {
        let host = match &model.host {
            Some(host) => host.trim(),
            None => DEFAULT_HOST,
        };

        let port = match model.port {
            Some(port) => port as u16,
            None => DEFAULT_PORT,
        };

        let result = crate::scripts::check_outbound_smtp(host, port).await;

        Ok(CheckOutboundSmtpResponse {
            connected: result.connected,
            banner: result.banner,
            error: result.error,
        })
    }
}
