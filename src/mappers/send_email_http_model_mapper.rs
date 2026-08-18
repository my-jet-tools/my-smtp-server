use crate::{
    http_server::controllers::email::{EmailAttachmentHttpModel, SendEmailHttpModel},
    models::{EmailAttachmentModel, SendEmailModel},
};

impl From<SendEmailHttpModel> for SendEmailModel {
    fn from(src: SendEmailHttpModel) -> Self {
        Self {
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
        }
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
