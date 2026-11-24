# TakeCareDev

[English](README.md) · [Русский](README.ru.md) · **中文**

一个像素小脸，提醒你做眼保健操、起身走动。常驻系统托盘（macOS 上在菜单栏），支持 Windows、macOS 和 Linux。

这是一个用 Rust 写的小巧原生应用：内部没有浏览器引擎，下载约 5 MB，等待时几乎不占用 CPU。

![icon](assets/icon.png)

**下载：** [最新版本](https://github.com/ArthurGoesFurther/takecaredev-rs/releases/latest)

| 系统                          | 文件                               |
| ----------------------------- | ---------------------------------- |
| macOS（Apple Silicon，M1–M4） | `takecaredev-macos-arm64.zip`      |
| Windows                       | `takecaredev-windows-x86_64.zip`   |
| Linux                         | `takecaredev-linux-x86_64.zip`     |

## 功能

- **眼保健操。** 每隔 N 分钟（默认 20 分钟，即 20-20-20 法则），一张像素小脸会出现在鼠标旁边，
  用 N 秒（默认 10 秒）演示一个动作：左右看、上下看、转圈、对角线、画"8"字、眨眼、紧闭双眼、
  远眺、远近交替、闭眼休息。你只要跟着它的瞳孔转动就行。窗口透明、点击穿透、不抢焦点，不会打断你打字。
- **起身走走。** 每隔 N 分钟（默认 50 分钟）休息 N 分钟（默认 5 分钟）：一张带计时器和小提示
  （深蹲、爬楼梯、拉伸、喝水）的卡片，可以"稍后提醒"或"跳过"。
- **严格模式。** 休息画面覆盖屏幕并隐藏按钮。紧急退出在托盘菜单里。
- **诚实计时。** 默认情况下，只有在你不碰鼠标和键盘时休息计时才会走——坐在休息画面前干等不算数。
- **自然休息。** 如果你离开超过 N 分钟（或笔记本休眠了），倒计时会重新开始，不会你一坐下就催你。
- 休息前 N 秒发送通知。
- 暂停 30 分钟 / 1 小时 / 2 小时 / 直到手动恢复，每日统计，开机自启。
- 可以选择要做的动作、顺序、小脸大小、显示在屏幕角落而不是鼠标旁、提示气泡和声音。
- 界面语言：英语和俄语。

所有间隔和时长都可以调整：托盘 →"设置…"，或者再次启动应用。

### 首次启动

- **macOS：** 解压后把 `TakeCareDev.app` 拖进"应用程序"。应用没有 Apple 开发者签名，所以第一次启动会被拦截：
  右键点击应用 →**打开**→**打开**（或"系统设置"→"隐私与安全性"→**仍要打开**）。之后就能正常启动。
  它只在菜单栏显示，程序坞里没有图标。
- **Windows：** 解压后运行 `takecaredev.exe`。程序未签名，SmartScreen 会弹出警告——点"更多信息"→"仍要运行"。
- **Linux：** 解压，执行 `chmod +x takecaredev`，然后运行。GNOME 上的托盘图标需要 AppIndicator 扩展（Ubuntu 默认启用）。

## 为什么要做这个

程序员每天坐 8–12 个小时，盯着半米外的屏幕。有两样东西在悄悄被损耗：眼睛，以及腰部以下。

### 视力下降

- **近视已成流行病。** 2020 年全球约 30% 的人近视；预计到 2050 年将达到约 50%，接近 50 亿人。
  长时间近距离用眼、很少远眺，是最主要的风险因素之一。
- **调节痉挛。** 为了对焦 50 厘米处，睫状肌要连续紧绷好几个小时。漫长的一天过后，看远处发虚、
  眼睛酸痛、头痛——这就是"视频终端综合征"。
- **干眼。** 盯着屏幕时，我们眨眼的次数比平时少 3–4 倍（每分钟约 15–20 次降到约 5 次）。
  泪膜蒸发，导致灼烧感、发红、眼里像进了沙子。
- **有效的做法：** 眼科医生推荐的 20-20-20 法则——每 20 分钟看向约 6 米（20 英尺）外的物体
  20 秒——再加上有意识地眨眼和切换焦距。小脸做的正是这些。

实话实说：眼保健操无法逆转已有的近视。它能缓解疲劳和干涩，并帮你养成移开视线的习惯——这是你能掌控的部分。

### 痔疮与久坐

- **压力。** 坐着时，肛管静脉持续受压，盆腔血液回流变慢。静脉被撑开，肛垫下移——这就是痔病。
- **肠道不动。** 活动少，肠蠕动就慢，导致便秘和用力排便——这是痔疮最主要的诱因。
  蹲厕所时长时间刷手机也一样。
- **非常普遍。** 根据研究方法不同，成年人中痔病的患病率约在 4% 到 40% 之间；只是大多数人不到疼的时候都不愿提起。
- **有效的做法：** 每 45–60 分钟起身走几分钟。走路能恢复静脉回流、促进肠道蠕动。
  休息提示里的深蹲和爬楼梯效果更好。

久坐还与腰背和颈部疼痛、下肢血液循环变差以及心血管风险升高有关，所以起身走走的回报远不止一处。

TakeCareDev 不能替代医生。如果已经疼了或视力已经下降——请去看眼科 / 肛肠科医生。

## 工作原理

窗口用 [egui](https://egui.rs) 绘制；托盘在 Linux 上通过 D-Bus 使用 StatusNotifierItem
（[ksni](https://crates.io/crates/ksni)），在 Windows 和 macOS 上使用 [tray-icon](https://crates.io/crates/tray-icon)。
小脸和其他所有图像都用代码绘制——没有图片资源。

一个可执行文件，四种角色：

| 命令                      | 作用                                                   |
| ------------------------- | ------------------------------------------------------ |
| `takecaredev`             | 守护进程：托盘、计时、设置、统计；负责启动下面这些窗口 |
| `takecaredev face <json>` | 弹出的像素小脸（透明、点击穿透、不抢焦点）             |
| `takecaredev break ...`   | 休息卡片，严格模式下为全屏遮罩                         |
| `takecaredev settings`    | 设置窗口                                               |

每个窗口都是独立进程，因为 egui/winit 的事件循环在一个进程里只能运行一次。它们通过 stdin/stdout
以 JSON 行与守护进程通信（`src/ipc.rs`）。在 Windows 和 macOS 上，托盘占用主线程，守护进程运行在工作线程中。

```
src/
  main.rs         入口：选择角色
  daemon.rs       组装：事件、托盘菜单、小脸和休息的生命周期
  scheduler.rs    纯计时逻辑，有测试覆盖（时钟作为参数传入）
  settings.rs     设置结构、默认值、取值范围
  exercises.rs    动作定义：小脸每一秒在做什么
  sprite.rs       小脸，用代码绘制到任意像素缓冲区
  i18n.rs         翻译（ru、en）
  store.rs        settings.json 和 stats.json
  platform.rs     会话类型、鼠标、显示器、空闲时间（X11 / Mutter / Windows / macOS）
  sound.rs        提示音：渲染成 WAV，用 afplay / paplay / pw-play / aplay 播放
  png.rs          生成应用图标用的迷你 PNG 编码器
  tray.rs, notify.rs, autostart.rs, ipc.rs
  ui/             face.rs, break_screen.rs, settings.rs
scripts/
  macos-app.sh    构建 TakeCareDev.app（带图标，ad-hoc 签名）
```

设置和统计数据保存在 `~/Library/Application Support/TakeCareDev/`（macOS）、`%APPDATA%\TakeCareDev\`（Windows）
和 `~/.config/TakeCareDev/`（Linux）。文件格式与之前的 Electron 版 TakeCareDev 相同，所以设置可以直接沿用。

### 构建

需要 [Rust 工具链](https://rustup.rs)。

```bash
cargo run --release                          # 托盘守护进程；首次启动会打开设置窗口
cargo test                                   # 调度器、设置、i18n、精灵图、存储
TCDEV_DATA_DIR=/tmp/tc cargo run --release   # 使用独立的设置目录
scripts/macos-app.sh /Applications           # macOS：构建并安装 TakeCareDev.app
```

推送 `v*` 标签后，GitHub Actions 会构建三个平台并发布版本。

### 平台说明

- **macOS：** 已在 Apple Silicon 上测试。鼠标位置来自 CoreGraphics，空闲时间来自 `IOHIDSystem`。
  目前还不枚举显示器：休息卡片在它打开的那个屏幕上居中，严格模式也只覆盖这一个屏幕。
- **Linux（GNOME，Wayland/XWayland）：** 已测试。窗口在 XWayland 下运行，这样才能按坐标放置窗口并找到鼠标
  （屏幕会被一个透明的探测窗口短暂覆盖）。空闲时间来自 Mutter（通过 D-Bus 调用 `org.gnome.Mutter.IdleMonitor`）；
  在其他 Wayland 合成器上，依赖空闲时间的功能会被关闭。在 Wayland 下小脸只能出现在鼠标旁边，无法跟随。
- **Windows：** 托盘、鼠标、显示器、空闲时间、开机自启和通知都已实现，但还没有在真机上测试过——欢迎反馈。
- 同一时间只运行一个实例（用固定的本地端口 `127.0.0.1:47821` 作锁）；再次启动会打开正在运行实例的设置。

## 参考资料

- Holden B.A. et al., _Global Prevalence of Myopia and High Myopia and Temporal Trends from 2000
  through 2050_, Ophthalmology, 2016.
- American Academy of Ophthalmology, _Computers, Digital Devices and Eye Strain_（20-20-20 法则）。
- Patel S. et al., _Effect of visual display unit use on blink rate and tear stability_, Optometry
  and Vision Science, 1991.
- Johanson J.F., Sonnenberg A., _The prevalence of hemorrhoids and chronic constipation_,
  Gastroenterology, 1990.
- Riss S. et al., _The prevalence of hemorrhoids in adults_, International Journal of Colorectal
  Disease, 2012.
- Lohsiriwat V., _Hemorrhoids: from basic pathophysiology to clinical management_, World Journal of
  Gastroenterology, 2012.
