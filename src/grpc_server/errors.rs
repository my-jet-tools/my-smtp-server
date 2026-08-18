use my_grpc_extensions::GrpcError;

use crate::models::SendEmailError;

impl From<SendEmailError> for GrpcError {
    fn from(src: SendEmailError) -> Self {
        match src {
            SendEmailError::InvalidEmailModel(message) => GrpcError::invalid_argument(message),
            SendEmailError::MailServerError(message) => GrpcError::internal(message),
        }
    }
}
