pub struct SmtpSubmitResult {
    /// Id of the message inside the postfix queue - it is the id every line of the
    /// postfix log of this message is prefixed with.
    pub queue_id: Option<String>,
    pub smtp_response: String,
}

impl SmtpSubmitResult {
    pub fn new(smtp_response: String) -> Self {
        Self {
            queue_id: extract_queue_id(smtp_response.as_str()),
            smtp_response,
        }
    }
}

/// KumoMTA answers with `OK ids=303b51a79b1711f1b81beaf9a57beb6c`, other mail servers with
/// `250 2.0.0 Ok: queued as 4bTgZ12Rz3zP` - both forms carry the id of the message.
fn extract_queue_id(smtp_response: &str) -> Option<String> {
    for prefix in ["ids=", "queued as "] {
        let Some(index) = smtp_response.find(prefix) else {
            continue;
        };

        let queue_id = smtp_response[index + prefix.len()..]
            .split_whitespace()
            .next()?;

        if !queue_id.is_empty() {
            return Some(queue_id.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::extract_queue_id;

    #[test]
    fn test_queue_id_is_extracted() {
        let result = extract_queue_id("2.0.0 Ok: queued as 4bTgZ12Rz3zP");
        assert_eq!(result.unwrap().as_str(), "4bTgZ12Rz3zP");
    }

    #[test]
    fn test_kumo_mta_queue_id_is_extracted() {
        let result = extract_queue_id("OK ids=303b51a79b1711f1b81beaf9a57beb6c");
        assert_eq!(result.unwrap().as_str(), "303b51a79b1711f1b81beaf9a57beb6c");
    }

    #[test]
    fn test_no_queue_id() {
        let result = extract_queue_id("2.0.0 Ok");
        assert!(result.is_none());
    }
}
