//! Things the OS has to tell us: session type, pointer, monitors, idle time.
use std::process::Command;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn contains(&self, (px, py): (i32, i32)) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.w && py < self.y + self.h
    }

    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    fn intersect(&self, o: &Rect) -> Option<Rect> {
        let (x, y) = (self.x.max(o.x), self.y.max(o.y));
        let (r, b) = ((self.x + self.w).min(o.x + o.w), (self.y + self.h).min(o.y + o.h));
        (r > x && b > y).then_some(Rect { x, y, w: r - x, h: b - y })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Session {
    pub wayland: bool,
    pub xwayland: bool,
}

pub fn session() -> Session {
    static S: OnceLock<Session> = OnceLock::new();
    *S.get_or_init(|| {
        let var = |k| std::env::var(k).ok().filter(|v| !v.is_empty());
        let wayland = cfg!(target_os = "linux")
            && (var("XDG_SESSION_TYPE").as_deref() == Some("wayland")
                || (var("WAYLAND_DISPLAY").is_some() && var("XDG_SESSION_TYPE").as_deref() != Some("x11")));
        Session { wayland, xwayland: wayland && var("DISPLAY").is_some() }
    })
}

/// Whether pointer coordinates are in points (macOS) rather than physical pixels.
pub const POINTER_IN_POINTS: bool = cfg!(target_os = "macos");

/// direct (X11, Windows, macOS), probe (XWayland: pointer is only known over our own windows) or none.
pub fn cursor_mode() -> &'static str {
    let s = session();
    match (s.wayland, s.xwayland) {
        (false, _) => "direct",
        (true, true) => "probe",
        _ => "none",
    }
}

/// A command that never flashes a console window on Windows.
pub fn quiet(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    std::os::windows::process::CommandExt::creation_flags(&mut cmd, 0x0800_0000); // CREATE_NO_WINDOW
    cmd
}

#[allow(dead_code)] // only some platforms read idle time or the like through a helper command
fn run(program: &str, args: &[&str]) -> Option<String> {
    let out = quiet(program).args(args).output().ok().filter(|o| o.status.success())?;
    String::from_utf8(out.stdout).ok()
}

// ---------- Linux ----------

#[cfg(target_os = "linux")]
mod sys {
    use super::*;
    use x11rb::connection::Connection;
    use x11rb::protocol::randr::ConnectionExt as _;
    use x11rb::protocol::screensaver::ConnectionExt as _;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _};
    use x11rb::rust_connection::RustConnection;

    pub struct X {
        conn: RustConnection,
        root: u32,
    }

    impl X {
        pub fn connect() -> Option<X> {
            let (conn, screen) = x11rb::connect(None).ok()?;
            let root = conn.setup().roots.get(screen)?.root;
            Some(X { conn, root })
        }

        pub fn pointer(&self) -> Option<(i32, i32)> {
            let r = self.conn.query_pointer(self.root).ok()?.reply().ok()?;
            Some((r.root_x as i32, r.root_y as i32))
        }

        pub fn screen_size(&self) -> (i32, i32) {
            let s = &self.conn.setup().roots[0];
            (s.width_in_pixels as i32, s.height_in_pixels as i32)
        }

        /// Monitors clipped to the desktop work area (panels and docks excluded).
        pub fn monitors(&self) -> Vec<Rect> {
            let monitors: Vec<Rect> = self
                .conn
                .randr_get_monitors(self.root, true)
                .ok()
                .and_then(|c| c.reply().ok())
                .map(|r| r.monitors.iter().map(|m| Rect { x: m.x as i32, y: m.y as i32, w: m.width as i32, h: m.height as i32 }).collect())
                .unwrap_or_default();
            let work = self.work_area();
            monitors.into_iter().map(|m| work.and_then(|w| m.intersect(&w)).unwrap_or(m)).collect()
        }

        fn work_area(&self) -> Option<Rect> {
            let atom = self.conn.intern_atom(false, b"_NET_WORKAREA").ok()?.reply().ok()?.atom;
            let r = self.conn.get_property(false, self.root, atom, AtomEnum::CARDINAL, 0, 4).ok()?.reply().ok()?;
            let v: Vec<u32> = r.value32()?.collect();
            (v.len() == 4).then(|| Rect { x: v[0] as i32, y: v[1] as i32, w: v[2] as i32, h: v[3] as i32 })
        }

        pub fn idle_sec(&self) -> Option<f64> {
            let r = self.conn.screensaver_query_info(self.root).ok()?.reply().ok()?;
            Some(r.ms_since_user_input as f64 / 1000.0)
        }
    }

    pub fn mutter_idle_sec() -> Option<f64> {
        let out = run(
            "gdbus",
            &["call", "--session", "--dest", "org.gnome.Mutter.IdleMonitor", "--object-path", "/org/gnome/Mutter/IdleMonitor/Core", "--method", "org.gnome.Mutter.IdleMonitor.GetIdletime"],
        )?;
        let digits: String = out.split("uint64").nth(1)?.trim_start().chars().take_while(char::is_ascii_digit).collect();
        digits.parse::<f64>().ok().map(|ms| ms / 1000.0)
    }
}

// ---------- Windows ----------

#[cfg(target_os = "windows")]
mod sys {
    use super::*;
    use windows_sys::Win32::Foundation::{LPARAM, POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO};
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    pub fn pointer() -> Option<(i32, i32)> {
        let mut p = POINT { x: 0, y: 0 };
        (unsafe { GetCursorPos(&mut p) } != 0).then_some((p.x, p.y))
    }

    pub fn monitors() -> Vec<Rect> {
        unsafe extern "system" fn each(m: HMONITOR, _: HDC, _: *mut RECT, out: LPARAM) -> i32 {
            let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if unsafe { GetMonitorInfoW(m, &mut info) } != 0 {
                let r = info.rcWork;
                unsafe { &mut *(out as *mut Vec<Rect>) }.push(Rect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top });
            }
            1
        }
        let mut list: Vec<Rect> = vec![];
        unsafe { EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), Some(each), &mut list as *mut _ as LPARAM) };
        list
    }

    pub fn idle_sec() -> Option<f64> {
        let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
        (unsafe { GetLastInputInfo(&mut info) } != 0).then(|| unsafe { GetTickCount() }.wrapping_sub(info.dwTime) as f64 / 1000.0)
    }
}

// ---------- Windows: transparent windows ----------
//
// A transparent OpenGL window is unreliable on Windows: depending on the driver and DWM, what egui
// draws shows up, or the window is an empty frame (the face), or a blank white box (the break card).
// So transparent windows are layered windows fed with `UpdateLayeredWindow`: every frame egui paints
// as usual, the frame is read back and handed to Windows with its per-pixel alpha. Pixels with zero
// alpha are also click-through, so the break card's transparent margin does not block the desktop.

/// Makes `hwnd` a layered window whose content comes from [`present_layered`].
/// `click_through` also lets clicks on opaque pixels through (the face).
#[cfg(target_os = "windows")]
pub fn layered_window(hwnd: isize, click_through: bool) {
    use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    // Each window process has exactly one such window.
    static ORIGINAL: AtomicIsize = AtomicIsize::new(0);
    static EXTRA_EX_STYLE: AtomicU32 = AtomicU32::new(0);
    unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            // winit shifts an undecorated window's client area down by 1 px (to get a DWM shadow);
            // the client area must be the whole window so the frame lines up with it.
            WM_NCCALCSIZE if wparam != 0 => return 0,
            // winit rewrites the extended style whenever the window moves or resizes.
            WM_STYLECHANGING if wparam as i32 == GWL_EXSTYLE => {
                unsafe { (*(lparam as *mut STYLESTRUCT)).styleNew |= EXTRA_EX_STYLE.load(Ordering::Relaxed) };
            }
            _ => {}
        }
        let original: WNDPROC = unsafe { std::mem::transmute(ORIGINAL.load(Ordering::Relaxed)) };
        unsafe { CallWindowProcW(original, hwnd, msg, wparam, lparam) }
    }

    let hwnd = hwnd as HWND;
    let extra = WS_EX_LAYERED | if click_through { WS_EX_TRANSPARENT } else { 0 };
    EXTRA_EX_STYLE.store(extra, Ordering::Relaxed);
    unsafe {
        if ORIGINAL.load(Ordering::Relaxed) == 0 {
            ORIGINAL.store(SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize), Ordering::Relaxed);
        }
        SetWindowLongW(hwnd, GWL_EXSTYLE, GetWindowLongW(hwnd, GWL_EXSTYLE) | extra as i32);
        SetWindowPos(hwnd, std::ptr::null_mut(), 0, 0, 0, 0, SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

/// Shows one frame (premultiplied RGBA, top row first, `w`×`h` physical pixels) in a layered window.
#[cfg(target_os = "windows")]
pub fn present_layered(hwnd: isize, w: i32, h: i32, rgba: impl Iterator<Item = [u8; 4]>) {
    use windows_sys::Win32::Foundation::{POINT, SIZE};
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::{ULW_ALPHA, UpdateLayeredWindow};
    if w <= 0 || h <= 0 {
        return;
    }
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let mem = CreateCompatibleDC(screen);
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = w;
        info.bmiHeader.biHeight = -h; // top-down
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let bitmap = CreateDIBSection(screen, &info, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
        if !bitmap.is_null() && !bits.is_null() {
            let dst = std::slice::from_raw_parts_mut(bits as *mut u8, (w * h * 4) as usize);
            for (px, [r, g, b, a]) in dst.chunks_exact_mut(4).zip(rgba) {
                px.copy_from_slice(&[b, g, r, a]);
            }
            let old = SelectObject(mem, bitmap);
            let size = SIZE { cx: w, cy: h };
            let origin = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
            UpdateLayeredWindow(hwnd as _, screen, std::ptr::null(), &size, mem, &origin, 0, &blend, ULW_ALPHA);
            SelectObject(mem, old);
            DeleteObject(bitmap);
        }
        DeleteDC(mem);
        ReleaseDC(std::ptr::null_mut(), screen);
    }
}

// ---------- macOS ----------

#[cfg(target_os = "macos")]
mod sys {
    use super::*;
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    /// Global pointer position in points, origin at the top left of the primary display.
    pub fn pointer() -> Option<(i32, i32)> {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok()?;
        let p = CGEvent::new(source).ok()?.location();
        Some((p.x as i32, p.y as i32))
    }

    pub fn idle_sec() -> Option<f64> {
        let out = run("ioreg", &["-c", "IOHIDSystem", "-d", "4"])?;
        let ns: f64 = out.lines().find(|l| l.contains("HIDIdleTime"))?.rsplit('=').next()?.trim().parse().ok()?;
        Some(ns / 1e9)
    }
}

/// Monitor work areas in pixels (empty if unknown: callers fall back to the window's own monitor).
pub fn monitors() -> Vec<Rect> {
    #[cfg(target_os = "linux")]
    return sys::X::connect().map(|x| x.monitors()).unwrap_or_default();
    #[cfg(target_os = "windows")]
    return sys::monitors();
    #[cfg(target_os = "macos")]
    return vec![];
}

/// Stateful access to the pointer, so a window can poll it cheaply.
pub struct Pointer {
    #[cfg(target_os = "linux")]
    x: Option<sys::X>,
}

impl Pointer {
    pub fn new() -> Self {
        Pointer {
            #[cfg(target_os = "linux")]
            x: sys::X::connect(),
        }
    }

    pub fn get(&self) -> Option<(i32, i32)> {
        #[cfg(target_os = "linux")]
        return self.x.as_ref()?.pointer();
        #[cfg(target_os = "windows")]
        return sys::pointer();
        #[cfg(target_os = "macos")]
        return sys::pointer();
    }

    /// Size of the whole X screen: big enough to catch the pointer entering our probe window.
    pub fn screen_size(&self) -> Option<(i32, i32)> {
        #[cfg(target_os = "linux")]
        return self.x.as_ref().map(|x| x.screen_size());
        #[cfg(not(target_os = "linux"))]
        return None;
    }
}

/// Seconds since the last keyboard/mouse input.
pub struct Idle {
    pub name: &'static str,
    pub reliable: bool,
    #[cfg(target_os = "linux")]
    x: Option<sys::X>,
    cached: (std::time::Instant, f64),
}

impl Idle {
    /// On GNOME (Wayland included) Mutter is asked directly: X11 only sees X input.
    pub fn detect() -> Self {
        let cached = (std::time::Instant::now() - std::time::Duration::from_secs(3600), 0.0);
        #[cfg(target_os = "linux")]
        {
            if sys::mutter_idle_sec().is_some() {
                return Idle { name: "mutter", reliable: true, x: None, cached };
            }
            let x = sys::X::connect().filter(|x| x.idle_sec().is_some());
            let reliable = x.is_some() && !session().wayland;
            return Idle { name: if x.is_some() { "xscreensaver" } else { "none" }, reliable, x, cached };
        }
        #[cfg(not(target_os = "linux"))]
        Idle { name: "system", reliable: sys::idle_sec().is_some(), cached }
    }

    /// `max_age_ms` lets the per-second scheduler reuse a recent (expensive) reading.
    pub fn read(&mut self, max_age_ms: u64) -> f64 {
        if self.cached.0.elapsed().as_millis() as u64 > max_age_ms {
            #[cfg(target_os = "linux")]
            let v = match &self.x {
                Some(x) => x.idle_sec(),
                None => sys::mutter_idle_sec(),
            };
            #[cfg(not(target_os = "linux"))]
            let v = sys::idle_sec();
            self.cached = (std::time::Instant::now(), v.unwrap_or(0.0));
        }
        self.cached.1
    }
}
