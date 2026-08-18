use serde::{Deserialize, Serialize};

use crate::models::DeliveryMode;

/// `deny_unknown_fields` is deliberate: a key which is indented one level off - `relay`
/// under `smtp`, say - would otherwise be silently ignored, and the mail would quietly take
/// another route. Better to refuse to start and name the key.
#[derive(my_settings_reader::SettingsModel, Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SettingsModel {
    pub smtp: SmtpSettingsModel,
    #[serde(default)]
    pub dkim: Vec<DkimSettingsModel>,
    /// Optional. When it is set - the mail is not delivered to the mail server of the
    /// recipient directly, but handed over to this smtp server. Needed when the outgoing
    /// ip address has no proper PTR record or the provider blocks the outgoing port 25 -
    /// a home connection, for example.
    #[serde(default)]
    pub relay: Option<RelaySettingsModel>,
    /// Optional. The http api of mailgun as another route: it goes over 443, which no
    /// provider blocks, unlike the smtp ports.
    #[serde(default)]
    pub mailgun_http: Option<MailgunHttpSettingsModel>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct MailgunHttpSettingsModel {
    /// The api key of the account - NOT the smtp password, they are different secrets.
    pub api_key: String,

    /// The domain as it is registered in mailgun.
    pub domain: String,

    /// 'eu' or 'us'. Default is 'us' - the same as the default of mailgun itself.
    #[serde(default)]
    pub region: Option<String>,

    /// Overrides the url which is derived from the region.
    #[serde(default)]
    pub base_url: Option<String>,
}

impl MailgunHttpSettingsModel {
    pub fn get_base_url(&self) -> String {
        if let Some(base_url) = &self.base_url {
            let base_url = base_url.trim().trim_end_matches('/');

            if !base_url.is_empty() {
                return base_url.to_string();
            }
        }

        match self.get_region() {
            "eu" => "https://api.eu.mailgun.net".to_string(),
            _ => "https://api.mailgun.net".to_string(),
        }
    }

    pub fn get_region(&self) -> &str {
        match &self.region {
            Some(region) => region.trim(),
            None => "us",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SmtpSettingsModel {
    /// Host name the mail server introduces itself with (HELO/EHLO). Must have a matching
    /// A and PTR record, otherwise the recipient mail servers reject the mail.
    pub my_hostname: String,

    /// Email which is used as a sender when the request has no from_email.
    pub default_from_email: String,

    #[serde(default)]
    pub default_from_name: Option<String>,

    #[serde(default)]
    pub message_size_limit_mb: Option<u64>,

    /// How long the mail server keeps retrying the delivery before it gives up.
    #[serde(default)]
    pub max_queue_lifetime_hours: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RelaySettingsModel {
    pub host: String,

    #[serde(default)]
    pub port: Option<u16>,

    #[serde(default)]
    pub user: Option<String>,

    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct DkimSettingsModel {
    pub domain: String,
    pub selector: String,
    /// Path of the file with the private key in PEM format - the file itself is never
    /// a part of the settings. `~` and the environment variables are resolved.
    /// The matching public key has to be published as a TXT record of
    /// {selector}._domainkey.{domain}
    pub private_key_path: String,
}

impl DkimSettingsModel {
    pub fn get_private_key_path(&self) -> String {
        rust_extensions::file_utils::format_path(self.private_key_path.trim()).to_string()
    }
}

impl SettingsModel {
    pub fn get_dkim_enabled(&self) -> bool {
        !self.dkim.is_empty()
    }

    /// The route a message takes when the request does not ask for a particular one. It is
    /// decided by which section is present, so there is no third place where the same
    /// choice could be spelled out differently:
    ///
    /// * `mailgun_http` is set - the http api of mailgun,
    /// * otherwise `smtp.relay` is set - the relay over smtp,
    /// * otherwise straight to the mail server of the recipient.
    ///
    /// When both are set, the http api wins - configuring it is the deliberate act, and the
    /// relay stays available for a request which asks for it explicitly.
    pub fn get_default_delivery_mode(&self) -> DeliveryMode {
        if self.mailgun_http.is_some() {
            return DeliveryMode::MailgunHttp;
        }

        if self.relay.is_some() {
            return DeliveryMode::Relay;
        }

        DeliveryMode::Direct
    }
}

impl SmtpSettingsModel {
    pub fn get_message_size_limit_bytes(&self) -> u64 {
        self.message_size_limit_mb.unwrap_or(25) * 1024 * 1024
    }

    pub fn get_max_queue_lifetime_hours(&self) -> u64 {
        self.max_queue_lifetime_hours.unwrap_or(24)
    }
}

impl RelaySettingsModel {
    pub fn get_port(&self) -> u16 {
        self.port.unwrap_or(587)
    }

    pub fn has_authentication(&self) -> bool {
        self.get_user().is_some() && self.get_password().is_some()
    }

    pub fn get_user(&self) -> Option<&str> {
        get_not_empty(&self.user)
    }

    pub fn get_password(&self) -> Option<&str> {
        get_not_empty(&self.password)
    }
}

fn get_not_empty(src: &Option<String>) -> Option<&str> {
    let value = src.as_ref()?.trim();

    if value.is_empty() {
        return None;
    }

    Some(value)
}
