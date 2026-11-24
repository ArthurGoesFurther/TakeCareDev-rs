# TakeCareDev

**English** · [Русский](README.ru.md) · [中文](README.zh-CN.md)

A little pixel face that reminds you to exercise your eyes and get up from the chair. Lives in the
tray (the menu bar on macOS) and runs on Windows, macOS and Linux.

It is a small native app written in Rust: no browser engine inside, a ~5 MB download, and it barely
touches the CPU while it waits.

![icon](assets/icon.png)

**Download:** [latest release](https://github.com/ArthurGoesFurther/takecaredev-rs/releases/latest)

| System                       | File                               |
| ---------------------------- | ---------------------------------- |
| macOS (Apple Silicon, M1–M4) | `takecaredev-macos-arm64.zip`      |
| Windows                      | `takecaredev-windows-x86_64.zip`   |
| Linux                        | `takecaredev-linux-x86_64.zip`     |

## Features

- **Eye exercises.** Every N minutes (20 by default, the 20-20-20 rule) a pixel face pops up next
  to your cursor and shows an exercise for N seconds (10 by default): left–right, up–down,
  circles, diagonals, figure eight, blinking, squeezing, looking far away, near–far focus, resting
  with closed eyes. You just follow its pupils. The window is transparent, click-through and never
  steals focus, so it doesn't interrupt typing.
- **Stand up and walk.** Every N minutes (50 by default) a break of N minutes (5 by default): a card
  with a timer, a tip (squats, stairs, stretching, water), "Snooze" and "Skip".
- **Strict mode.** The break covers the screen and hides the buttons. Emergency exit is in the
  tray menu.
- **Honest timer.** By default the break timer only runs while you don't touch the mouse and
  keyboard — sitting in front of the break screen doesn't count.
- **Natural breaks.** If you were away longer than N minutes (or the laptop slept), countdowns start
  over instead of nagging you right after you sit down.
- A notification N seconds before a break.
- Pause for 30 min / 1 h / 2 h / until resumed, daily stats, launch at login.
- Choose which exercises to show, their order, face size, a screen corner instead of the cursor,
  hint bubble and sound.
- English and Russian UI.

Every interval and duration is configurable: tray → "Settings…", or launch the app again.

### First launch

- **macOS:** unzip, move `TakeCareDev.app` to Applications. The app is not signed with an Apple
  developer ID, so the first launch is blocked: right-click the app → **Open** → **Open** (or System
  Settings → Privacy & Security → **Open Anyway**). After that it starts normally. It lives in the
  menu bar only; there is no Dock icon.
- **Windows:** unzip and run `takecaredev.exe`. The build is unsigned, so SmartScreen will warn —
  "More info" → "Run anyway".
- **Linux:** unzip, `chmod +x takecaredev`, run it. The tray icon on GNOME needs the AppIndicator
  extension (enabled by default on Ubuntu).

## Why this exists

Programmers sit for 8–12 hours a day staring at a screen half a meter away. Two things quietly
break from that: eyes and the lower body.

### Failing eyesight

- **Myopia is an epidemic.** About 30% of the world was nearsighted in 2020; the projection for 2050
  is ~50%, nearly 5 billion people. Long near work and little time looking at distance are among
  the main risk factors.
- **Accommodation spasm.** To focus at 50 cm the ciliary muscle stays contracted for hours. After a
  long day distance looks blurry, the eyes ache, and the head hurts — "computer vision syndrome".
- **Dry eye.** While reading a screen we blink about 3–4 times less often than normal (roughly
  15–20 times a minute drops to around 5). The tear film evaporates: burning, redness, sand in the
  eyes.
- **What helps:** the 20-20-20 rule recommended by ophthalmologists — every 20 minutes look at
  something ~6 m (20 ft) away for 20 seconds — plus deliberate blinking and changing focus. That's
  exactly what the face does.

To be clear: eye exercises won't reverse existing myopia. They relieve strain and dryness and keep
the habit of looking away, which is the part that's in your control.

### Hemorrhoids and sitting

- **Pressure.** Sitting puts constant pressure on the veins of the anal canal, and blood return
  from the pelvis slows down. The veins stretch and the cushions slide down — that's hemorrhoidal
  disease.
- **Motionless gut.** Little movement means slower bowel motility, constipation and straining —
  the main trigger of hemorrhoids. The same goes for long sitting on the toilet with a phone.
- **It's common.** Studies estimate hemorrhoidal disease in anywhere from ~4% to ~40% of adults
  depending on the method; most people just don't talk about it until it hurts.
- **What helps:** getting up every 45–60 minutes and walking for a few minutes. Walking restores
  venous return and gets the gut moving. Squats and stairs from the break tips work even better.

Long sitting is also tied to back and neck pain, worse circulation in the legs and higher
cardiovascular risk, so the walking break pays off several times over.

TakeCareDev doesn't replace a doctor. If something already hurts or your vision has dropped — see
an ophthalmologist / proctologist.

## How it works

Windows are drawn with [egui](https://egui.rs); the tray is StatusNotifierItem over D-Bus
([ksni](https://crates.io/crates/ksni)) on Linux and [tray-icon](https://crates.io/crates/tray-icon)
on Windows and macOS. The face and every other picture are drawn in code — there are no image
assets.

One binary plays four roles:

| Command                   | Role                                                                    |
| ------------------------- | ----------------------------------------------------------------------- |
| `takecaredev`             | the daemon: tray, timers, settings, stats; starts the windows below     |
| `takecaredev face <json>` | the pixel-face popup (transparent, click-through, never takes focus)    |
| `takecaredev break ...`   | the break card, or a full-screen cover in strict mode                   |
| `takecaredev settings`    | the settings window                                                     |

Windows are separate processes because an egui/winit event loop can only run once per process. They
talk to the daemon in JSON lines over stdin/stdout (`src/ipc.rs`). On Windows and macOS the tray owns
the main thread and the daemon runs on a worker thread.

```
src/
  main.rs         entry point: picks the role
  daemon.rs       wiring: events, tray menu, face and break lifecycle
  scheduler.rs    pure timer logic with tests (the clock is a parameter)
  settings.rs     settings schema, defaults, clamping
  exercises.rs    exercises: what the face does at each second
  sprite.rs       the face, drawn in code into any pixel sink
  i18n.rs         translations (ru, en)
  store.rs        settings.json and stats.json
  platform.rs     session type, pointer, monitors, idle time (X11 / Mutter / Windows / macOS)
  sound.rs        chirp rendered to WAV and played with afplay / paplay / pw-play / aplay
  png.rs          tiny PNG writer for the app icon
  tray.rs, notify.rs, autostart.rs, ipc.rs
  ui/             face.rs, break_screen.rs, settings.rs
scripts/
  macos-app.sh    builds TakeCareDev.app (with icon, ad-hoc signed)
```

Settings and stats live in `~/Library/Application Support/TakeCareDev/` on macOS,
`%APPDATA%\TakeCareDev\` on Windows and `~/.config/TakeCareDev/` on Linux. The files use the same
format as the earlier Electron version of TakeCareDev, so settings carry over.

### Build

Needs a [Rust toolchain](https://rustup.rs).

```bash
cargo run --release                          # tray daemon; first launch opens the settings window
cargo test                                   # scheduler, settings, i18n, sprite, store
TCDEV_DATA_DIR=/tmp/tc cargo run --release   # separate settings profile
scripts/macos-app.sh /Applications           # macOS: build and install TakeCareDev.app
```

Pushing a `v*` tag builds all three platforms on GitHub Actions and publishes a release.

### Platform notes

- **macOS:** tested on Apple Silicon. The pointer comes from CoreGraphics, idle time from
  `IOHIDSystem`. Monitors are not enumerated yet: the break card is centered on the screen it opens
  on, and strict mode covers only that one screen.
- **Linux (GNOME, Wayland/XWayland):** tested. Windows run under XWayland so they can be positioned
  and the pointer can be found (a transparent probe window covers the screen for a split second).
  Idle time comes from Mutter (`org.gnome.Mutter.IdleMonitor` over D-Bus); on other Wayland
  compositors idle-based features are disabled. On Wayland the face appears next to the cursor but
  can't follow it.
- **Windows:** tested on Windows 11 (two monitors, 100% scaling). Transparent windows (the face, the
  break card) are layered windows fed with `UpdateLayeredWindow`, because transparent OpenGL windows
  show up empty or white depending on the driver. Mixed-DPI monitors have not been tried.
- Only one copy runs at a time (a fixed local port, `127.0.0.1:47821`, is the lock); launching it
  again opens the settings of the running copy.

## Sources

- Holden B.A. et al., _Global Prevalence of Myopia and High Myopia and Temporal Trends from 2000
  through 2050_, Ophthalmology, 2016.
- American Academy of Ophthalmology, _Computers, Digital Devices and Eye Strain_ (20-20-20 rule).
- Patel S. et al., _Effect of visual display unit use on blink rate and tear stability_, Optometry
  and Vision Science, 1991.
- Johanson J.F., Sonnenberg A., _The prevalence of hemorrhoids and chronic constipation_,
  Gastroenterology, 1990.
- Riss S. et al., _The prevalence of hemorrhoids in adults_, International Journal of Colorectal
  Disease, 2012.
- Lohsiriwat V., _Hemorrhoids: from basic pathophysiology to clinical management_, World Journal of
  Gastroenterology, 2012.
