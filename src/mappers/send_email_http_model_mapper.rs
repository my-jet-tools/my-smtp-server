use crate::{
    http_server::controllers::email::{EmailAttachmentHttpModel, SendEmailHttpModel},
    models::{DeliveryMode, EmailAttachmentModel, SendEmailError, SendEmailModel},
};

impl TryFrom<SendEmailHttpModel> for SendEmailModel {
    type Error = SendEmailError;

    fn try_from(src: SendEmailHttpModel) -> Result<Self, Self::Error> {
        let delivery_mode = DeliveryMode::parse(src.delivery_mode.as_deref())
            .map_err(SendEmailError::InvalidEmailModel)?;

        Ok(Self {
            from_email: src.from_email,
            from_name: src.from_name,
            to: src.to,
            cc: src.cc.unwrap_or_default(),
            bcc: src.bcc.unwrap_or_default(),
            subject: src.subject,
            body: src.body,
            is_html: src.is_html.unwrap_or(false),
            attachments: match src.attachments {
                Some(attachments) => attachments.into_iter().map(|itm| itm.into()).collect(),
                None => vec![],
            },
            delivery_mode,
        })
    }
}

impl From<EmailAttachmentHttpModel> for EmailAttachmentModel {
    fn from(src: EmailAttachmentHttpModel) -> Self {
        Self {
            file_name: src.file_name,
            content_type: src.content_type,
            base64_content: src.base64_content,
        }
    }
}
