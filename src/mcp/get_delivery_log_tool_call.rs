use std::sync::Arc;

use mcp_server_middleware::*;

use serde::*;

use crate::app::AppContext;

const DEFAULT_AMOUNT_OF_RECORDS: usize = 50;

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetDeliveryLogInputData {
    #[property(description = "How many last records to return. Default is 50")]
    pub amount_of_records: Option<u32>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetDeliveryLogResponse {
    #[property(
        description = "One json record per event: Reception, Delivery, TransientFailure or Bounce. The 'response' of a failure holds what the receiving mail server answered"
    )]
    pub records: Vec<String>,
}

pub struct GetDeliveryLogHandler {
    _app: Arc<AppContext>,
}

impl GetDeliveryLogHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { _app: app }
    }
}

impl ToolDefinition for GetDeliveryLogHandler {
    const FUNC_NAME: &'static str = "get_delivery_log";
    const DESCRIPTION: &'static str = "Returns the delivery log of the mail server: what happened to each message after it was accepted, and what the receiving mail server answered. This is where the reason of a message which does not arrive is written - a rejected authentication, a refused recipient, a deferral.";
}

#[async_trait::async_trait]
impl McpToolCall<GetDeliveryLogInputData, GetDeliveryLogResponse> for GetDeliveryLogHandler {
    async fn execute_tool_call(
        &self,
        model: GetDeliveryLogInputData,
    ) -> Result<GetDeliveryLogResponse, String> {
        let amount_of_records = match model.amount_of_records {
            Some(amount_of_records) => amount_of_records as usize,
            None => DEFAULT_AMOUNT_OF_RECORDS,
        };

        let records = crate::scripts::read_delivery_log(amount_of_records).await?;

        Ok(GetDeliveryLogResponse { records })
    }
}
