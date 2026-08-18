use base64::{Engine, engine::general_purpose::STANDARD};
use rsa::{
    RsaPrivateKey,
    pkcs1::{DecodeRsaPrivateKey, EncodeRsaPrivateKey, LineEnding},
    pkcs8::EncodePublicKey,
};

use crate::settings::SettingsModel;

/// 2048 is what the big mailbox providers expect today. 1024 still verifies, but it is the
/// bottom of what is accepted, and there is no reason to start there.
const KEY_SIZE: usize = 2048;

pub struct DkimDnsRecord {
    pub domain: String,
    /// Name of the TXT record: {selector}._domainkey.{domain}
    pub name: String,
    /// Value of the TXT record: v=DKIM1; k=rsa; p=...
    pub value: String,
    /// Just the base64 of the public key - the `p=` part, which is what has to be compared
    /// with what is published in dns.
    pub public_key: String,
}

pub struct DkimDnsRecords {
    pub records: Vec<DkimDnsRecord>,
    /// Keys whose record could not be compiled, and why.
    pub errors: Vec<String>,
}

/// Compiles the dns records of every configured key. The keys are read from the copies the
/// mail server signs with - not from the files the settings point at, which could have been
/// changed after the mail server was started.
pub async fn collect_dkim_dns_records(settings: &SettingsModel) -> DkimDnsRecords {
    let mut records = Vec::new();
    let mut errors = Vec::new();

    for dkim in settings.dkim.iter() {
        let private_key_file = crate::kumo_mta::get_dkim_private_key_file(dkim);

        let private_key = match tokio::fs::read_to_string(private_key_file.as_str()).await {
            Ok(private_key) => private_key,
            Err(err) => {
                errors.push(format!(
                    "Can not read the private key of the domain '{}' from '{}'. Err: {}",
                    dkim.domain, private_key_file, err
                ));
                continue;
            }
        };

        match compile_dkim_dns_record(&dkim.domain, &dkim.selector, private_key.as_str()) {
            Ok(record) => records.push(record),
            Err(err) => errors.push(format!(
                "Can not compile the dns record of the domain '{}'. Err: {}",
                dkim.domain, err
            )),
        }
    }

    DkimDnsRecords { records, errors }
}

/// Generates a new dkim private key in the PKCS#1 PEM format - the one dkim tooling
/// expects (`-----BEGIN RSA PRIVATE KEY-----`).
pub async fn generate_dkim_private_key() -> Result<String, String> {
    // Generating a 2048 bit key is a second of pure cpu - it must not block the runtime.
    let result = tokio::task::spawn_blocking(|| {
        let mut rng = rsa::rand_core::OsRng;

        let private_key = RsaPrivateKey::new(&mut rng, KEY_SIZE)
            .map_err(|err| format!("Can not generate the dkim private key. Err: {}", err))?;

        private_key
            .to_pkcs1_pem(LineEnding::LF)
            .map_err(|err| format!("Can not encode the dkim private key. Err: {}", err))
    })
    .await;

    match result {
        Ok(result) => Ok(result?.to_string()),
        Err(err) => Err(format!(
            "The task which generates the dkim private key is failed. Err: {}",
            err
        )),
    }
}

/// Compiles the dns record which has to be published for the key - without it the signature
/// can not be verified by anybody.
pub fn compile_dkim_dns_record(
    domain: &str,
    selector: &str,
    private_key_pem: &str,
) -> Result<DkimDnsRecord, String> {
    let private_key = read_private_key(private_key_pem)?;

    let public_key_der = private_key
        .to_public_key()
        .to_public_key_der()
        .map_err(|err| format!("Can not extract the public key. Err: {}", err))?;

    let public_key = STANDARD.encode(public_key_der.as_bytes());

    Ok(DkimDnsRecord {
        domain: domain.trim().to_string(),
        name: format!("{}._domainkey.{}", selector.trim(), domain.trim()),
        value: format!("v=DKIM1; k=rsa; p={}", public_key),
        public_key,
    })
}

/// Both formats are accepted on the reading side: PKCS#1 is what `openssl genrsa
/// -traditional` writes, PKCS#8 is what OpenSSL 3 writes by default.
fn read_private_key(private_key_pem: &str) -> Result<RsaPrivateKey, String> {
    if let Ok(private_key) = RsaPrivateKey::from_pkcs1_pem(private_key_pem) {
        return Ok(private_key);
    }

    match rsa::pkcs8::DecodePrivateKey::from_pkcs8_pem(private_key_pem) {
        Ok(private_key) => Ok(private_key),
        Err(err) => Err(format!(
            "The private key is neither a valid PKCS#1 nor a valid PKCS#8 pem. Err: {}",
            err
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{compile_dkim_dns_record, generate_dkim_private_key};

    #[tokio::test]
    async fn test_generated_key_produces_a_dns_record() {
        let private_key = generate_dkim_private_key().await.unwrap();

        assert!(private_key.starts_with("-----BEGIN RSA PRIVATE KEY-----"));

        let record = compile_dkim_dns_record("mydomain.com", "mail", private_key.as_str()).unwrap();

        assert_eq!(record.name.as_str(), "mail._domainkey.mydomain.com");
        assert!(record.value.starts_with("v=DKIM1; k=rsa; p="));
    }
}
