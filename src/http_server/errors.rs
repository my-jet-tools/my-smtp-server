use my_http_server::HttpFailResult;

use crate::models::SendEmailError;

impl From<SendEmailError> for HttpFailResult {
    fn from(src: SendEmailError) -> Self {
        match src {
            SendEmailError::InvalidEmailModel(message) => {
                HttpFailResult::as_validation_error(message)
            }
            SendEmailError::MailServerError(message) => HttpFailResult::as_fatal_error(message),
        }
    }
}
