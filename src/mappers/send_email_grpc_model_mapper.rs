use crate::{
    models::{DeliveryMode, EmailAttachmentModel, SendEmailError, SendEmailModel},
    my_smtp_sender_grpc::{EmailAttachmentGrpcModel, SendEmailGrpcRequest},
};

impl TryFrom<SendEmailGrpcRequest> for SendEmailModel {
    type Error = SendEmailError;

    fn try_from(src: SendEmailGrpcRequest) -> Result<Self, Self::Error> {
        let delivery_mode = DeliveryMode::parse(src.delivery_mode.as_deref())
            .map_err(SendEmailError::InvalidEmailModel)?;

        Ok(Self {
            from_email: src.from_email,
            from_name: src.from_name,
            to: src.to,
            cc: src.cc,
            bcc: src.bcc,
            subject: src.subject,
            body: src.body,
            is_html: src.is_html,
            attachments: src.attachments.into_iter().map(|itm| itm.into()).collect(),
            delivery_mode,
        })
    }
}

impl From<EmailAttachmentGrpcModel> for EmailAttachmentModel {
    fn from(src: EmailAttachmentGrpcModel) -> Self {
        Self {
            file_name: src.file_name,
            content_type: src.content_type,
            base64_content: src.base64_content,
        }
    }
}
