use std::sync::Arc;

use mcp_server_middleware::McpMiddleware;

use crate::app::{APP_NAME, APP_VERSION, AppContext};

use super::*;

pub const MCP_ROUTE: &str = "/mcp";

/// The service exposes the same operations over MCP as it does over its rest api, plus the
/// ones which are only interesting while something is wrong: the output of the mail server,
/// the configuration it was started with and the check of the outgoing smtp port.
pub fn build_mcp_middleware(app: &Arc<AppContext>) -> McpMiddleware {
    let mut mcp = McpMiddleware::new(
        MCP_ROUTE,
        APP_NAME,
        APP_VERSION,
        "Sends emails and manages the mail server which delivers them",
    );

    mcp.register_tool_call(Arc::new(SendEmailToolHandler::new(app.clone())));
    mcp.register_tool_call(Arc::new(GetMailServerStatusHandler::new(app.clone())));
    mcp.register_tool_call(Arc::new(GetMailServerOutputHandler::new(app.clone())));
    mcp.register_tool_call(Arc::new(GetMailServerPolicyHandler::new(app.clone())));
    mcp.register_tool_call(Arc::new(RestartMailServerHandler::new(app.clone())));
    mcp.register_tool_call(Arc::new(CheckOutboundSmtpHandler::new(app.clone())));

    mcp
}
