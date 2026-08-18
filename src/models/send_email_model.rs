/// Which way one particular message has to leave the container. It exists to be able to
/// test both paths on the same installation - the settings decide what happens by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryMode {
    /// Whatever the settings say: through the relay when it is configured, straight to the
    /// recipient when it is not.
    AsConfigured,
    /// Through the relay. Fails when no relay is configured.
    Relay,
    /// Straight to the mail server of the recipient, even when a relay is configured.
    Direct,
}

impl DeliveryMode {
    pub fn parse(src: Option<&str>) -> Result<Self, String> {
        let Some(src) = src else {
            return Ok(DeliveryMode::AsConfigured);
        };

        match src.trim().to_lowercase().as_str() {
            "" => Ok(DeliveryMode::AsConfigured),
            "as_configured" => Ok(DeliveryMode::AsConfigured),
            "relay" => Ok(DeliveryMode::Relay),
            "direct" => Ok(DeliveryMode::Direct),
            _ => Err(format!(
                "Unknown delivery mode '{}'. It has to be 'relay', 'direct' or empty",
                src
            )),
        }
    }
}

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
    pub delivery_mode: DeliveryMode,
}

#[derive(Debug, Clone)]
pub struct EmailAttachmentModel {
    pub file_name: String,
    pub content_type: String,
    pub base64_content: String,
}
