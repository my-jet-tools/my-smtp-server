use std::{net::IpAddr, sync::Arc};

use crate::{
    app::AppContext,
    models::{CheckItem, CheckupReport},
    scripts::DkimDnsRecord,
    settings::SettingsModel,
};

/// One of the mail servers of gmail - the simplest way to find out whether the outgoing
/// port 25 works at all from this host.
const OUTBOUND_SMTP_PROBE_HOST: &str = "aspmx.l.google.com";
const OUTBOUND_SMTP_PROBE_PORT: u16 = 25;

/// Walks through everything which has to be true for the mail to reach an inbox, and
/// reports what is in place and what is not. Nothing here changes anything.
pub async fn get_checkup(app: &Arc<AppContext>) -> CheckupReport {
    let settings = app.settings_reader.get_settings().await;

    let mut items = Vec::new();

    items.push(check_mail_server(app).await);

    let public_ip = crate::scripts::lookup_own_public_ip().await;

    let public_ip = match public_ip {
        Ok(public_ip) => {
            items.push(
                CheckItem::ok(
                    "Outgoing ip address",
                    "This is the address the recipient mail servers see the mail coming from",
                )
                .with_actual(public_ip.to_string()),
            );
            Some(public_ip)
        }
        Err(err) => {
            items.push(CheckItem::warning(
                "Outgoing ip address",
                format!(
                    "Can not find out the outgoing ip address, so the checks which need it are skipped. {}",
                    err
                ),
            ));
            None
        }
    };

    items.push(check_outbound_smtp(&settings).await);

    items.extend(check_hostname(&settings, public_ip).await);

    let dns_records = crate::scripts::collect_dkim_dns_records(&settings).await;

    for record in dns_records.records.iter() {
        let dkim = settings.dkim.iter().find(|dkim| {
            dkim.domain
                .trim()
                .eq_ignore_ascii_case(record.domain.as_str())
        });

        items.push(check_dkim_record(record, dkim).await);
    }

    for error in dns_records.errors.iter() {
        items.push(CheckItem::failed("DKIM key", error.as_str()));
    }

    for dkim in settings.dkim.iter() {
        items.push(check_dkim_key_file(dkim).await);
    }

    items.push(check_default_sender_is_signed(&settings));

    for domain in get_sending_domains(&settings) {
        items.push(check_spf(domain.as_str(), public_ip, &settings).await);
        items.push(check_dmarc(domain.as_str()).await);
    }

    CheckupReport {
        public_ip: public_ip.map(|ip| ip.to_string()),
        items,
    }
}

async fn check_mail_server(app: &Arc<AppContext>) -> CheckItem {
    if app.kumo_mta.is_running().await {
        return CheckItem::ok(
            "Mail server",
            "The KumoMTA process of this container is running",
        );
    }

    CheckItem::failed(
        "Mail server",
        "The KumoMTA process is not running - nothing can be sent at all. Read its output to find out why",
    )
}

async fn check_outbound_smtp(settings: &SettingsModel) -> CheckItem {
    let (host, port, title) = match &settings.relay {
        Some(relay) => (
            relay.host.trim().to_string(),
            relay.get_port(),
            "Connection to the relay",
        ),
        None => (
            OUTBOUND_SMTP_PROBE_HOST.to_string(),
            OUTBOUND_SMTP_PROBE_PORT,
            "Outgoing smtp port",
        ),
    };

    let result = crate::scripts::check_outbound_smtp(host.as_str(), port).await;

    if let Some(banner) = result.banner {
        return CheckItem::ok(
            title,
            format!("{}:{} answers, so the port is not blocked", host, port),
        )
        .with_actual(banner);
    }

    let message = match result.error {
        Some(error) => error,
        None => format!("Can not talk to {}:{}", host, port),
    };

    CheckItem::failed(title, message)
}

async fn check_hostname(settings: &SettingsModel, public_ip: Option<IpAddr>) -> Vec<CheckItem> {
    let my_hostname = settings.smtp.my_hostname.trim().to_string();

    // With a relay the last hop is made by somebody else, and the PTR/A pair of this host
    // is not what the recipients look at.
    let matters = settings.relay.is_none();

    let mut result = Vec::new();

    let addresses = crate::scripts::lookup_a(my_hostname.as_str()).await;

    let hostname_item = match &addresses {
        Ok(addresses) => {
            let actual = addresses
                .iter()
                .map(|ip| ip.to_string())
                .collect::<Vec<String>>()
                .join(", ");

            match public_ip {
                Some(public_ip) if addresses.contains(&public_ip) => CheckItem::ok(
                    format!("A record of {}", my_hostname),
                    "The host name resolves to the outgoing ip address",
                )
                .with_actual(actual),
                Some(public_ip) => downgrade(
                    CheckItem::failed(
                        format!("A record of {}", my_hostname),
                        "The host name does not resolve to the outgoing ip address",
                    )
                    .with_expected(public_ip.to_string())
                    .with_actual(actual),
                    matters,
                ),
                None => CheckItem::warning(
                    format!("A record of {}", my_hostname),
                    "The outgoing ip address is unknown, so there is nothing to compare with",
                )
                .with_actual(actual),
            }
        }
        Err(err) => downgrade(
            CheckItem::failed(format!("A record of {}", my_hostname), err.as_str()),
            matters,
        ),
    };

    result.push(hostname_item);

    let Some(public_ip) = public_ip else {
        return result;
    };

    let ptr_item = match crate::scripts::lookup_ptr(public_ip).await {
        Ok(names) => {
            let actual = names.join(", ");

            if names
                .iter()
                .any(|name| name.eq_ignore_ascii_case(&my_hostname))
            {
                CheckItem::ok(
                    "PTR record",
                    "The reverse dns of the outgoing ip address matches the host name of the mail server",
                )
                .with_actual(actual)
            } else {
                downgrade(
                    CheckItem::failed(
                        "PTR record",
                        "The reverse dns of the outgoing ip address does not match the host name. Gmail rejects the mail from an address whose PTR does not match, with 550 5.7.25",
                    )
                    .with_expected(my_hostname.clone())
                    .with_actual(actual),
                    matters,
                )
            }
        }
        Err(err) => downgrade(
            CheckItem::failed(
                "PTR record",
                format!(
                    "{}. An address without a PTR record is rejected by gmail with 550 5.7.25",
                    err
                ),
            )
            .with_expected(my_hostname.clone()),
            matters,
        ),
    };

    result.push(ptr_item);

    result
}

async fn check_dkim_record(
    record: &DkimDnsRecord,
    dkim: Option<&crate::settings::DkimSettingsModel>,
) -> CheckItem {
    let title = format!("DKIM record of {}", record.domain);

    // A published record which holds another key almost always means the key file is not
    // on a persistent volume: the service then generates a new key on every start, and
    // whatever was published becomes stale.
    let key_file_hint = match dkim {
        Some(dkim) => format!(
            " The key is taken from '{}' - if that path is not on a persistent volume, a new key is generated on every restart and the published record goes stale.",
            dkim.private_key_path.trim()
        ),
        None => String::new(),
    };

    let published = match crate::scripts::lookup_txt_authoritative(record.name.as_str()).await {
        Ok(published) => published,
        Err(err) => {
            return CheckItem::failed(title, err).with_expected(record.value.clone());
        }
    };

    let Some(dkim_record) = published
        .iter()
        .find(|value| value.to_lowercase().contains("v=dkim1"))
    else {
        return CheckItem::failed(
            title,
            format!("Nothing is published as '{}' yet - until it is, every recipient fails to verify the signature", record.name),
        )
        .with_expected(record.value.clone());
    };

    match get_public_key_of(dkim_record.as_str()) {
        Some(published_key) if published_key == record.public_key => CheckItem::ok(
            title,
            format!("'{}' is published and matches the key the mail is signed with", record.name),
        )
        .with_actual(dkim_record.clone()),
        Some(_) => CheckItem::failed(
            title,
            format!(
                "'{}' is published, but it holds another public key - the signature can not be verified.{}",
                record.name, key_file_hint
            ),
        )
        .with_expected(record.value.clone())
        .with_actual(dkim_record.clone()),
        None => CheckItem::failed(
            title,
            format!("'{}' is published, but it has no p= value", record.name),
        )
        .with_expected(record.value.clone())
        .with_actual(dkim_record.clone()),
    }
}

async fn check_spf(domain: &str, public_ip: Option<IpAddr>, settings: &SettingsModel) -> CheckItem {
    let title = format!("SPF record of {}", domain);

    let expected = match (&settings.relay, public_ip) {
        (Some(relay), _) => format!(
            "v=spf1 include:{} -all   (a guess - the relay documents the exact include to use, and it is often not its smtp host name)",
            get_relay_spf_include(relay.host.trim())
        ),
        (None, Some(public_ip)) => format!("v=spf1 ip4:{} -all", public_ip),
        (None, None) => "v=spf1 ip4:{your outgoing ip} -all".to_string(),
    };

    let published = match crate::scripts::lookup_txt_authoritative(domain).await {
        Ok(published) => published,
        Err(err) => return CheckItem::failed(title, err).with_expected(expected),
    };

    let Some(spf_record) = published
        .iter()
        .find(|value| value.to_lowercase().starts_with("v=spf1"))
    else {
        return CheckItem::failed(
            title,
            "No SPF record is published. Without it the recipients treat the mail as unauthorized",
        )
        .with_expected(expected);
    };

    // A full SPF evaluation would mean following every include - what is checked here is
    // only whether the record can possibly cover this sender.
    if settings.relay.is_none()
        && let Some(public_ip) = public_ip
        && !spf_record.contains(&format!("ip4:{}", public_ip))
        && !spf_record.contains(&format!("ip6:{}", public_ip))
        && !spf_record.contains("include:")
        && !spf_record
            .split_whitespace()
            .any(|part| part == "a" || part == "mx")
    {
        return CheckItem::warning(
            title,
            "An SPF record is published, but it does not mention the outgoing ip address of this host",
        )
        .with_expected(expected)
        .with_actual(spf_record.clone());
    }

    CheckItem::ok(title, "An SPF record is published").with_actual(spf_record.clone())
}

async fn check_dmarc(domain: &str) -> CheckItem {
    let title = format!("DMARC record of {}", domain);
    let name = format!("_dmarc.{}", domain);
    let expected = format!("v=DMARC1; p=none; rua=mailto:postmaster@{}", domain);

    let published = match crate::scripts::lookup_txt_authoritative(name.as_str()).await {
        Ok(published) => published,
        Err(err) => {
            return CheckItem::warning(title, err).with_expected(expected);
        }
    };

    match published
        .iter()
        .find(|value| value.to_lowercase().starts_with("v=dmarc1"))
    {
        Some(dmarc_record) => {
            CheckItem::ok(title, format!("'{}' is published", name)).with_actual(dmarc_record.clone())
        }
        None => CheckItem::warning(
            title,
            format!(
                "'{}' is not published. The mail is delivered without it, but the big providers trust a domain with a DMARC policy more",
                name
            ),
        )
        .with_expected(expected),
    }
}

/// Where the key comes from, and whether that place survives a restart of the container.
/// A key which is generated into the container filesystem is a different key after every
/// restart, and the published dns record is stale from that moment on.
async fn check_dkim_key_file(dkim: &crate::settings::DkimSettingsModel) -> CheckItem {
    let title = format!("DKIM key file of {}", dkim.domain);
    let source_file = dkim.get_private_key_path();

    let source_key = match tokio::fs::read_to_string(source_file.as_str()).await {
        Ok(source_key) => source_key,
        Err(err) => {
            return CheckItem::failed(
                title,
                format!(
                    "'{}' can not be read now, although the mail server was started with a key. It means the key was generated into the filesystem of the container and a NEW one will be generated on the next restart. Mount that path from the host. Err: {}",
                    source_file, err
                ),
            )
            .with_expected(source_file);
        }
    };

    let used_key_file = crate::kumo_mta::get_dkim_private_key_file(dkim);

    let used_key = match tokio::fs::read_to_string(used_key_file.as_str()).await {
        Ok(used_key) => used_key,
        Err(err) => {
            return CheckItem::failed(
                title,
                format!(
                    "The mail server has no key at '{}'. Err: {}",
                    used_key_file, err
                ),
            );
        }
    };

    if source_key.trim() != used_key.trim() {
        return CheckItem::warning(
            title,
            format!(
                "'{}' holds another key than the one the mail server is signing with - it has been changed since the start up. Restart the mail server to apply it",
                source_file
            ),
        )
        .with_expected(source_file);
    }

    CheckItem::ok(
        title,
        format!(
            "The mail server signs with the key from '{}'. Make sure that path is mounted from the host, otherwise the next restart generates a new one",
            source_file
        ),
    )
    .with_actual(source_file)
}

/// `smtp.mailgun.org` -> `mailgun.org`: the smtp host name of a relay is not what its spf
/// include is, and the registrable domain is the closest guess which can be made without
/// knowing the provider.
fn get_relay_spf_include(relay_host: &str) -> String {
    let labels: Vec<&str> = relay_host.split('.').collect();

    if labels.len() < 3 {
        return relay_host.to_string();
    }

    labels[labels.len() - 2..].join(".")
}

/// The key is picked by the domain of the From header - a sender whose domain has no key
/// configured goes out unsigned, and nothing anywhere reports it as an error.
fn check_default_sender_is_signed(settings: &SettingsModel) -> CheckItem {
    let title = "Default sender is signed";
    let default_from_email = settings.smtp.default_from_email.trim();

    let Some((_, from_domain)) = default_from_email.split_once('@') else {
        return CheckItem::failed(
            title,
            format!(
                "'{}' is not an email address, so nothing can be sent with the default sender",
                default_from_email
            ),
        );
    };

    if settings.dkim.is_empty() {
        return CheckItem::warning(
            title,
            "No dkim key is configured at all - the mail goes out unsigned",
        );
    }

    let signed_domains: Vec<String> = settings
        .dkim
        .iter()
        .map(|dkim| dkim.domain.trim().to_lowercase())
        .collect();

    if signed_domains.contains(&from_domain.to_lowercase()) {
        return CheckItem::ok(
            title,
            format!("The mail from '{}' is dkim signed", default_from_email),
        )
        .with_actual(from_domain.to_string());
    }

    CheckItem::failed(
        title,
        format!(
            "The mail from '{}' goes out UNSIGNED: the key is picked by the domain of the From header, and there is no key for '{}'. Either send from one of the signed domains, or add a key for this one",
            default_from_email, from_domain
        ),
    )
    .with_expected(format!("a dkim entry with domain: {}", from_domain))
    .with_actual(format!("keys are configured for: {}", signed_domains.join(", ")))
}

/// The domains the service signs for - they are the ones whose dns has to be in order.
fn get_sending_domains(settings: &SettingsModel) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();

    for dkim in settings.dkim.iter() {
        let domain = dkim.domain.trim().to_string();

        if !result.contains(&domain) {
            result.push(domain);
        }
    }

    if result.is_empty()
        && let Some((_, domain)) = settings.smtp.default_from_email.trim().split_once('@')
    {
        result.push(domain.to_string());
    }

    result
}

/// A check which does not decide anything in the current configuration is a warning, not
/// a failure.
fn downgrade(item: CheckItem, matters: bool) -> CheckItem {
    if matters {
        return item;
    }

    CheckItem::warning(
        item.title,
        format!(
            "{}. It only matters for the direct delivery - the mail is handed over to the relay, so the relay answers for it",
            item.message
        ),
    )
}

fn get_public_key_of(dkim_record: &str) -> Option<String> {
    for part in dkim_record.split(';') {
        let part = part.trim();

        if let Some(public_key) = part.strip_prefix("p=") {
            return Some(public_key.trim().to_string());
        }
    }

    None
}
