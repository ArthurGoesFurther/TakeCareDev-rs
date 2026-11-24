//! Tray icon and menu. The menu is described as plain data; only the Linux backend is implemented.
use crate::sprite::{Raster, TRAY_SIZE, draw_tray_icon};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum Cmd {
    EyeNow(Option<String>),
    WalkNow,
    SkipBreak,
    /// Minutes; `None` pauses until resumed.
    Pause(Option<f64>),
    Resume,
    Reset,
    Settings,
    Quit,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Label(String),
    Sep,
    Action { label: String, enabled: bool, cmd: CmdKey },
    Sub(String, Vec<Item>),
}

/// `Cmd` without payload floats, so menus can be compared to skip needless rebuilds.
#[derive(Clone, Debug, PartialEq)]
pub enum CmdKey { EyeNow, WalkNow, SkipBreak, Pause(Option<u32>), Resume, Reset, Settings, Quit }

impl CmdKey {
    pub fn cmd(&self) -> Cmd {
        match self {
            CmdKey::EyeNow => Cmd::EyeNow(None),
            CmdKey::WalkNow => Cmd::WalkNow,
            CmdKey::SkipBreak => Cmd::SkipBreak,
            CmdKey::Pause(m) => Cmd::Pause(m.map(f64::from)),
            CmdKey::Resume => Cmd::Resume,
            CmdKey::Reset => Cmd::Reset,
            CmdKey::Settings => Cmd::Settings,
            CmdKey::Quit => Cmd::Quit,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Menu {
    pub items: Vec<Item>,
    pub sleeping: bool,
}

pub type Callback = Arc<dyn Fn(Cmd) + Send + Sync>;

#[cfg(target_os = "linux")]
pub use linux::Tray;

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use ksni::blocking::{Handle, TrayMethods};

    fn icons(sleeping: bool) -> Vec<ksni::Icon> {
        (1..=3)
            .map(|scale| {
                let mut r = Raster::new(TRAY_SIZE, TRAY_SIZE, scale);
                draw_tray_icon(&mut |x, y, c| r.put(x, y, c), sleeping, false);
                // ksni wants ARGB32 in network byte order.
                let data = r.data.chunks(4).flat_map(|p| [p[3], p[0], p[1], p[2]]).collect();
                ksni::Icon { width: r.width, height: r.height, data }
            })
            .collect()
    }

    struct Model {
        menu: Menu,
        normal: Vec<ksni::Icon>,
        sleeping: Vec<ksni::Icon>,
        on_cmd: Callback,
    }

    fn convert(items: &[Item], on_cmd: &Callback) -> Vec<ksni::MenuItem<Model>> {
        use ksni::menu::{StandardItem, SubMenu};
        items
            .iter()
            .map(|item| match item {
                Item::Label(label) => StandardItem { label: label.clone(), enabled: false, ..Default::default() }.into(),
                Item::Sep => ksni::MenuItem::Separator,
                Item::Action { label, enabled, cmd } => {
                    let (cmd, on_cmd) = (cmd.cmd(), on_cmd.clone());
                    StandardItem { label: label.clone(), enabled: *enabled, activate: Box::new(move |_: &mut Model| on_cmd(cmd.clone())), ..Default::default() }.into()
                }
                Item::Sub(label, sub) => SubMenu { label: label.clone(), submenu: convert(sub, on_cmd), ..Default::default() }.into(),
            })
            .collect()
    }

    impl ksni::Tray for Model {
        fn id(&self) -> String {
            "takecaredev".into()
        }
        fn title(&self) -> String {
            "TakeCareDev".into()
        }
        fn icon_pixmap(&self) -> Vec<ksni::Icon> {
            if self.menu.sleeping { self.sleeping.clone() } else { self.normal.clone() }
        }
        fn activate(&mut self, _x: i32, _y: i32) {
            (self.on_cmd)(Cmd::Settings);
        }
        fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
            convert(&self.menu.items, &self.on_cmd)
        }
    }

    pub struct Tray(Handle<Model>);

    impl Tray {
        pub fn start(menu: Menu, on_cmd: Callback) -> Option<Tray> {
            let model = Model { menu, normal: icons(false), sleeping: icons(true), on_cmd };
            match model.spawn() {
                Ok(handle) => Some(Tray(handle)),
                Err(e) => {
                    eprintln!("[tcdev] no tray available: {e}");
                    None
                }
            }
        }

        pub fn set(&self, menu: Menu) {
            self.0.update(|m| m.menu = menu);
        }
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub use desktop::{Tray, run_main};

/// Windows and macOS: the tray icon must be created and serviced on the main thread's event loop,
/// so the daemon runs on a second thread and talks to the loop through a proxy.
#[cfg(any(target_os = "windows", target_os = "macos"))]
mod desktop {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use tao::event::Event;
    use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
    use tray_icon::menu::{IsMenuItem, Menu as TrayMenu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
    use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

    enum Msg {
        Create(Menu, Callback),
        Set(Menu),
    }

    static PROXY: OnceLock<Mutex<EventLoopProxy<Msg>>> = OnceLock::new();
    /// Menu item ids are indexes into this list; it is rebuilt with every menu.
    static ACTIONS: Mutex<Vec<CmdKey>> = Mutex::new(Vec::new());
    static CALLBACK: OnceLock<Callback> = OnceLock::new();

    const MONO: bool = cfg!(target_os = "macos");

    fn icon(sleeping: bool) -> Icon {
        let mut r = Raster::new(TRAY_SIZE, TRAY_SIZE, 2);
        draw_tray_icon(&mut |x, y, c| r.put(x, y, c), sleeping, MONO);
        Icon::from_rgba(r.data, r.width as u32, r.height as u32).expect("tray icon dimensions match its data")
    }

    fn append(add: &dyn Fn(&dyn IsMenuItem), items: &[Item], actions: &mut Vec<CmdKey>) {
        for item in items {
            match item {
                Item::Label(label) => add(&MenuItem::new(label, false, None)),
                Item::Sep => add(&PredefinedMenuItem::separator()),
                Item::Action { label, enabled, cmd } => {
                    actions.push(cmd.clone());
                    add(&MenuItem::with_id(MenuId::new((actions.len() - 1).to_string()), label, *enabled, None));
                }
                Item::Sub(label, sub) => {
                    let submenu = Submenu::new(label, true);
                    append(&|i| drop(submenu.append(i)), sub, actions);
                    add(&submenu);
                }
            }
        }
    }

    fn build(menu: &Menu) -> TrayMenu {
        let root = TrayMenu::new();
        let mut actions = vec![];
        append(&|i| drop(root.append(i)), &menu.items, &mut actions);
        *ACTIONS.lock().expect("actions lock") = actions;
        root
    }

    fn send(msg: Msg) {
        if let Some(proxy) = PROXY.get() {
            let _ = proxy.lock().expect("proxy lock").send_event(msg);
        }
    }

    pub struct Tray;

    impl Tray {
        pub fn start(menu: Menu, on_cmd: Callback) -> Option<Tray> {
            send(Msg::Create(menu, on_cmd));
            Some(Tray)
        }

        pub fn set(&self, menu: Menu) {
            send(Msg::Set(menu));
        }
    }

    /// Runs `daemon` on a worker thread and the tray event loop on this (main) thread. Never returns.
    pub fn run_main(daemon: fn()) -> ! {
        #[allow(unused_mut)]
        let mut event_loop = EventLoopBuilder::<Msg>::with_user_event().build();
        #[cfg(target_os = "macos")]
        {
            use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
            event_loop.set_activation_policy(ActivationPolicy::Accessory);
        }
        let _ = PROXY.set(Mutex::new(event_loop.create_proxy()));
        std::thread::spawn(move || {
            daemon();
            std::process::exit(0);
        });

        MenuEvent::set_event_handler(Some(|event: MenuEvent| {
            let key = event.id.0.parse::<usize>().ok().and_then(|i| ACTIONS.lock().ok()?.get(i).cloned());
            if let (Some(key), Some(callback)) = (key, CALLBACK.get()) {
                callback(key.cmd());
            }
        }));

        let mut tray: Option<TrayIcon> = None;
        let mut sleeping = false;
        event_loop.run(move |event, _, flow| {
            *flow = ControlFlow::Wait;
            let Event::UserEvent(msg) = event else { return };
            match msg {
                Msg::Create(menu, callback) => {
                    let _ = CALLBACK.set(callback);
                    sleeping = menu.sleeping;
                    let builder = TrayIconBuilder::new().with_tooltip("TakeCareDev").with_menu(Box::new(build(&menu)));
                    // macOS wants a monochrome template image that follows the menu bar's light/dark mode.
                    #[cfg(target_os = "macos")]
                    let builder = builder.with_icon_templated(icon(sleeping));
                    #[cfg(not(target_os = "macos"))]
                    let builder = builder.with_icon(icon(sleeping));
                    // Windows: a left click opens settings (the menu is on the right click).
                    #[cfg(target_os = "windows")]
                    let builder = {
                        tray_icon::TrayIconEvent::set_event_handler(Some(|event: tray_icon::TrayIconEvent| {
                            use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
                            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                                if let Some(callback) = CALLBACK.get() {
                                    callback(Cmd::Settings);
                                }
                            }
                        }));
                        builder.with_menu_on_left_click(false)
                    };
                    match builder.build() {
                        Ok(t) => tray = Some(t),
                        Err(e) => eprintln!("[tcdev] no tray available: {e}"),
                    }
                }
                Msg::Set(menu) => {
                    let Some(t) = &tray else { return };
                    t.set_menu(Some(Box::new(build(&menu))));
                    if menu.sleeping != sleeping {
                        sleeping = menu.sleeping;
                        #[cfg(target_os = "macos")]
                        let _ = t.set_icon_templated(Some(icon(sleeping)));
                        #[cfg(not(target_os = "macos"))]
                        let _ = t.set_icon(Some(icon(sleeping)));
                    }
                }
            }
        })
    }
}
