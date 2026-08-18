use std::sync::Arc;

use my_grpc_extensions::GrpcError;
use my_grpc_extensions::server::*;

use crate::{
    app::{APP_NAME, APP_VERSION, AppContext},
    models::SendEmailModel,
};

generate_server!(
    proto_file: "./proto/MySmtpSender.proto",
    crate_ns: "crate::my_smtp_sender_grpc",
    with_error: true
);

async fn send_email(
    app: &Arc<AppContext>,
    request: SendEmailGrpcRequest,
) -> Result<SendEmailGrpcResponse, GrpcError> {
    let model: SendEmailModel = request.try_into()?;

    let result = crate::flows::send_email(app, model).await?;

    Ok(SendEmailGrpcResponse {
        queue_id: result.queue_id,
        response: result.smtp_response,
    })
}

async fn is_alive(app: &Arc<AppContext>, _request: ()) -> Result<IsAliveGrpcResponse, GrpcError> {
    Ok(IsAliveGrpcResponse {
        name: APP_NAME.to_string(),
        version: APP_VERSION.to_string(),
        mail_server_running: app.kumo_mta.is_running().await,
    })
}
