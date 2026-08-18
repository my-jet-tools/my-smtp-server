use std::{net::SocketAddr, sync::Arc};

use my_http_server::{MyHttpServer, controllers::swagger::SwaggerMiddleware};

use crate::app::{APP_NAME, APP_VERSION, AppContext};

const HTTP_PORT: u16 = 8000;

pub fn setup_server(app: &Arc<AppContext>) {
    let listen_address = SocketAddr::from(([0, 0, 0, 0], HTTP_PORT));

    println!("Starting HTTP server at {}", listen_address);

    let mut http_server = MyHttpServer::new(listen_address);

    let controllers = Arc::new(super::build_controllers(app));

    let swagger_middleware = SwaggerMiddleware::new(
        controllers.clone(),
        APP_NAME.to_string(),
        APP_VERSION.to_string(),
    );

    http_server.add_middleware(Arc::new(swagger_middleware));
    http_server.add_middleware(Arc::new(crate::mcp::build_mcp_middleware(app)));
    http_server.add_middleware(controllers);

    http_server.start(app.states.clone(), my_logger::LOGGER.clone());
}
