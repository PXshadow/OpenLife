//! Append-only wire transcript: every client→server and server→client message.
//!
//! LOGIN / RLOGIN lines are redacted (email + HMAC) so transcripts are safer to share.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::secrets::{is_login_wire_line, redact_login_wire, redact_note};

/// Thread-safe transcript writer.
#[derive(Debug)]
pub struct WireLog {
    path: PathBuf,
    file: Mutex<File>,
}

impl WireLog {
    pub fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;
        writeln!(
            file,
            "# ohol-headless wire log\n# started_unix_ms={}\n# format: <ms> TX|RX <body>\n",
            now_ms()
        )?;
        file.flush()?;
        Ok(Self {
            path,
            file: Mutex::new(file),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn note(&self, text: &str) {
        let safe = redact_note(text);
        let _ = self.write_line(&format!("# {safe}"));
    }

    pub fn tx(&self, message: &str) {
        let shown = if message.ends_with('#') {
            message.to_string()
        } else {
            format!("{message}#")
        };
        let shown = if is_login_wire_line(&shown) {
            redact_login_wire(&shown)
        } else {
            shown
        };
        let flat = shown.replace('\n', "\\n");
        let _ = self.write_line(&format!("{} TX {}", now_ms(), flat));
    }

    pub fn rx(&self, body: &str) {
        let flat = body.replace('\n', "\\n");
        let flat = if flat.len() > 2000 {
            format!("{}…({} chars)", &flat[..2000], flat.len())
        } else {
            flat
        };
        let _ = self.write_line(&format!("{} RX {}#", now_ms(), flat));
    }

    fn write_line(&self, line: &str) -> io::Result<()> {
        let mut f = self.file.lock().map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("wire log lock: {e}"))
        })?;
        writeln!(f, "{line}")?;
        f.flush()
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
