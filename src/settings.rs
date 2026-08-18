use serde::{Deserialize, Serialize};

#[derive(my_settings_reader::SettingsModel, Serialize, Deserialize, Debug, Clone)]
pub struct SettingsModel {
    pub smtp: SmtpSettingsModel,
    #[serde(default)]
    pub dkim: Vec<DkimSettingsModel>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

    /// Optional. When it is set - the mail is not delivered to the mail server of the
    /// recipient directly, but handed over to this smtp server. Needed when the outgoing
    /// ip address has no proper PTR record or the provider blocks the outgoing port 25 -
    /// a home connection, for example.
    #[serde(default)]
    pub relay: Option<RelaySettingsModel>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
pub struct DkimSettingsModel {
    pub domain: String,
    pub selector: String,
    /// Private key in PEM format. The matching public key has to be published as
    /// a TXT record of {selector}._domainkey.{domain}
    pub private_key: String,
}

impl SettingsModel {
    pub fn get_dkim_enabled(&self) -> bool {
        !self.dkim.is_empty()
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
