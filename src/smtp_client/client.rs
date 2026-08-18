use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::kumo_mta::{LOCAL_SMTP_HOST, LOCAL_SMTP_PORT};

use super::SmtpSubmitResult;

/// Submits the messages to the kumod instance which is running inside the same container.
/// The connection is plain text on purpose - it never leaves the loopback interface, and
/// kumod is the one who negotiates tls with the recipient mail server.
pub struct SmtpClient {
    transport: AsyncSmtpTransport<Tokio1Executor>,
}

impl SmtpClient {
    pub fn new() -> Self {
        let transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(LOCAL_SMTP_HOST)
            .port(LOCAL_SMTP_PORT)
            .build();

        Self { transport }
    }

    pub async fn send(&self, message: Message) -> Result<SmtpSubmitResult, String> {
        match self.transport.send(message).await {
            Ok(response) => {
                let smtp_response = response
                    .message()
                    .map(|line| line.to_string())
                    .collect::<Vec<String>>()
                    .join(" ");

                Ok(SmtpSubmitResult::new(smtp_response))
            }
            Err(err) => Err(format!("{}", err)),
        }
    }
}
