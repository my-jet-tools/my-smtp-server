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
        .txt_lookup(name)
        .await
        .map_err(|err| format!("Can not resolve the TXT of '{}'. Err: {}", name, err))?;

    let result = lookup
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
        .collect();

    Ok(result)
}

pub async fn lookup_ip(name: &str) -> Result<Vec<IpAddr>, String> {
    let resolver = create_system_resolver()?;

    let lookup = resolver
        .lookup_ip(name)
        .await
        .map_err(|err| format!("Can not resolve the ip address of '{}'. Err: {}", name, err))?;

    Ok(lookup.iter().collect())
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

fn create_system_resolver() -> Result<TokioResolver, String> {
    Resolver::builder_tokio()
        .map_err(|err| format!("Can not create the dns resolver. Err: {}", err))?
        .build()
        .map_err(|err| format!("Can not create the dns resolver. Err: {}", err))
}
