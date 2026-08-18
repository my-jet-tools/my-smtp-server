use std::net::{IpAddr, Ipv4Addr};

use hickory_resolver::{
    Resolver, TokioResolver,
    config::{NameServerConfig, ResolverConfig},
    net::runtime::TokioRuntimeProvider,
    proto::rr::RData,
};

/// The classic dns way to find out the ip address the world sees us as: opendns answers
/// this name with the address the query came from. It costs one dns lookup and needs no
/// http client and no third party api.
const MY_IP_NAME: &str = "myip.opendns.com";
const OPEN_DNS_RESOLVERS: [IpAddr; 2] = [
    IpAddr::V4(Ipv4Addr::new(208, 67, 222, 222)),
    IpAddr::V4(Ipv4Addr::new(208, 67, 220, 220)),
];

pub async fn lookup_txt(name: &str) -> Result<Vec<String>, String> {
    let resolver = create_system_resolver()?;

    let lookup = resolver
        .txt_lookup(to_fqdn(name))
        .await
        .map_err(|err| format!("Can not resolve the TXT of '{}'. Err: {}", name, err))?;

    Ok(read_txt_values(&lookup))
}

fn read_txt_values(lookup: &hickory_resolver::lookup::Lookup) -> Vec<String> {
    lookup
        .answers()
        .iter()
        .filter_map(|record| match &record.data {
            // A long TXT value is split into several strings on the wire - and a dkim
            // record always is, since a 2048 bit key does not fit into 255 bytes.
            RData::TXT(txt) => Some(
                txt.txt_data
                    .iter()
                    .map(|data| String::from_utf8_lossy(data).to_string())
                    .collect::<Vec<String>>()
                    .join(""),
            ),
            _ => None,
        })
        .collect()
}

/// The same as [`lookup_txt`], but asked directly of the name servers of the zone, so the
/// answer is never a cached copy. A record which has just been changed shows up here
/// immediately, while the resolver of the host can still be holding the old value for the
/// rest of its ttl - and a check-up which reports a stale value as a failure is worse than
/// no check-up at all. Falls back to the resolver of the host when the zone can not be
/// found.
pub async fn lookup_txt_authoritative(name: &str) -> Result<Vec<String>, String> {
    let name_servers = match lookup_zone_name_servers(name).await {
        Ok(name_servers) if !name_servers.is_empty() => name_servers,
        _ => return lookup_txt(name).await,
    };

    let config = ResolverConfig::from_parts(
        None,
        Vec::new(),
        name_servers
            .iter()
            .map(|ip| NameServerConfig::udp_and_tcp(*ip))
            .collect(),
    );

    let resolver =
        match Resolver::builder_with_config(config, TokioRuntimeProvider::default()).build() {
            Ok(resolver) => resolver,
            Err(_) => return lookup_txt(name).await,
        };

    let lookup = resolver
        .txt_lookup(to_fqdn(name))
        .await
        .map_err(|err| format!("Can not resolve the TXT of '{}'. Err: {}", name, err))?;

    Ok(read_txt_values(&lookup))
}

/// Walks up the labels of the name until a zone with name servers is found:
/// 'mail._domainkey.mydomain.com' has none of its own, 'mydomain.com' has.
async fn lookup_zone_name_servers(name: &str) -> Result<Vec<IpAddr>, String> {
    let resolver = create_system_resolver()?;

    let name = name.trim().trim_end_matches('.').to_string();
    let labels: Vec<&str> = name.split('.').collect();

    for skip in 0..labels.len() {
        let zone = labels[skip..].join(".");

        // Nothing below two labels can hold a zone we care about.
        if zone.split('.').count() < 2 {
            break;
        }

        let Ok(lookup) = resolver.ns_lookup(to_fqdn(zone.as_str())).await else {
            continue;
        };

        let mut result = Vec::new();

        for record in lookup.answers().iter() {
            let RData::NS(ns) = &record.data else {
                continue;
            };

            if let Ok(addresses) = lookup_a(ns.to_string().as_str()).await {
                result.extend(addresses);
            }
        }

        if !result.is_empty() {
            return Ok(result);
        }
    }

    Err(format!("Can not find the name servers of '{}'", name))
}

/// Only the A records: the check which uses it compares them with the ipv4 address the
/// world sees us as, and asking for AAAA as well only produces a confusing error on a host
/// which has no ipv6 at all.
pub async fn lookup_a(name: &str) -> Result<Vec<IpAddr>, String> {
    let resolver = create_system_resolver()?;

    let lookup = resolver
        .ipv4_lookup(to_fqdn(name))
        .await
        .map_err(|err| format!("Can not resolve the A record of '{}'. Err: {}", name, err))?;

    let result = lookup
        .answers()
        .iter()
        .filter_map(|record| match &record.data {
            RData::A(ip) => Some(IpAddr::V4(ip.0)),
            _ => None,
        })
        .collect();

    Ok(result)
}

pub async fn lookup_ptr(ip: IpAddr) -> Result<Vec<String>, String> {
    let resolver = create_system_resolver()?;

    let lookup = resolver
        .reverse_lookup(ip)
        .await
        .map_err(|err| format!("Can not resolve the PTR record of '{}'. Err: {}", ip, err))?;

    let result = lookup
        .answers()
        .iter()
        .filter_map(|record| match &record.data {
            RData::PTR(ptr) => Some(ptr.to_string().trim_end_matches('.').to_string()),
            _ => None,
        })
        .collect();

    Ok(result)
}

/// The ip address the recipient mail servers see the mail coming from.
pub async fn lookup_own_public_ip() -> Result<IpAddr, String> {
    let name_servers = OPEN_DNS_RESOLVERS
        .iter()
        .map(|ip| NameServerConfig::udp_and_tcp(*ip))
        .collect();

    let config = ResolverConfig::from_parts(None, Vec::new(), name_servers);

    let resolver = Resolver::builder_with_config(config, TokioRuntimeProvider::default())
        .build()
        .map_err(|err| format!("Can not create the dns resolver. Err: {}", err))?;

    let lookup = resolver
        .lookup_ip(MY_IP_NAME)
        .await
        .map_err(|err| format!("Can not find out the own public ip address. Err: {}", err))?;

    match lookup.iter().next() {
        Some(ip) => Ok(ip),
        None => Err("The public ip address lookup returned nothing".to_string()),
    }
}

/// Without the trailing dot the resolver treats the name as relative and walks the `search`
/// list of /etc/resolv.conf - which on a home connection is the domain of the provider, so
/// 'mail.mydomain.com' is asked for as 'mail.mydomain.com.provider.net' and resolves to
/// nothing. Every name this service looks up is absolute.
fn to_fqdn(name: &str) -> String {
    format!("{}.", name.trim().trim_end_matches('.'))
}

fn create_system_resolver() -> Result<TokioResolver, String> {
    Resolver::builder_tokio()
        .map_err(|err| format!("Can not create the dns resolver. Err: {}", err))?
        .build()
        .map_err(|err| format!("Can not create the dns resolver. Err: {}", err))
}
