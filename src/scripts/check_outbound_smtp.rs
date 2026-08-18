use std::time::Duration;

use tokio::{io::AsyncReadExt, net::TcpStream, time::timeout};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const BANNER_TIMEOUT: Duration = Duration::from_secs(10);

pub struct OutboundSmtpCheckResult {
    pub connected: bool,
    pub banner: Option<String>,
    pub error: Option<String>,
}

/// Opens a tcp connection to a mail server and reads its greeting - nothing is sent.
/// This is the way to find out whether the provider of the host blocks the outgoing
/// port 25: a blocked port shows up as a connection which never completes.
pub async fn check_outbound_smtp(host: &str, port: u16) -> OutboundSmtpCheckResult {
    let connection = timeout(CONNECT_TIMEOUT, TcpStream::connect((host, port))).await;

    let mut connection = match connection {
        Ok(Ok(connection)) => connection,
        Ok(Err(err)) => {
            return OutboundSmtpCheckResult {
                connected: false,
                banner: None,
                error: Some(format!(
                    "Can not connect to {}:{}. Err: {}",
                    host, port, err
                )),
            };
        }
        Err(_) => {
            return OutboundSmtpCheckResult {
                connected: false,
                banner: None,
                error: Some(format!(
                    "Connection to {}:{} timed out after {} seconds. A connection which is silently timing out usually means the provider blocks the outgoing port {}",
                    host,
                    port,
                    CONNECT_TIMEOUT.as_secs(),
                    port
                )),
            };
        }
    };

    let mut buffer = [0u8; 1024];

    match timeout(BANNER_TIMEOUT, connection.read(&mut buffer)).await {
        Ok(Ok(read_size)) => OutboundSmtpCheckResult {
            connected: true,
            banner: Some(
                String::from_utf8_lossy(&buffer[..read_size])
                    .trim()
                    .to_string(),
            ),
            error: None,
        },
        Ok(Err(err)) => OutboundSmtpCheckResult {
            connected: true,
            banner: None,
            error: Some(format!("Can not read the greeting. Err: {}", err)),
        },
        Err(_) => OutboundSmtpCheckResult {
            connected: true,
            banner: None,
            error: Some(format!(
                "The connection is established but the mail server did not send the greeting within {} seconds",
                BANNER_TIMEOUT.as_secs()
            )),
        },
    }
}
