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
    /// Over the http api of mailgun - port 443 instead of an smtp port. The mail server of
    /// this container is not involved at all, so there is no local queue behind it.
    MailgunHttp,
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
            "mailgun_http" => Ok(DeliveryMode::MailgunHttp),
            "http" => Ok(DeliveryMode::MailgunHttp),
            _ => Err(format!(
                "Unknown delivery mode '{}'. It has to be 'relay', 'direct', 'mailgun_http' or empty",
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

impl SendEmailModel {
    /// Everybody the message goes to - the http api of mailgun needs them as the envelope,
    /// it does not read them out of the message.
    pub fn get_all_recipients(&self) -> Vec<String> {
        let mut result = Vec::with_capacity(self.to.len() + self.cc.len() + self.bcc.len());

        result.extend(self.to.iter().cloned());
        result.extend(self.cc.iter().cloned());
        result.extend(self.bcc.iter().cloned());

        result
    }
}

#[derive(Debug, Clone)]
pub struct EmailAttachmentModel {
    pub file_name: String,
    pub content_type: String,
    pub base64_content: String,
}
