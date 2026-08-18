use std::{net::SocketAddr, sync::Arc};

use tonic::transport::Server;

use crate::{app::AppContext, my_smtp_sender_grpc::my_smtp_sender_server::MySmtpSenderServer};

use super::SdkGrpcService;

pub const GRPC_PORT: u16 = 8001;

/// The grpc endpoint carries the same operations as the rest api - it exists for the
/// services which talk grpc to everything else and do not want an http client just for
/// sending mail.
pub fn start_grpc_server(app: &Arc<AppContext>) {
    let listen_address = SocketAddr::from(([0, 0, 0, 0], GRPC_PORT));

    println!("Starting GRPC server at {}", listen_address);

    let service = SdkGrpcService::new(app.clone());

    tokio::spawn(async move {
        let result = Server::builder()
            .add_service(MySmtpSenderServer::new(service))
            .serve(listen_address)
            .await;

        if let Err(err) = result {
            my_logger::LOGGER.write_error(
                "start_grpc_server",
                format!("The grpc server is stopped. Err: {}", err),
                my_logger::LogEventCtx::new(),
            );
        }
    });
}
