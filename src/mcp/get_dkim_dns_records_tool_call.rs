use std::sync::Arc;

use mcp_server_middleware::*;

use serde::*;

use crate::{app::AppContext, kumo_mta::get_dkim_private_key_file};

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetDkimDnsRecordsInputData {}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct DkimDnsRecordModel {
    #[property(description = "Domain the key signs the mail of")]
    pub domain: String,

    #[property(description = "Name of the TXT record to publish")]
    pub record_name: String,

    #[property(description = "Value of the TXT record to publish")]
    pub record_value: String,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetDkimDnsRecordsResponse {
    #[property(description = "One record per configured dkim key")]
    pub records: Vec<DkimDnsRecordModel>,

    #[property(description = "Keys whose dns record could not be compiled, and why")]
    pub errors: Vec<String>,
}

pub struct GetDkimDnsRecordsHandler {
    app: Arc<AppContext>,
}

impl GetDkimDnsRecordsHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for GetDkimDnsRecordsHandler {
    const FUNC_NAME: &'static str = "get_dkim_dns_records";
    const DESCRIPTION: &'static str = "Returns the TXT records which have to be published in dns for the dkim keys the mail server is actually signing with. Until a record is published, the recipients can not verify the signature.";
}

#[async_trait::async_trait]
impl McpToolCall<GetDkimDnsRecordsInputData, GetDkimDnsRecordsResponse>
    for GetDkimDnsRecordsHandler
{
    async fn execute_tool_call(
        &self,
        _model: GetDkimDnsRecordsInputData,
    ) -> Result<GetDkimDnsRecordsResponse, String> {
        let settings = self.app.settings_reader.get_settings().await;

        let mut records = Vec::new();
        let mut errors = Vec::new();

        for dkim in settings.dkim.iter() {
            // The copy the mail server signs with - not the source file, which could have
            // been changed after the mail server was started.
            let private_key_file = get_dkim_private_key_file(dkim);

            let private_key = match tokio::fs::read_to_string(private_key_file.as_str()).await {
                Ok(private_key) => private_key,
                Err(err) => {
                    errors.push(format!(
                        "Can not read the private key of the domain '{}' from '{}'. Err: {}",
                        dkim.domain, private_key_file, err
                    ));
                    continue;
                }
            };

            match crate::scripts::compile_dkim_dns_record(
                &dkim.domain,
                &dkim.selector,
                private_key.as_str(),
            ) {
                Ok(record) => records.push(DkimDnsRecordModel {
                    domain: dkim.domain.clone(),
                    record_name: record.name,
                    record_value: record.value,
                }),
                Err(err) => errors.push(format!(
                    "Can not compile the dns record of the domain '{}'. Err: {}",
                    dkim.domain, err
                )),
            }
        }

        Ok(GetDkimDnsRecordsResponse { records, errors })
    }
}
