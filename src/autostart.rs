//! Launch at login: a .desktop file on Linux, the Run key on Windows, a LaunchAgent on macOS.
use std::path::PathBuf;

fn home() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).unwrap_or_default())
}

pub fn set(enabled: bool) -> std::io::Result<()> {
    let exe = std::env::current_exe()?.to_string_lossy().to_string();
    if cfg!(target_os = "windows") {
        let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
        let args: Vec<&str> = if enabled {
            vec!["add", key, "/v", "TakeCareDev", "/t", "REG_SZ", "/d", &exe, "/f"]
        } else {
            vec!["delete", key, "/v", "TakeCareDev", "/f"]
        };
        crate::platform::quiet("reg").args(args).output()?;
        return Ok(());
    }
    let (file, body) = if cfg!(target_os = "macos") {
        let plist = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<plist version=\"1.0\"><dict><key>Label</key><string>dev.tcdev.app</string>\
             <key>ProgramArguments</key><array><string>{exe}</string></array><key>RunAtLoad</key><true/></dict></plist>\n"
        );
        (home().join("Library/LaunchAgents/dev.tcdev.app.plist"), plist)
    } else {
        let config = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".config"));
        let escaped: String = exe.chars().flat_map(|c| if "\"`$\\".contains(c) { vec!['\\', c] } else { vec![c] }).collect();
        let quoted = format!("\"{escaped}\"");
        let desktop = format!(
            "[Desktop Entry]\nType=Application\nName=TakeCareDev\nComment=Eye exercise and walking break reminders\nExec={quoted}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
        );
        (config.join("autostart/takecaredev.desktop"), desktop)
    };
    if !enabled {
        return match std::fs::remove_file(file) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    std::fs::create_dir_all(file.parent().unwrap())?;
    std::fs::write(file, body)
}
