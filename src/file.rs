use std::{fmt::Display, path::PathBuf, time::SystemTime};

use time::OffsetDateTime;

pub struct File {
    pub modified: SystemTime,
    pub path: PathBuf,
}

impl PartialEq for File {
    fn eq(&self, other: &Self) -> bool {
        round_to_s(self.modified) == round_to_s(other.modified)
    }
}

impl PartialOrd for File {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(round_to_s(self.modified).cmp(&round_to_s(other.modified)))
    }
}

impl Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({})",
            self.path.display(),
            format_timestamp(self.modified),
        )
    }
}

impl Default for File {
    fn default() -> Self {
        Self {
            modified: SystemTime::UNIX_EPOCH,
            path: PathBuf::new(),
        }
    }
}

/// Format a Timestamp as the elapsed seconds, and milliseconds since the time.
fn format_timestamp(ts: SystemTime) -> String {
    let elapsed = OffsetDateTime::now_utc() - OffsetDateTime::from(ts);
    format!("{elapsed:.3}")
}

fn round_to_s(ts: SystemTime) -> u64 {
    ts.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
}
