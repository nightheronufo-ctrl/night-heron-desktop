// 夜鹭骑行 · 桌面小窗版
// © 2026 不明飞行鹭 · CC BY-NC-ND 4.0
//
// 外壳只负责窗口：无边框圆角小窗、悬停拖动条（见 shell.js）、置顶、托盘、
// 贴边吸附、记住位置大小。游戏本体 index.html 原样打包，不做修改。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Size, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, WindowEvent, Wry,
};

const SHELL_JS: &str = include_str!("shell.js");
const DEFAULT_W: f64 = 480.0; // 逻辑像素，16:10
const DEFAULT_H: f64 = 300.0;
const MIN_W: f64 = 320.0;
const MIN_H: f64 = 200.0;
const GAP: f64 = 12.0; // 吸附后离屏幕边缘的距离
const SNAP: f64 = 40.0; // 离边缘多近时吸过去
const HOME_GAP: f64 = 16.0; // 默认右下角的边距

// ---------- 设置（位置、大小、置顶、语言） ----------

#[derive(Serialize, Deserialize, Clone, Default)]
struct Prefs {
    x: Option<i32>,
    y: Option<i32>,
    w: Option<u32>,
    h: Option<u32>,
    #[serde(default)]
    pin: bool,
    #[serde(default)]
    lang: String,
}

struct Shell {
    prefs: Mutex<Prefs>,
    path: PathBuf,
    last_move: Mutex<Option<Instant>>,
    animating: AtomicBool,
    tray: Mutex<Option<TrayItems>>,
}

struct TrayItems {
    show: MenuItem<Wry>,
    pin: CheckMenuItem<Wry>,
    full: MenuItem<Wry>,
    home: MenuItem<Wry>,
    quit: MenuItem<Wry>,
}

impl Shell {
    fn save(&self) {
        if let Ok(p) = self.prefs.lock() {
            if let Some(dir) = self.path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let _ = fs::write(&self.path, serde_json::to_string_pretty(&*p).unwrap_or_default());
        }
    }
    fn en(&self) -> bool {
        self.prefs.lock().map(|p| p.lang == "en").unwrap_or(false)
    }
}

fn load_prefs(path: &PathBuf) -> Prefs {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

// ---------- 鼠标左键是否按着（拖动结束后才吸附） ----------

#[cfg(target_os = "windows")]
fn mouse_down() -> bool {
    #[link(name = "user32")]
    extern "system" {
        fn GetAsyncKeyState(v: i32) -> i16;
    }
    unsafe { (GetAsyncKeyState(0x01) as u16 & 0x8000) != 0 }
}

#[cfg(target_os = "macos")]
fn mouse_down() -> bool {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventSourceButtonState(state: i32, button: u32) -> bool;
    }
    // kCGEventSourceStateHIDSystemState = 1, kCGMouseButtonLeft = 0
    unsafe { CGEventSourceButtonState(1, 0) }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn mouse_down() -> bool {
    false
}

// ---------- 位置计算 ----------

struct Area {
    l: f64,
    t: f64,
    r: f64,
    b: f64,
    scale: f64,
}

fn area_of(m: &tauri::Monitor) -> Area {
    let wa = m.work_area();
    let l = wa.position.x as f64;
    let t = wa.position.y as f64;
    Area { l, t, r: l + wa.size.width as f64, b: t + wa.size.height as f64, scale: m.scale_factor() }
}

fn window_area(win: &WebviewWindow) -> Option<Area> {
    let m = win.current_monitor().ok().flatten().or_else(|| win.primary_monitor().ok().flatten())?;
    Some(area_of(&m))
}

/// 默认位置：主屏工作区右下角
fn home_rect(win: &WebviewWindow) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>)> {
    let m = win.primary_monitor().ok().flatten()?;
    let a = area_of(&m);
    let w = (DEFAULT_W * a.scale).round();
    let h = (DEFAULT_H * a.scale).round();
    let g = HOME_GAP * a.scale;
    Some((
        PhysicalPosition::new((a.r - w - g) as i32, (a.b - h - g) as i32),
        PhysicalSize::new(w as u32, h as u32),
    ))
}

/// 保存的位置是否还在某块屏幕上（换了显示器时回到默认位置）
fn on_some_screen(win: &WebviewWindow, x: i32, y: i32, w: u32, h: u32) -> bool {
    let ms = win.available_monitors().unwrap_or_default();
    ms.iter().any(|m| {
        let a = area_of(m);
        let (x, y, w, h) = (x as f64, y as f64, w as f64, h as f64);
        // 至少有 80×40 的区域露在这块屏幕内
        let ix = (x + w).min(a.r) - x.max(a.l);
        let iy = (y + h).min(a.b) - y.max(a.t);
        ix >= 80.0 * a.scale && iy >= 40.0 * a.scale
    })
}

fn move_to(win: &WebviewWindow, from: (f64, f64), to: (f64, f64)) {
    let st = win.state::<Shell>();
    st.animating.store(true, Ordering::SeqCst);
    // 6 帧 ease-out 小动画，像被磁铁吸过去
    let n = 6;
    for i in 1..=n {
        let k = i as f64 / n as f64;
        let e = 1.0 - (1.0 - k).powi(3);
        let x = from.0 + (to.0 - from.0) * e;
        let y = from.1 + (to.1 - from.1) * e;
        let _ = win.set_position(Position::Physical(PhysicalPosition::new(x.round() as i32, y.round() as i32)));
        std::thread::sleep(Duration::from_millis(14));
    }
    std::thread::sleep(Duration::from_millis(60));
    st.animating.store(false, Ordering::SeqCst);
}

/// 贴边吸附：松手时离工作区边缘不到 SNAP 就吸过去，停在离边缘 GAP 的位置
fn snap(win: &WebviewWindow) {
    if win.is_fullscreen().unwrap_or(false) || win.is_maximized().unwrap_or(false) || win.is_minimized().unwrap_or(false) {
        return;
    }
    let (Ok(pos), Ok(size)) = (win.outer_position(), win.outer_size()) else { return };
    let Some(a) = window_area(win) else { return };
    let (w, h) = (size.width as f64, size.height as f64);
    let (gap, th) = (GAP * a.scale, SNAP * a.scale);
    let (x0, y0) = (pos.x as f64, pos.y as f64);
    let (mut x, mut y) = (x0, y0);
    if w + 2.0 * gap <= a.r - a.l {
        if x - a.l < th {
            x = a.l + gap;
        } else if a.r - (x + w) < th {
            x = a.r - w - gap;
        }
    }
    if h + 2.0 * gap <= a.b - a.t {
        if y - a.t < th {
            y = a.t + gap;
        } else if a.b - (y + h) < th {
            y = a.b - h - gap;
        }
    }
    if (x - x0).abs() >= 1.0 || (y - y0).abs() >= 1.0 {
        move_to(win, (x0, y0), (x, y));
    }
}

fn remember(win: &WebviewWindow) {
    if win.is_fullscreen().unwrap_or(false) || win.is_minimized().unwrap_or(false) || win.is_maximized().unwrap_or(false) {
        return;
    }
    let st = win.state::<Shell>();
    if let (Ok(p), Ok(s)) = (win.outer_position(), win.inner_size()) {
        if let Ok(mut pr) = st.prefs.lock() {
            pr.x = Some(p.x);
            pr.y = Some(p.y);
            pr.w = Some(s.width);
            pr.h = Some(s.height);
        }
        st.save();
    }
}

// ---------- 状态同步到网页和托盘 ----------

fn broadcast(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else { return };
    let st = app.state::<Shell>();
    let pin = st.prefs.lock().map(|p| p.pin).unwrap_or(false);
    let full = win.is_fullscreen().unwrap_or(false);
    let visible = win.is_visible().unwrap_or(true);
    let _ = app.emit("nh-state", serde_json::json!({ "pin": pin, "full": full }));
    if let Ok(t) = st.tray.lock() {
        if let Some(t) = t.as_ref() {
            let en = st.en();
            let _ = t.pin.set_checked(pin);
            let _ = t.pin.set_text(if en { "Always on top" } else { "一直置顶" });
            let _ = t.show.set_text(match (visible, en) {
                (true, false) => "隐藏窗口",
                (false, false) => "显示窗口",
                (true, true) => "Hide window",
                (false, true) => "Show window",
            });
            let _ = t.full.set_text(match (full, en) {
                (false, false) => "全屏",
                (true, false) => "回到小窗",
                (false, true) => "Full screen",
                (true, true) => "Back to small window",
            });
            let _ = t.home.set_text(if en { "Move to bottom-right corner" } else { "回到右下角" });
            let _ = t.quit.set_text(if en { "Quit" } else { "退出" });
        }
    };
}

fn set_pin(app: &AppHandle, on: bool) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_always_on_top(on);
    }
    let st = app.state::<Shell>();
    if let Ok(mut p) = st.prefs.lock() {
        p.pin = on;
    }
    st.save();
    broadcast(app);
}

fn toggle_visible(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) && !win.is_minimized().unwrap_or(false) {
            let _ = win.hide();
        } else {
            let _ = win.unminimize();
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
    broadcast(app);
}

fn set_full(app: &AppHandle, on: bool) {
    let Some(win) = app.get_webview_window("main") else { return };
    if on {
        remember(&win);
    }
    let _ = win.set_fullscreen(on);
    if !on {
        // 退出全屏后回到记住的小窗位置
        let st = app.state::<Shell>();
        let p = st.prefs.lock().map(|p| p.clone()).unwrap_or_default();
        if let (Some(x), Some(y), Some(w), Some(h)) = (p.x, p.y, p.w, p.h) {
            let _ = win.set_size(Size::Physical(PhysicalSize::new(w, h)));
            let _ = win.set_position(Position::Physical(PhysicalPosition::new(x, y)));
        }
    }
    let _ = win.show();
    broadcast(app);
}

fn go_home(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else { return };
    if win.is_fullscreen().unwrap_or(false) {
        let _ = win.set_fullscreen(false);
    }
    if let Some((p, s)) = home_rect(&win) {
        let _ = win.set_size(Size::Physical(s));
        let _ = win.set_position(Position::Physical(p));
    }
    let _ = win.unminimize();
    let _ = win.show();
    remember(&win);
    broadcast(app);
}

fn quit(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        remember(&win);
    }
    app.exit(0);
}

// ---------- 给悬停拖动条用的命令 ----------

#[tauri::command]
fn nh_drag(win: WebviewWindow) {
    if !win.is_fullscreen().unwrap_or(false) {
        let _ = win.start_dragging();
    }
}

#[tauri::command]
fn nh_minimize(win: WebviewWindow) {
    let _ = win.minimize();
}

#[tauri::command]
fn nh_close(app: AppHandle) {
    quit(&app);
}

#[tauri::command]
fn nh_toggle_pin(app: AppHandle) {
    let on = app.state::<Shell>().prefs.lock().map(|p| !p.pin).unwrap_or(false);
    set_pin(&app, on);
}

#[tauri::command]
fn nh_toggle_full(win: WebviewWindow, app: AppHandle) {
    let full = win.is_fullscreen().unwrap_or(false);
    set_full(&app, !full);
}

#[tauri::command]
fn nh_set_lang(app: AppHandle, lang: String) {
    let st = app.state::<Shell>();
    if let Ok(mut p) = st.prefs.lock() {
        p.lang = if lang == "en" { "en".into() } else { "zh".into() };
    }
    st.save();
    broadcast(&app);
}

#[tauri::command]
fn nh_state(app: AppHandle) -> serde_json::Value {
    let pin = app.state::<Shell>().prefs.lock().map(|p| p.pin).unwrap_or(false);
    let full = app.get_webview_window("main").map(|w| w.is_fullscreen().unwrap_or(false)).unwrap_or(false);
    serde_json::json!({ "pin": pin, "full": full })
}

// ---------- 启动 ----------

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "隐藏窗口", true, None::<&str>)?;
    let pin = CheckMenuItem::with_id(app, "pin", "一直置顶", true, false, None::<&str>)?;
    let full = MenuItem::with_id(app, "full", "全屏", true, None::<&str>)?;
    let home = MenuItem::with_id(app, "home", "回到右下角", true, None::<&str>)?;
    let about = MenuItem::with_id(app, "about", "© 2026 不明飞行鹭 · CC BY-NC-ND 4.0", false, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &pin, &full, &home, &sep1, &about, &sep2, &quit_i])?;

    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
    TrayIconBuilder::with_id("nh")
        .icon(icon)
        .tooltip("夜鹭骑行")
        .menu(&menu)
        // Windows：左键点托盘图标直接显示/隐藏，右键出菜单；Mac 菜单栏习惯点一下出菜单
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_menu_event(|app, e| match e.id().as_ref() {
            "show" => toggle_visible(app),
            "pin" => {
                let on = app.state::<Shell>().prefs.lock().map(|p| !p.pin).unwrap_or(false);
                set_pin(app, on)
            }
            "full" => {
                let full = app.get_webview_window("main").map(|w| w.is_fullscreen().unwrap_or(false)).unwrap_or(false);
                set_full(app, !full)
            }
            "home" => go_home(app),
            "quit" => quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                if !cfg!(target_os = "macos") {
                    toggle_visible(tray.app_handle());
                }
            }
        })
        .build(app)?;

    *app.state::<Shell>().tray.lock().unwrap() = Some(TrayItems { show, pin, full, home, quit: quit_i });
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 再次打开时，把已有的小窗叫出来
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .invoke_handler(tauri::generate_handler![
            nh_drag,
            nh_minimize,
            nh_close,
            nh_toggle_pin,
            nh_toggle_full,
            nh_set_lang,
            nh_state
        ])
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            let path = dir.join("desktop-window.json");
            let prefs = load_prefs(&path);
            app.manage(Shell {
                prefs: Mutex::new(prefs.clone()),
                path,
                last_move: Mutex::new(None),
                animating: AtomicBool::new(false),
                tray: Mutex::new(None),
            });

            #[allow(unused_mut)]
            let mut b = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("夜鹭骑行")
                .inner_size(DEFAULT_W, DEFAULT_H)
                .min_inner_size(MIN_W, MIN_H)
                .decorations(false)
                .resizable(true)
                .shadow(true)
                .visible(false)
                .always_on_top(prefs.pin)
                .initialization_script(SHELL_JS);
            #[cfg(target_os = "macos")]
            {
                // Mac：透明窗口 + 网页里裁出圆角，系统按圆角形状画投影
                b = b.transparent(true);
            }
            #[cfg(target_os = "windows")]
            {
                // Windows 11 对无边框 + shadow 的窗口自动给圆角和投影；先用游戏底色避免白闪
                b = b.background_color(tauri::window::Color(22, 36, 58, 255));
            }
            let win = b.build()?;

            match (prefs.x, prefs.y, prefs.w, prefs.h) {
                (Some(x), Some(y), Some(w), Some(h)) if on_some_screen(&win, x, y, w, h) => {
                    let _ = win.set_size(Size::Physical(PhysicalSize::new(w, h)));
                    let _ = win.set_position(Position::Physical(PhysicalPosition::new(x, y)));
                }
                _ => {
                    if let Some((p, s)) = home_rect(&win) {
                        let _ = win.set_size(Size::Physical(s));
                        let _ = win.set_position(Position::Physical(p));
                    }
                }
            }
            win.show()?;

            build_tray(app.handle())?;
            broadcast(app.handle());

            // 移动/缩放停下且松开鼠标后：贴边吸附，然后记住位置
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_millis(80));
                let Some(win) = handle.get_webview_window("main") else { continue };
                let st = handle.state::<Shell>();
                let due = st
                    .last_move
                    .lock()
                    .map(|t| t.map(|t| t.elapsed() > Duration::from_millis(220)).unwrap_or(false))
                    .unwrap_or(false);
                if !due || mouse_down() {
                    continue;
                }
                *st.last_move.lock().unwrap() = None;
                snap(&win);
                remember(&win);
            });
            Ok(())
        })
        .on_window_event(|win, e| match e {
            WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                let st = win.state::<Shell>();
                if !st.animating.load(Ordering::SeqCst) {
                    *st.last_move.lock().unwrap() = Some(Instant::now());
                }
                if matches!(e, WindowEvent::Resized(_)) {
                    broadcast(win.app_handle());
                }
            }
            WindowEvent::CloseRequested { .. } => {
                if let Some(w) = win.app_handle().get_webview_window("main") {
                    remember(&w);
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running 夜鹭骑行");
}
