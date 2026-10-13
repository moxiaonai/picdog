use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::window::Color;
use tauri::{App, AppHandle, Emitter, Manager, Runtime};

const MENU_SHOW_MAIN: &str = "show-main";
const MENU_OPEN_SETTINGS: &str = "open-settings";
const MENU_OPEN_CDN_SETTINGS: &str = "open-cdn-settings";
const MENU_OPEN_HISTORY: &str = "open-history";
const MENU_HIDE_MAIN: &str = "hide-main";
const MENU_QUIT: &str = "quit";

const EVENT_OPEN_SETTINGS: &str = "picdog://open-settings";
const EVENT_OPEN_CDN_SETTINGS: &str = "picdog://open-cdn-settings";
const EVENT_OPEN_HISTORY: &str = "picdog://open-history";

pub fn setup(app: &mut App) -> tauri::Result<()> {
    setup_main_window_chrome(app);

    let menu = build_menu(app)?;
    app.set_menu(menu)?;

    let tray_menu = build_tray_menu(app)?;
    let mut tray = TrayIconBuilder::with_id("picdog")
        .menu(&tray_menu)
        .tooltip("PicDog")
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| handle_menu_event(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.build(app)?;
    Ok(())
}

fn setup_main_window_chrome(app: &App) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_shadow(true);
        let _ = window.set_background_color(Some(Color(0, 0, 0, 0)));
    }
}

fn build_menu(app: &App) -> tauri::Result<Menu<tauri::Wry>> {
    let about = PredefinedMenuItem::about(
        app,
        None,
        Some(AboutMetadata {
            name: Some("PicDog".to_string()),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            short_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            comments: Some("Image compression and upload desktop tool".to_string()),
            ..Default::default()
        }),
    )?;
    let show = MenuItem::with_id(
        app,
        MENU_SHOW_MAIN,
        "显示 PicDog",
        true,
        Some("CmdOrCtrl+Shift+P"),
    )?;
    let settings = MenuItem::with_id(app, MENU_OPEN_SETTINGS, "设置…", true, Some("CmdOrCtrl+,"))?;
    let cdn_settings = MenuItem::with_id(
        app,
        MENU_OPEN_CDN_SETTINGS,
        "CDN 配置…",
        true,
        Some("CmdOrCtrl+Shift+,"),
    )?;
    let history = MenuItem::with_id(
        app,
        MENU_OPEN_HISTORY,
        "历史记录",
        true,
        Some("CmdOrCtrl+Y"),
    )?;
    let hide = MenuItem::with_id(app, MENU_HIDE_MAIN, "隐藏窗口", true, Some("CmdOrCtrl+H"))?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "退出 PicDog", true, Some("CmdOrCtrl+Q"))?;
    let app_menu = Submenu::with_items(
        app,
        "PicDog",
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(app)?,
            &show,
            &settings,
            &cdn_settings,
            &history,
            &PredefinedMenuItem::separator(app)?,
            &hide,
            &quit,
        ],
    )?;

    Menu::with_items(app, &[&app_menu])
}

fn build_tray_menu(app: &App) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, MENU_SHOW_MAIN, "显示 PicDog", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, MENU_OPEN_SETTINGS, "设置", true, None::<&str>)?;
    let cdn_settings =
        MenuItem::with_id(app, MENU_OPEN_CDN_SETTINGS, "CDN 配置", true, None::<&str>)?;
    let history = MenuItem::with_id(app, MENU_OPEN_HISTORY, "历史记录", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "退出 PicDog", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &show,
            &settings,
            &cdn_settings,
            &history,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        MENU_SHOW_MAIN => show_main_window(app),
        MENU_OPEN_SETTINGS => {
            show_main_window(app);
            let _ = app.emit(EVENT_OPEN_SETTINGS, ());
        }
        MENU_OPEN_CDN_SETTINGS => {
            show_main_window(app);
            let _ = app.emit(EVENT_OPEN_CDN_SETTINGS, ());
        }
        MENU_OPEN_HISTORY => {
            show_main_window(app);
            let _ = app.emit(EVENT_OPEN_HISTORY, ());
        }
        MENU_HIDE_MAIN => hide_main_window(app),
        MENU_QUIT => app.exit(0),
        _ => {}
    }
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn hide_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}
