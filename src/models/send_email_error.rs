#[derive(Debug)]
pub enum SendEmailError {
    /// The request itself is wrong - a broken email address, a broken attachment, etc.
    InvalidEmailModel(String),
    /// The local mail server did not accept the message.
    MailServerError(String),
}

/// The mcp tool calls answer with a plain string as an error.
impl From<SendEmailError> for String {
    fn from(src: SendEmailError) -> Self {
        match src {
            SendEmailError::InvalidEmailModel(message) => message,
            SendEmailError::MailServerError(message) => message,
        }
    }
}
