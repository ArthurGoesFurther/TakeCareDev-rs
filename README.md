# TakeCareDev (Rust)

A pure-Rust rewrite of the Electron app in `../tcDev`: a little pixel face that reminds you to exercise
your eyes and to get up from the chair. No web engine: windows are drawn with [egui](https://egui.rs),
the tray uses StatusNotifierItem over D-Bus ([ksni](https://crates.io/crates/ksni)). The release binary is ~11 MB.

Settings and stats use the same JSON files as the Electron version (`~/.config/TakeCareDev/`), so they carry over.

## Build and run

```bash
cargo run --release          # tray daemon; first launch opens the settings window
cargo test                   # scheduler, settings, i18n, sprite, store
TCDEV_DATA_DIR=/tmp/tc cargo run --release   # separate settings profile
```

Launching it a second time just opens the settings window of the running instance.

## How it is put together

One binary, four roles (`src/main.rs`):

| Command                  | Role                                                                    |
| ------------------------ | ----------------------------------------------------------------------- |
| `takecaredev`            | the daemon: tray, timers, settings, stats; starts the windows below     |
| `takecaredev face <json>`| the pixel-face popup (transparent, click-through, never takes focus)    |
| `takecaredev break ...`  | the break card, or one full-screen cover per monitor in strict mode     |
| `takecaredev settings`   | the settings window                                                     |

Windows are separate processes because an egui/winit event loop can only run once per process and a hidden
root window does not keep child windows alive. They talk to the daemon in JSON lines over stdin/stdout
(`src/ipc.rs`).

```
src/
  daemon.rs       wiring: events, tray menu, face and break lifecycle
  scheduler.rs    pure timer logic with tests (clock is a parameter)
  settings.rs     schema, defaults, clamping
  exercises.rs    what the face does at each second
  sprite.rs       the face, drawn in code into any pixel sink
  i18n.rs         ru / en
  store.rs        settings.json and stats.json
  platform.rs     session type, pointer, monitors, idle time (X11 / Mutter / Windows / macOS)
  sound.rs        chirp rendered to WAV and played with paplay / pw-play / aplay / afplay
  tray.rs, notify.rs, autostart.rs
  ui/             face.rs, break_screen.rs, settings.rs
```

## Platform status

- **Linux (GNOME, Wayland/XWayland): built and tested.** Like the Electron version, windows run under
  XWayland so they can be positioned and the pointer can be found (a transparent probe window covers the
  screen for a split second). Idle time comes from Mutter over D-Bus. The tray needs the AppIndicator extension.
- **Windows:** tray ([tray-icon](https://crates.io/crates/tray-icon) + tao), pointer, monitors, idle time,
  autostart and notifications are written; `cargo check --target x86_64-pc-windows-gnu` passes.
  **Never run on Windows.**
- **macOS:** same, with CoreGraphics for the pointer, a template tray icon and no Dock icon;
  `cargo check --target aarch64-apple-darwin` passes. **Never run on macOS.** Monitors are not enumerated:
  the break card is centered on its own monitor, strict mode goes fullscreen on the monitor it opens on
  (so multi-monitor strict mode covers only one screen).

On Windows and macOS the tray owns the main thread's event loop and the daemon runs on a worker thread
(`tray::run_main`).

## Known differences from the Electron version

- No pop-in animation for the face (only a fade); bold text is not available in egui's default font.
- Screen scaling is assumed to be 1 px per point when the very first window position is chosen,
  then corrected; fractional scaling on X11 has not been tried.
- A fixed local port (`127.0.0.1:47821`) is the single-instance lock.
- Lock-screen events are not tracked; idle time alone decides "away".
