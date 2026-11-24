//! Desktop notifications through the OS's own command-line tool.
use std::process::Stdio;

pub fn notify(title: &str, body: &str) {
    let mut cmd = if cfg!(target_os = "macos") {
        let mut c = crate::platform::quiet("osascript");
        c.args(["-e", &format!("display notification {body:?} with title {title:?}")]);
        c
    } else if cfg!(target_os = "windows") {
        let mut c = crate::platform::quiet("powershell");
        c.args(["-NoProfile", "-Command", &format!(
            "Add-Type -AssemblyName System.Windows.Forms; $n = New-Object System.Windows.Forms.NotifyIcon; \
             $n.Icon = [System.Drawing.SystemIcons]::Information; $n.Visible = $true; \
             $n.ShowBalloonTip(5000, '{}', '{}', 'Info'); Start-Sleep 6; $n.Dispose()",
            title.replace('\'', "''"), body.replace('\'', "''"))]);
        c
    } else {
        let mut c = crate::platform::quiet("notify-send");
        c.args(["--app-name=TakeCareDev", title, body]);
        c
    };
    // Fire and forget: a missing tool just means no notification.
    let _ = cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map(|mut c| std::thread::spawn(move || c.wait()));
}
