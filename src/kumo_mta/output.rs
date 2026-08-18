use std::{collections::VecDeque, sync::Arc};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{ChildStderr, ChildStdout},
    sync::Mutex,
};

/// How many last lines of the kumod output are kept in memory. Everything kumod writes
/// also goes to the stdout of the container - this buffer exists so the output can be
/// read remotely, without an access to the docker logs.
const MAX_LINES: usize = 2000;

pub struct KumoMtaOutput {
    lines: Mutex<VecDeque<String>>,
}

impl KumoMtaOutput {
    pub fn new() -> Self {
        Self {
            lines: Mutex::new(VecDeque::with_capacity(MAX_LINES)),
        }
    }

    pub async fn add_line(&self, line: String) {
        let mut write_access = self.lines.lock().await;

        while write_access.len() >= MAX_LINES {
            write_access.pop_front();
        }

        write_access.push_back(line);
    }

    pub async fn get_last_lines(&self, amount: usize) -> Vec<String> {
        let read_access = self.lines.lock().await;

        let skip = read_access.len().saturating_sub(amount);

        read_access.iter().skip(skip).cloned().collect()
    }
}

pub fn start_reading_std_out(output: Arc<KumoMtaOutput>, std_out: ChildStdout) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(std_out).lines();

        while let Ok(Some(line)) = lines.next_line().await {
            println!("{}", line);
            output.add_line(line).await;
        }
    });
}

pub fn start_reading_std_err(output: Arc<KumoMtaOutput>, std_err: ChildStderr) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(std_err).lines();

        while let Ok(Some(line)) = lines.next_line().await {
            eprintln!("{}", line);
            output.add_line(line).await;
        }
    });
}
