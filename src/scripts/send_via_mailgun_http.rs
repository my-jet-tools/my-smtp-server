use base64::{Engine, engine::general_purpose::STANDARD};
use flurl::{FlUrl, body::new_form_data};
use lettre::Message;

use crate::settings::MailgunHttpSettingsModel;

/// The endpoint which takes a complete MIME message. The other one - /messages - takes the
/// fields and builds the message itself, which would mean giving up the control over what
/// exactly is sent.
const MESSAGES_MIME_PATH: &str = "messages.mime";

pub struct MailgunHttpSubmitResult {
    /// The id mailgun gave the message.
    pub message_id: Option<String>,
    pub response: String,
}

/// Hands the message over to mailgun over their http api instead of smtp. The message is
/// the very same one the smtp route sends - built and, when kumod is not in the way, signed
/// by mailgun itself, since their signing key is delegated to our domain by the dkim CNAME.
pub async fn send_via_mailgun_http(
    settings: &MailgunHttpSettingsModel,
    message: &Message,
    recipients: &[String],
) -> Result<MailgunHttpSubmitResult, String> {
    if recipients.is_empty() {
        return Err("There is nobody to send the message to".to_string());
    }

    let mut form_data = new_form_data();

    // The envelope recipients: the api does not read them out of the message itself.
    for recipient in recipients {
        form_data = form_data.append_form_data_field("to", recipient.as_str());
    }

    let mime = message.formatted();

    form_data = form_data.append_form_data_file("message", "message.mime", "message/rfc822", &mime);

    let url = settings.get_base_url();

    let response = FlUrl::new(url.as_str())
        .append_path_segment("v3")
        .append_path_segment(settings.domain.trim())
        .append_path_segment(MESSAGES_MIME_PATH)
        .with_header("Authorization", compile_authorization(settings))
        .post(form_data)
        .await;

    let mut response = match response {
        Ok(response) => response,
        Err(err) => {
            return Err(format!(
                "Can not reach the mailgun http api at '{}'. Err: {:?}",
                url, err
            ));
        }
    };

    let status_code = response.get_status_code();

    let body = match response.get_body_as_str().await {
        Ok(body) => body.to_string(),
        Err(err) => format!("<can not read the answer: {:?}>", err),
    };

    if !(200..=299).contains(&status_code) {
        return Err(format!(
            "The mailgun http api answered with the status code {}. {}",
            status_code, body
        ));
    }

    Ok(MailgunHttpSubmitResult {
        message_id: extract_message_id(body.as_str()),
        response: body,
    })
}

/// Mailgun authenticates the api with the http basic scheme, where the user is the literal
/// 'api' and the password is the api key.
fn compile_authorization(settings: &MailgunHttpSettingsModel) -> String {
    let credentials = format!("api:{}", settings.api_key.trim());

    format!("Basic {}", STANDARD.encode(credentials.as_bytes()))
}

/// The answer is `{"id": "<20260818....@x-fine.online>", "message": "Queued. Thank you."}`.
fn extract_message_id(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;

    let id = value.get("id")?.as_str()?.trim();

    if id.is_empty() {
        return None;
    }

    Some(id.to_string())
}

#[cfg(test)]
mod tests {
    use crate::settings::MailgunHttpSettingsModel;

    use super::{compile_authorization, extract_message_id};

    fn create_settings(region: Option<&str>) -> MailgunHttpSettingsModel {
        MailgunHttpSettingsModel {
            api_key: "key-secret".to_string(),
            domain: "mydomain.com".to_string(),
            region: region.map(|itm| itm.to_string()),
            base_url: None,
        }
    }

    #[test]
    fn test_base_url_of_the_region() {
        assert_eq!(
            create_settings(Some("eu")).get_base_url().as_str(),
            "https://api.eu.mailgun.net"
        );

        assert_eq!(
            create_settings(None).get_base_url().as_str(),
            "https://api.mailgun.net"
        );
    }

    #[test]
    fn test_authorization_is_the_api_user_and_the_key() {
        // base64 of "api:key-secret"
        assert_eq!(
            compile_authorization(&create_settings(None)).as_str(),
            "Basic YXBpOmtleS1zZWNyZXQ="
        );
    }

    #[test]
    fn test_message_id_is_extracted() {
        let body = r#"{"id":"<20260818.1@mydomain.com>","message":"Queued. Thank you."}"#;

        assert_eq!(
            extract_message_id(body).unwrap().as_str(),
            "<20260818.1@mydomain.com>"
        );

        assert!(extract_message_id("not a json").is_none());
    }
}
