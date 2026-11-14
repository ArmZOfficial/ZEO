//! ZEO Milestone 0: a frameless window with two WebView2 views sharing one environment:
//! the browser UI (HTML from `ui/`, served at https://zeo.ui/) on top, and one tab view below it.
//! Every request in the tab view is checked by ZEO Shields.
#![windows_subsystem = "windows"]

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashSet;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock, mpsc};
use std::time::Instant;

use serde_json::{Value, json};
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;
use windows::core::{Error, Result};
use zeo_shields::{Category, ResourceKind, Shields};

const HOME: &str = "https://duckduckgo.com/";
const UI_HOST: &str = "zeo.ui";
const UI_ORIGIN: &str = "https://zeo.ui/";
/// Height of the UI rows above the page in CSS px: --row-top + --row-tabs (bookmark bar hidden).
const UI_BAR_CSS: i32 = 70;
const TIMER_PUSH_SHIELDS: usize = 1;
const BG: COREWEBVIEW2_COLOR = COREWEBVIEW2_COLOR { A: 255, R: 10, G: 10, B: 11 };

#[derive(Clone)]
struct View {
    controller: ICoreWebView2Controller,
    webview: ICoreWebView2,
}

#[derive(Clone, Copy, PartialEq)]
enum Level {
    Standard,
    Strict,
    Off,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Standard => "standard",
            Level::Strict => "strict",
            Level::Off => "off",
        }
    }
}

thread_local! {
    static MAIN: Cell<HWND> = Cell::new(HWND::default());
    static ENV: OnceCell<ICoreWebView2Environment> = const { OnceCell::new() };
    static TAB: OnceCell<View> = const { OnceCell::new() };
    static UI: OnceCell<View> = const { OnceCell::new() };
    static SHIELDS: OnceCell<Shields> = const { OnceCell::new() };
    static ADS: Cell<u32> = const { Cell::new(0) };
    static TRACKERS: Cell<u32> = const { Cell::new(0) };
    static PAGE_URL: RefCell<String> = const { RefCell::new(String::new()) };
    static DISABLED_HOSTS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static LEVEL: Cell<Level> = const { Cell::new(Level::Standard) };
    static LOADING: Cell<bool> = const { Cell::new(false) };
    /// Bottom of the open popover in CSS px; the UI view grows to cover it.
    static POPOVER_BOTTOM: Cell<Option<i32>> = const { Cell::new(None) };
    static PUSH_PENDING: Cell<bool> = const { Cell::new(false) };
    /// Host HWND of the UI view, kept above the tab view so popovers aren't hidden by the page.
    static UI_HOST_HWND: Cell<HWND> = Cell::new(HWND::default());
}

static LOG: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();

/// Logging is off unless ZEO_LOG points at a file. Used to verify blocking without a console.
fn log(line: &str) {
    let log = LOG.get_or_init(|| {
        let path = std::env::var_os("ZEO_LOG")?;
        std::fs::OpenOptions::new().create(true).append(true).open(path).ok().map(Mutex::new)
    });
    if let Some(f) = log
        && let Ok(mut f) = f.lock()
    {
        let _ = writeln!(f, "{line}");
    }
}

fn tab() -> Option<View> {
    TAB.with(|t| t.get().cloned())
}

fn ui_view() -> Option<View> {
    UI.with(|t| t.get().cloned())
}

fn main_hwnd() -> HWND {
    MAIN.with(Cell::get)
}

fn take(f: impl FnOnce(&mut PWSTR) -> Result<()>) -> String {
    let mut p = PWSTR::null();
    match f(&mut p) {
        Ok(()) => take_pwstr(p),
        Err(_) => String::new(),
    }
}

/// Turn omnibox input into a URL: explicit schemes pass through, things that look like hosts get
/// https://, everything else is a DuckDuckGo search.
fn to_url(input: &str) -> String {
    let s = input.trim();
    if s.contains("://") || s.starts_with("about:") {
        return s.to_string();
    }
    let looks_like_host =
        !s.contains(char::is_whitespace) && (s.contains('.') || s.starts_with("localhost"));
    if looks_like_host {
        return format!("https://{s}");
    }
    let mut q = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => q.push(b as char),
            b' ' => q.push('+'),
            _ => q.push_str(&format!("%{b:02X}")),
        }
    }
    format!("https://duckduckgo.com/?q={q}")
}

/// Host part of a URL, lowercase, without port or credentials. Empty if there is none.
fn host_of(url: &str) -> String {
    let Some((_, rest)) = url.split_once("://") else { return String::new() };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority.rsplit('@').next().unwrap_or("");
    let host = if host.starts_with('[') {
        host.split(']').next().map(|h| format!("{h}]")).unwrap_or_default()
    } else {
        host.split(':').next().unwrap_or("").to_string()
    };
    host.trim_start_matches("www.").to_ascii_lowercase()
}

fn kind_of(ctx: COREWEBVIEW2_WEB_RESOURCE_CONTEXT) -> ResourceKind {
    match ctx {
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT => ResourceKind::Document,
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_SCRIPT => ResourceKind::Script,
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_STYLESHEET => ResourceKind::Stylesheet,
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_IMAGE => ResourceKind::Image,
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FONT => ResourceKind::Font,
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_MEDIA => ResourceKind::Media,
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_XML_HTTP_REQUEST
        | COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FETCH
        | COREWEBVIEW2_WEB_RESOURCE_CONTEXT_EVENT_SOURCE => ResourceKind::XmlHttpRequest,
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_WEBSOCKET => ResourceKind::WebSocket,
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_PING => ResourceKind::Ping,
        _ => ResourceKind::Other,
    }
}

// ---------- Rust -> UI ----------

fn push(kind: &str, payload: Value) {
    let Some(ui) = ui_view() else { return };
    let msg = json!({ "type": kind, "payload": payload }).to_string();
    let js = HSTRING::from(format!("window.zen && window.zen.receive({msg})"));
    unsafe {
        let _ = ui.webview.ExecuteScript(
            &js,
            &ExecuteScriptCompletedHandler::create(Box::new(|_, _| Ok(()))),
        );
    }
}

fn shields_on_for(host: &str) -> bool {
    LEVEL.with(Cell::get) != Level::Off && !DISABLED_HOSTS.with(|d| d.borrow().contains(host))
}

fn push_shields() {
    let host = PAGE_URL.with(|p| host_of(&p.borrow()));
    push(
        "state:shields",
        json!({
            "host": host,
            "enabled": !DISABLED_HOSTS.with(|d| d.borrow().contains(&host)),
            "level": LEVEL.with(Cell::get).as_str(),
            "ads": ADS.with(Cell::get),
            "trackers": TRACKERS.with(Cell::get),
            // Not measured yet: blocked requests never download, so their size is unknown.
            "savedMB": 0,
        }),
    );
}

/// Blocking can fire hundreds of times per page; batch UI updates on a short timer.
fn schedule_shields_push() {
    if !PUSH_PENDING.with(|p| p.replace(true)) {
        unsafe {
            SetTimer(Some(main_hwnd()), TIMER_PUSH_SHIELDS, 150, None);
        }
    }
}

fn push_nav() {
    let Some(t) = tab() else { return };
    let url = take(|p| unsafe { t.webview.Source(p) });
    let (mut back, mut fwd) = (BOOL(0), BOOL(0));
    unsafe {
        let _ = t.webview.CanGoBack(&mut back);
        let _ = t.webview.CanGoForward(&mut fwd);
    }
    push(
        "state:nav",
        json!({
            "url": url,
            "host": host_of(&url),
            "secure": url.starts_with("https://"),
            "canBack": back.as_bool(),
            "canForward": fwd.as_bool(),
            "loading": LOADING.with(Cell::get),
            "bookmarked": false,
        }),
    );
}

fn push_tabs() {
    let Some(t) = tab() else { return };
    let url = take(|p| unsafe { t.webview.Source(p) });
    let title = take(|p| unsafe { t.webview.DocumentTitle(p) });
    let title = if title.is_empty() { "แท็บใหม่".to_string() } else { title };
    push("state:tabs", json!({ "tabs": [{ "id": "t1", "title": title, "host": host_of(&url), "active": true }] }));
}

fn push_zoom() {
    let Some(t) = tab() else { return };
    let mut z = 1.0;
    unsafe {
        let _ = t.controller.ZoomFactor(&mut z);
    }
    push("state:zoom", json!({ "percent": (z * 100.0).round() as i64 }));
}

fn push_all() {
    let name = std::env::var("USERNAME").unwrap_or_else(|_| "ZEO".into());
    push("state:profile", json!({ "name": name, "label": format!("{name} · โปรไฟล์ในเครื่อง") }));
    push("ui:bookmark-bar", json!({ "visible": false }));
    push("state:bookmarks", json!({ "bar": [], "folders": [{ "id": "bar", "name": "แถบบุ๊กมาร์ก" }] }));
    push_tabs();
    push_nav();
    push_shields();
    push_zoom();
}

// ---------- UI -> Rust ----------

fn navigate(input: &str) {
    let url = to_url(input);
    if let Some(t) = tab() {
        let h = HSTRING::from(url.as_str());
        unsafe {
            let _ = t.webview.Navigate(&h);
            let _ = t.controller.MoveFocus(COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC);
        }
    }
}

fn reload_tab() {
    if let Some(t) = tab() {
        unsafe {
            let _ = t.webview.Reload();
        }
    }
}

fn handle_ui_message(kind: &str, p: &Value) {
    let hwnd = main_hwnd();
    let s = |k: &str| p.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let t = tab();
    unsafe {
        match kind {
            "ui:ready" => push_all(),
            "nav:back" => {
                if let Some(t) = t {
                    let _ = t.webview.GoBack();
                }
            }
            "nav:forward" => {
                if let Some(t) = t {
                    let _ = t.webview.GoForward();
                }
            }
            "nav:reload" => reload_tab(),
            "nav:go" => navigate(&s("input")),
            "tab:new" => navigate(HOME),
            "tab:close" | "window:close" => {
                let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
            "window:minimize" => {
                let _ = ShowWindow(hwnd, SW_MINIMIZE);
            }
            "window:maximize" => {
                let _ = ShowWindow(hwnd, if IsZoomed(hwnd).as_bool() { SW_RESTORE } else { SW_MAXIMIZE });
            }
            "ui:popover" => {
                let open = p.get("open").and_then(Value::as_bool).unwrap_or(false);
                let bottom = p.pointer("/rect/bottom").and_then(Value::as_f64).map(|b| b.ceil() as i32);
                POPOVER_BOTTOM.with(|c| c.set(if open { bottom } else { None }));
                layout(hwnd);
            }
            "shields:set-site" => {
                let host = s("host");
                let enabled = p.get("enabled").and_then(Value::as_bool).unwrap_or(true);
                DISABLED_HOSTS.with(|d| {
                    let mut d = d.borrow_mut();
                    if enabled { d.remove(&host) } else { d.insert(host) }
                });
                reload_tab();
            }
            "shields:set-level" => {
                let level = match s("level").as_str() {
                    "off" => Level::Off,
                    "strict" => Level::Strict,
                    _ => Level::Standard,
                };
                LEVEL.with(|l| l.set(level));
                reload_tab();
            }
            "shields:report-broken" => {
                DISABLED_HOSTS.with(|d| d.borrow_mut().insert(s("host")));
                reload_tab();
            }
            "page:zoom" => {
                if let Some(t) = t {
                    let delta = p.get("delta").and_then(Value::as_f64).unwrap_or(0.0) / 100.0;
                    let mut z = 1.0;
                    let _ = t.controller.ZoomFactor(&mut z);
                    let _ = t.controller.SetZoomFactor((z + delta).clamp(0.3, 3.0));
                    push_zoom();
                }
            }
            other => log(&format!("UNHANDLED {other} {p}")),
        }
    }
}

// ---------- window ----------

fn direct_children(parent: HWND) -> Vec<HWND> {
    let mut out = Vec::new();
    unsafe {
        let mut h = GetWindow(parent, GW_CHILD).ok();
        while let Some(c) = h {
            out.push(c);
            h = GetWindow(c, GW_HWNDNEXT).ok();
        }
    }
    out
}

fn scale(v: i32) -> i32 {
    let dpi = unsafe { GetDpiForWindow(main_hwnd()) } as i32;
    v * dpi.max(96) / 96
}

fn layout(hwnd: HWND) {
    let mut rc = RECT::default();
    unsafe {
        let _ = GetClientRect(hwnd, &mut rc);
    }
    let bar = scale(UI_BAR_CSS).min(rc.bottom);
    let ui_h = match POPOVER_BOTTOM.with(Cell::get) {
        Some(b) => scale(b + 16).max(bar).min(rc.bottom),
        None => bar,
    };
    unsafe {
        if let Some(t) = tab() {
            let _ = t.controller.SetBounds(RECT { left: 0, top: bar, right: rc.right, bottom: rc.bottom });
        }
        if let Some(u) = ui_view() {
            let _ = u.controller.SetBounds(RECT { left: 0, top: 0, right: rc.right, bottom: ui_h });
        }
        let host = UI_HOST_HWND.with(Cell::get);
        if !host.is_invalid() {
            let _ = SetWindowPos(host, Some(HWND_TOP), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
    }
}

/// Minimized: hide both views and ask WebView2 to drop memory. Restored: back to normal.
fn set_minimized(minimized: bool) {
    for v in [tab(), ui_view()].into_iter().flatten() {
        unsafe {
            let _ = v.controller.SetIsVisible(!minimized);
            if let Ok(w19) = v.webview.cast::<ICoreWebView2_19>() {
                let level = if minimized {
                    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
                } else {
                    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
                };
                let _ = w19.SetMemoryUsageTargetLevel(level);
            }
        }
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            // Frameless: drop the caption but keep the system resize borders and shadow.
            WM_NCCALCSIZE if wparam.0 != 0 => {
                let params = &mut *(lparam.0 as *mut NCCALCSIZE_PARAMS);
                let top = params.rgrc[0].top;
                let r = DefWindowProcW(hwnd, msg, wparam, lparam);
                params.rgrc[0].top = top;
                if IsZoomed(hwnd).as_bool() {
                    let dpi = GetDpiForWindow(hwnd);
                    params.rgrc[0].top += GetSystemMetricsForDpi(SM_CYFRAME, dpi)
                        + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);
                }
                r
            }
            WM_SIZE => {
                set_minimized(wparam.0 as u32 == SIZE_MINIMIZED);
                layout(hwnd);
                LRESULT(0)
            }
            WM_DPICHANGED => {
                let r = &*(lparam.0 as *const RECT);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    r.left,
                    r.top,
                    r.right - r.left,
                    r.bottom - r.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                layout(hwnd);
                LRESULT(0)
            }
            WM_TIMER if wparam.0 == TIMER_PUSH_SHIELDS => {
                let _ = KillTimer(Some(hwnd), TIMER_PUSH_SHIELDS);
                PUSH_PENDING.with(|p| p.set(false));
                push_shields();
                LRESULT(0)
            }
            WM_SETFOCUS => {
                if let Some(t) = tab() {
                    let _ = t.controller.MoveFocus(COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                for v in [ui_view(), tab()].into_iter().flatten() {
                    let _ = v.controller.Close();
                }
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

fn create_window() -> Result<HWND> {
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None)?.into();
        let class = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: instance,
            lpszClassName: w!("ZEO"),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hbrBackground: CreateSolidBrush(COLORREF(0x000B0A0A)),
            ..Default::default()
        };
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("ZEO"),
            w!("ZEO"),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1280,
            800,
            None,
            None,
            Some(instance),
            None,
        )?;
        MAIN.with(|m| m.set(hwnd));
        // Re-run WM_NCCALCSIZE now that our handler is in place.
        let _ = SetWindowPos(hwnd, None, 0, 0, 0, 0, SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER);
        Ok(hwnd)
    }
}

fn user_data_dir() -> HSTRING {
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
    HSTRING::from(format!("{base}\\ZEO\\User Data"))
}

/// `ui/` next to the exe (installed), or at the repo root (target/release during development).
fn ui_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("ZEO_UI_DIR") {
        return Some(PathBuf::from(d));
    }
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    [exe_dir.join("ui"), exe_dir.join("..").join("..").join("ui")]
        .into_iter()
        .find(|d| d.join("chrome").join("index.html").is_file())
        .and_then(|d| d.canonicalize().ok())
        .map(|d| PathBuf::from(d.to_string_lossy().trim_start_matches(r"\\?\")))
}

fn create_env() -> webview2_com::Result<ICoreWebView2Environment> {
    let (tx, rx) = mpsc::channel();
    let folder = user_data_dir();
    CreateCoreWebView2EnvironmentCompletedHandler::wait_for_async_operation(
        Box::new(move |handler| unsafe {
            CreateCoreWebView2EnvironmentWithOptions(
                PCWSTR::null(),
                &folder,
                None::<&ICoreWebView2EnvironmentOptions>,
                &handler,
            )
            .map_err(webview2_com::Error::WindowsError)
        }),
        Box::new(move |hr, env| {
            hr?;
            tx.send(env.ok_or_else(|| Error::from(E_POINTER))).expect("send env");
            Ok(())
        }),
    )?;
    Ok(rx.recv().map_err(|_| webview2_com::Error::SendError)??)
}

fn create_view(env: &ICoreWebView2Environment, parent: HWND) -> webview2_com::Result<View> {
    let (tx, rx) = mpsc::channel();
    let env = env.clone();
    CreateCoreWebView2ControllerCompletedHandler::wait_for_async_operation(
        Box::new(move |handler| unsafe {
            env.CreateCoreWebView2Controller(parent, &handler)
                .map_err(webview2_com::Error::WindowsError)
        }),
        Box::new(move |hr, c| {
            hr?;
            tx.send(c.ok_or_else(|| Error::from(E_POINTER))).expect("send controller");
            Ok(())
        }),
    )?;
    let controller = rx.recv().map_err(|_| webview2_com::Error::SendError)??;
    let webview = unsafe { controller.CoreWebView2()? };
    Ok(View { controller, webview })
}

/// Settings that cost memory/background work and that a privacy browser doesn't want on.
fn trim_settings(v: &View) -> Result<()> {
    unsafe {
        let settings = v.webview.Settings()?;
        if let Ok(s4) = settings.cast::<ICoreWebView2Settings4>() {
            s4.SetIsGeneralAutofillEnabled(false)?;
            s4.SetIsPasswordAutosaveEnabled(false)?;
        }
    }
    Ok(())
}

fn setup_ui_view(ui: &View, dir: &std::path::Path) -> Result<()> {
    unsafe {
        let settings = ui.webview.Settings()?;
        settings.SetAreDefaultContextMenusEnabled(false)?;
        settings.SetIsZoomControlEnabled(false)?;
        settings.SetIsStatusBarEnabled(false)?;
        if let Ok(s3) = settings.cast::<ICoreWebView2Settings3>() {
            // F5/Ctrl+R etc. must act on the tab, not reload the UI.
            s3.SetAreBrowserAcceleratorKeysEnabled(false)?;
        }
        if let Ok(s9) = settings.cast::<ICoreWebView2Settings9>() {
            // Lets `-webkit-app-region: drag` in the UI move the window.
            s9.SetIsNonClientRegionSupportEnabled(true)?;
        }
        if let Ok(c2) = ui.controller.cast::<ICoreWebView2Controller2>() {
            c2.SetDefaultBackgroundColor(BG)?;
        }
        let w3: ICoreWebView2_3 = ui.webview.cast()?;
        let dir = HSTRING::from(dir.as_os_str());
        w3.SetVirtualHostNameToFolderMapping(
            &HSTRING::from(UI_HOST),
            &dir,
            COREWEBVIEW2_HOST_RESOURCE_ACCESS_KIND_DENY,
        )?;

        AddScriptToExecuteOnDocumentCreatedCompletedHandler::wait_for_async_operation(
            Box::new({
                let webview = ui.webview.clone();
                move |handler| {
                    webview
                        .AddScriptToExecuteOnDocumentCreated(
                            w!("window.__zenNative = { postMessage: (s) => window.chrome.webview.postMessage(s) };"),
                            &handler,
                        )
                        .map_err(webview2_com::Error::WindowsError)
                }
            }),
            Box::new(|hr, _| hr),
        )
        .map_err(|e| match e {
            webview2_com::Error::WindowsError(e) => e,
            _ => Error::from(E_FAIL),
        })?;

        let mut token = 0i64;
        ui.webview.add_WebMessageReceived(
            &WebMessageReceivedEventHandler::create(Box::new(|_, args| {
                let Some(args) = args else { return Ok(()) };
                // Only our own UI pages may drive the browser.
                let source = take(|p| args.Source(p));
                if !source.starts_with(UI_ORIGIN) {
                    log(&format!("REJECTED message from {source}"));
                    return Ok(());
                }
                let raw = take(|p| args.TryGetWebMessageAsString(p));
                if let Ok(v) = serde_json::from_str::<Value>(&raw) {
                    let kind = v.get("type").and_then(Value::as_str).unwrap_or("");
                    log(&format!("UI {kind}"));
                    handle_ui_message(kind, v.get("payload").unwrap_or(&Value::Null));
                }
                Ok(())
            })),
            &mut token,
        )?;

        // Browser shortcuts while the UI has focus act on the tab.
        ui.controller.add_AcceleratorKeyPressed(
            &AcceleratorKeyPressedEventHandler::create(Box::new(|_, args| {
                let Some(args) = args else { return Ok(()) };
                let mut kind = COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN;
                args.KeyEventKind(&mut kind)?;
                if kind != COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN && kind != COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN {
                    return Ok(());
                }
                let mut vk = 0u32;
                args.VirtualKey(&mut vk)?;
                let vk = VIRTUAL_KEY(vk as u16);
                let ctrl = GetKeyState(VK_CONTROL.0 as i32) < 0;
                let alt = GetKeyState(VK_MENU.0 as i32) < 0;
                let t = tab();
                let handled = match (vk, ctrl, alt, t) {
                    (VK_F5, _, _, _) | (VK_R, true, _, _) => {
                        reload_tab();
                        true
                    }
                    (VK_LEFT, _, true, Some(t)) => t.webview.GoBack().is_ok(),
                    (VK_RIGHT, _, true, Some(t)) => t.webview.GoForward().is_ok(),
                    _ => false,
                };
                if handled {
                    args.SetHandled(true)?;
                }
                Ok(())
            })),
            &mut token,
        )?;

        let url = HSTRING::from(format!("{UI_ORIGIN}chrome/index.html"));
        ui.webview.Navigate(&url)?;
    }
    Ok(())
}

fn setup_tab_view(t: &View) -> Result<()> {
    let mut token = 0i64;
    unsafe {
        // Every request, including iframes and workers where the runtime supports it.
        match t.webview.cast::<ICoreWebView2_22>() {
            Ok(w22) => w22.AddWebResourceRequestedFilterWithRequestSourceKinds(
                w!("*"),
                COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
            )?,
            Err(_) => t.webview.AddWebResourceRequestedFilter(w!("*"), COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL)?,
        }

        t.webview.add_WebResourceRequested(
            &WebResourceRequestedEventHandler::create(Box::new(|_, args| {
                let Some(args) = args else { return Ok(()) };
                let uri = take(|p| args.Request()?.Uri(p));
                let mut ctx = COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL;
                args.ResourceContext(&mut ctx)?;
                let page = PAGE_URL.with(|p| p.borrow().clone());
                if !shields_on_for(&host_of(&page)) {
                    return Ok(());
                }
                let source = if page.is_empty() { uri.clone() } else { page };
                let verdict = SHIELDS.with(|s| s.get().and_then(|s| s.check(&uri, &source, kind_of(ctx))));
                if let Some(cat) = verdict {
                    let env = ENV.with(|e| e.get().cloned()).ok_or_else(|| Error::from(E_FAIL))?;
                    let resp = env.CreateWebResourceResponse(
                        None::<&IStream>,
                        403,
                        w!("Blocked by ZEO Shields"),
                        w!(""),
                    )?;
                    args.SetResponse(&resp)?;
                    match cat {
                        Category::Ad => ADS.with(|c| c.set(c.get() + 1)),
                        Category::Tracker => TRACKERS.with(|c| c.set(c.get() + 1)),
                    }
                    log(&format!("BLOCK {cat:?} {uri}"));
                    schedule_shields_push();
                }
                Ok(())
            })),
            &mut token,
        )?;

        t.webview.add_NavigationStarting(
            &NavigationStartingEventHandler::create(Box::new(|_, args| {
                if let Some(args) = args {
                    let uri = take(|p| args.Uri(p));
                    log(&format!("NAVIGATE {uri}"));
                    PAGE_URL.with(|p| *p.borrow_mut() = uri);
                    ADS.with(|c| c.set(0));
                    TRACKERS.with(|c| c.set(0));
                    LOADING.with(|l| l.set(true));
                    push_shields();
                    push_nav();
                }
                Ok(())
            })),
            &mut token,
        )?;

        t.webview.add_NavigationCompleted(
            &NavigationCompletedEventHandler::create(Box::new(|_, _| {
                log(&format!("DONE ads={} trackers={}", ADS.with(Cell::get), TRACKERS.with(Cell::get)));
                LOADING.with(|l| l.set(false));
                push_nav();
                push_shields();
                Ok(())
            })),
            &mut token,
        )?;

        t.webview.add_SourceChanged(
            &SourceChangedEventHandler::create(Box::new(|_, _| {
                push_nav();
                push_tabs();
                Ok(())
            })),
            &mut token,
        )?;

        t.webview.add_HistoryChanged(
            &HistoryChangedEventHandler::create(Box::new(|_, _| {
                push_nav();
                Ok(())
            })),
            &mut token,
        )?;

        t.webview.add_DocumentTitleChanged(
            &DocumentTitleChangedEventHandler::create(Box::new(|wv, _| {
                if let Some(wv) = wv {
                    let title = take(|p| wv.DocumentTitle(p));
                    let text = HSTRING::from(if title.is_empty() { "ZEO".into() } else { format!("{title} — ZEO") });
                    let _ = SetWindowTextW(main_hwnd(), &text);
                }
                push_tabs();
                Ok(())
            })),
            &mut token,
        )?;

        // No multi-tab yet: open new-window requests in the current view.
        t.webview.add_NewWindowRequested(
            &NewWindowRequestedEventHandler::create(Box::new(|wv, args| {
                if let (Some(wv), Some(args)) = (wv, args) {
                    let uri = take(|p| args.Uri(p));
                    args.SetHandled(true)?;
                    wv.Navigate(&HSTRING::from(uri.as_str()))?;
                }
                Ok(())
            })),
            &mut token,
        )?;

        // Ctrl+L while the page has focus: hand focus to the omnibox in the UI view.
        t.controller.add_AcceleratorKeyPressed(
            &AcceleratorKeyPressedEventHandler::create(Box::new(|_, args| {
                if let Some(args) = args {
                    let mut kind = COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN;
                    args.KeyEventKind(&mut kind)?;
                    let mut vk = 0u32;
                    args.VirtualKey(&mut vk)?;
                    let ctrl = GetKeyState(VK_CONTROL.0 as i32) < 0;
                    if kind == COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN && ctrl && vk == VK_L.0 as u32 {
                        args.SetHandled(true)?;
                        if let Some(ui) = ui_view() {
                            let _ = ui.controller.MoveFocus(COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC);
                            let _ = ui.webview.ExecuteScript(
                                w!("document.dispatchEvent(new KeyboardEvent('keydown', { key: 'l', ctrlKey: true }))"),
                                &ExecuteScriptCompletedHandler::create(Box::new(|_, _| Ok(()))),
                            );
                        }
                    }
                }
                Ok(())
            })),
            &mut token,
        )?;
    }
    Ok(())
}

fn run() -> std::result::Result<(), String> {
    let err = |e: Error| e.to_string();
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().map_err(err)?;
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let t0 = Instant::now();
    let shields = Shields::new(
        &[include_str!("../../../resources/filters/easylist.txt")],
        &[include_str!("../../../resources/filters/easyprivacy.txt")],
    );
    log(&format!("SHIELDS ready in {} ms", t0.elapsed().as_millis()));
    SHIELDS.with(|s| {
        let _ = s.set(shields);
    });

    let dir = ui_dir().ok_or("ไม่พบโฟลเดอร์ ui (ต้องมี ui\\chrome\\index.html ข้าง zeo.exe)")?;
    let hwnd = create_window().map_err(err)?;
    let env = create_env().map_err(|e| format!("WebView2: {e:?}"))?;
    ENV.with(|e| {
        let _ = e.set(env.clone());
    });

    let tab_view = create_view(&env, hwnd).map_err(|e| format!("WebView2: {e:?}"))?;
    trim_settings(&tab_view).map_err(err)?;
    setup_tab_view(&tab_view).map_err(err)?;
    TAB.with(|c| {
        let _ = c.set(tab_view.clone());
    });

    let before = direct_children(hwnd);
    let ui = create_view(&env, hwnd).map_err(|e| format!("WebView2: {e:?}"))?;
    if let Some(h) = direct_children(hwnd).into_iter().find(|h| !before.contains(h)) {
        UI_HOST_HWND.with(|c| c.set(h));
    }
    trim_settings(&ui).map_err(err)?;
    setup_ui_view(&ui, &dir).map_err(err)?;
    UI.with(|c| {
        let _ = c.set(ui.clone());
    });

    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        layout(hwnd);
        let _ = tab_view.controller.SetIsVisible(true);
        let _ = ui.controller.SetIsVisible(true);
    }
    let start = std::env::args().nth(1).unwrap_or_else(|| HOME.to_string());
    navigate(&start);
    log(&format!("STARTUP to first navigate {} ms", t0.elapsed().as_millis()));

    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        log(&format!("FATAL {e}"));
        let text = HSTRING::from(format!("ZEO เปิดไม่สำเร็จ:\n{e}"));
        unsafe {
            MessageBoxW(None, &text, w!("ZEO"), MB_ICONERROR);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{host_of, to_url};

    #[test]
    fn omnibox() {
        assert_eq!(to_url("example.com"), "https://example.com");
        assert_eq!(to_url("http://a.b/c"), "http://a.b/c");
        assert_eq!(to_url("rust lang"), "https://duckduckgo.com/?q=rust+lang");
        assert_eq!(to_url("ข่าว"), "https://duckduckgo.com/?q=%E0%B8%82%E0%B9%88%E0%B8%B2%E0%B8%A7");
        assert_eq!(to_url("localhost:8080"), "https://localhost:8080");
    }

    #[test]
    fn hosts() {
        assert_eq!(host_of("https://www.Sanook.com/news/?a=1"), "sanook.com");
        assert_eq!(host_of("http://user:pw@a.b.c:8080/x"), "a.b.c");
        assert_eq!(host_of("about:blank"), "");
    }
}
