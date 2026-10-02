use crate::timer::Session;
use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const HEADER: &str = "started_at,type,length_s,threshold_s\n";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub work_threshold_s: u64,
    pub break_threshold_s: u64,
    pub data_path: PathBuf,
}

impl Config {
    pub fn default_with(data_path: PathBuf) -> Self {
        Config { work_threshold_s: 25 * 60, break_threshold_s: 5 * 60, data_path }
    }
}

/// Load the config, falling back to defaults if it is missing or unreadable.
pub fn load_config(path: &Path, default_data: PathBuf) -> Config {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| Config::default_with(default_data))
}

pub fn save_config(path: &Path, cfg: &Config) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(cfg)?)?;
    fs::rename(tmp, path)
}

/// Append one row, writing the header first if the file is new or empty.
/// Synced to disk before returning so a following exit cannot lose it.
pub fn append_session(path: &Path, s: &Session) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    let mut out = String::new();
    if f.metadata()?.len() == 0 {
        out.push_str(HEADER);
    }
    out.push_str(&format!(
        "{},{},{},{}\n",
        s.started_at.to_rfc3339_opts(SecondsFormat::Secs, false),
        s.mode.as_str(),
        s.length_s,
        s.threshold_s
    ));
    f.write_all(out.as_bytes())?;
    f.flush()?;
    f.sync_all()
}

/// Move the data file to its new location if there is one to move and the
/// target does not exist yet. Otherwise the new path is simply used as-is.
pub fn move_data_file(old: &Path, new: &Path) -> io::Result<()> {
    if old == new || !old.exists() || new.exists() {
        return Ok(());
    }
    if let Some(dir) = new.parent() {
        fs::create_dir_all(dir)?;
    }
    if fs::rename(old, new).is_err() {
        // rename fails across filesystems
        fs::copy(old, new)?;
        fs::remove_file(old)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timer::Mode;
    use chrono::Local;

    fn tmp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("repoti-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    fn session(mode: Mode, length_s: u64) -> Session {
        Session { started_at: Local::now(), mode, length_s, threshold_s: 300 }
    }

    #[test]
    fn appends_header_once_then_rows() {
        let dir = tmp_dir("append");
        let p = dir.join("nested/sessions.csv");
        append_session(&p, &session(Mode::Work, 1700)).unwrap();
        append_session(&p, &session(Mode::Break, 420)).unwrap();
        let text = fs::read_to_string(&p).unwrap();
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], HEADER.trim_end());
        assert!(lines[1].ends_with(",work,1700,300"));
        assert!(lines[2].ends_with(",break,420,300"));
        // ISO8601 with offset, e.g. 2026-10-02T08:47:00+02:00
        assert!(chrono::DateTime::parse_from_rfc3339(lines[1].split(',').next().unwrap()).is_ok());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn moves_data_file_only_when_target_free() {
        let dir = tmp_dir("move");
        let old = dir.join("a.csv");
        let new = dir.join("sub/b.csv");
        append_session(&old, &session(Mode::Work, 1)).unwrap();
        move_data_file(&old, &new).unwrap();
        assert!(!old.exists() && new.exists());

        append_session(&old, &session(Mode::Work, 2)).unwrap();
        move_data_file(&old, &new).unwrap();
        assert!(old.exists(), "existing target must not be overwritten");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn config_roundtrip_and_fallback() {
        let dir = tmp_dir("config");
        let p = dir.join("config.json");
        let def = load_config(&p, dir.join("s.csv"));
        assert_eq!(def.work_threshold_s, 1500);
        let cfg = Config { work_threshold_s: 60, ..def };
        save_config(&p, &cfg).unwrap();
        assert_eq!(load_config(&p, PathBuf::new()).work_threshold_s, 60);
        fs::remove_dir_all(dir).unwrap();
    }
}
