//! Settings and daily stats as JSON files in the app's data directory (same layout as the Electron app).
use crate::settings::Settings;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("TCDEV_DATA_DIR") {
        return dir.into();
    }
    let home = || PathBuf::from(std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).unwrap_or_default());
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| home().join("AppData/Roaming"))
    } else if cfg!(target_os = "macos") {
        home().join("Library/Application Support")
    } else {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".config"))
    };
    base.join("TakeCareDev")
}

/// Writes through a temp file so a crash never leaves half a JSON document behind.
fn write_json(file: &Path, data: &impl Serialize) {
    let write = || -> std::io::Result<()> {
        std::fs::create_dir_all(file.parent().unwrap_or(Path::new(".")))?;
        let tmp = file.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(data).map_err(std::io::Error::other)?)?;
        std::fs::rename(tmp, file)
    };
    if let Err(e) = write() {
        eprintln!("[tcdev] cannot write {}: {e}", file.display());
    }
}

pub struct SettingsStore {
    file: PathBuf,
    pub existed: bool,
    pub current: Settings,
}

impl SettingsStore {
    pub fn open(file: PathBuf) -> Self {
        let existed = file.exists();
        let current = std::fs::read(&file)
            .ok()
            .and_then(|raw| serde_json::from_slice::<Settings>(&raw).ok())
            .unwrap_or_default()
            .normalized();
        write_json(&file, &current);
        Self { file, existed, current }
    }

    pub fn update(&mut self, next: Settings) -> Settings {
        self.current = next.normalized();
        write_json(&self.file, &self.current);
        self.current.clone()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Today {
    pub day: String,
    pub eyes: u32,
    pub walks: u32,
    pub skipped: u32,
    pub snoozed: u32,
}

pub struct Stats {
    file: PathBuf,
    data: Today,
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

impl Stats {
    pub fn open(file: PathBuf) -> Self {
        let data = std::fs::read(&file).ok().and_then(|raw| serde_json::from_slice(&raw).ok()).unwrap_or_default();
        Self { file, data }
    }

    pub fn today(&mut self) -> Today {
        if self.data.day != today() {
            self.data = Today { day: today(), ..Today::default() };
        }
        self.data.clone()
    }

    pub fn bump(&mut self, key: &str) {
        self.today();
        match key {
            "eyes" => self.data.eyes += 1,
            "walks" => self.data.walks += 1,
            "skipped" => self.data.skipped += 1,
            "snoozed" => self.data.snoozed += 1,
            _ => return,
        }
        write_json(&self.file, &self.data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_stats_count() {
        let dir = std::env::temp_dir().join(format!("tcdev-test-{}", std::process::id()));
        let mut store = SettingsStore::open(dir.join("settings.json"));
        assert!(!store.existed);
        let mut next = store.current.clone();
        next.eyes.interval_min = 33.0;
        store.update(next);
        let again = SettingsStore::open(dir.join("settings.json"));
        assert!(again.existed);
        assert_eq!(again.current.eyes.interval_min, 33.0);

        let mut stats = Stats::open(dir.join("stats.json"));
        stats.bump("eyes");
        stats.bump("eyes");
        assert_eq!(Stats::open(dir.join("stats.json")).data.eyes, 2);
        let _ = std::fs::remove_dir_all(dir);
    }
}
