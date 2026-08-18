#[derive(Debug, Clone)]
pub struct SendEmailModel {
    pub from_email: Option<String>,
    pub from_name: Option<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body: String,
    pub is_html: bool,
    pub attachments: Vec<EmailAttachmentModel>,
}

#[derive(Debug, Clone)]
pub struct EmailAttachmentModel {
    pub file_name: String,
    pub content_type: String,
    pub base64_content: String,
}
