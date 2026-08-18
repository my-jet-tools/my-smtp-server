use crate::kumo_mta::LOG_DIR;

/// The mail server writes one json record per delivery attempt - Reception, Delivery,
/// TransientFailure, Bounce - into zstd compressed segments in its log directory. That is
/// the only place where the answer of the recipient mail server is written down, so it is
/// the only place which can tell why a message did not arrive.
pub async fn read_delivery_log(amount_of_records: usize) -> Result<Vec<String>, String> {
    let mut segments = read_segment_files().await?;

    // The names of the segments are timestamps, so the newest ones are the last.
    segments.sort();

    let mut result: Vec<String> = Vec::new();

    for segment in segments.iter().rev() {
        let mut records = read_segment(segment.as_str()).await?;

        records.append(&mut result);
        result = records;

        if result.len() >= amount_of_records {
            break;
        }
    }

    let skip = result.len().saturating_sub(amount_of_records);

    Ok(result.into_iter().skip(skip).collect())
}

async fn read_segment_files() -> Result<Vec<String>, String> {
    let mut read_dir = match tokio::fs::read_dir(LOG_DIR).await {
        Ok(read_dir) => read_dir,
        Err(err) => {
            return Err(format!(
                "Can not read the log directory '{}'. Err: {}",
                LOG_DIR, err
            ));
        }
    };

    let mut result = Vec::new();

    loop {
        let entry = match read_dir.next_entry().await {
            Ok(entry) => entry,
            Err(err) => {
                return Err(format!(
                    "Can not read the log directory '{}'. Err: {}",
                    LOG_DIR, err
                ));
            }
        };

        let Some(entry) = entry else {
            break;
        };

        if entry.path().is_dir() {
            continue;
        }

        result.push(entry.path().to_string_lossy().to_string());
    }

    Ok(result)
}

async fn read_segment(file_name: &str) -> Result<Vec<String>, String> {
    let content = match tokio::fs::read(file_name).await {
        Ok(content) => content,
        Err(err) => {
            return Err(format!(
                "Can not read the log segment '{}'. Err: {}",
                file_name, err
            ));
        }
    };

    // The segment which is being written right now is not compressed yet.
    let content = match zstd::stream::decode_all(content.as_slice()) {
        Ok(decoded) => decoded,
        Err(_) => content,
    };

    let content = String::from_utf8_lossy(content.as_slice()).to_string();

    Ok(content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
        .collect())
}
