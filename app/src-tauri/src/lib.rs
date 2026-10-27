//! Facet core. Unofficial; not affiliated with Anthropic.
//!
//! Non-negotiable rules (CLAUDE.md) that this crate enforces:
//! official binaries only, no sign-in data, no network calls to Anthropic,
//! no rotation or pooling, no UI automation, scrubbed child environments,
//! backups before config edits, no "Claude" in Facet's own names, and no
//! prompt or message content by default.

pub mod adapter;
pub mod auth;
pub mod avatar;
pub mod bridge;
pub mod browser;
pub mod commands;
pub mod control;
pub mod ecosystem;
pub mod effective;
pub mod env;
pub mod error;
pub mod health;
pub mod launched;
pub mod mcp;
pub mod profile;
pub mod registry;
pub mod rules;
pub mod settings;
pub mod studio;
pub mod usage;

use std::path::PathBuf;
use std::time::Duration;
use tauri::{Emitter, Manager};

use commands::AppState;
use profile::Store;

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// `~/.config/facet/profiles.toml` (or `$XDG_CONFIG_HOME/facet/`).
pub fn store_path(home: &std::path::Path) -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    std::env::var_os("FACET_PROFILES")
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("facet/profiles.toml"))
}

/// `facet --open-url <url>`: open a link in the browser profile of the
/// account it came from, then exit.
pub fn run_open_url_cli(url: Option<&str>) -> i32 {
    let home = home_dir();
    let Some(url) = url else {
        eprintln!("facet: --open-url needs a link");
        return 2;
    };
    let accounts = Store::new(store_path(&home)).load().unwrap_or_default();
    match browser::open(url, &home, &accounts, commands::now_ms()) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("facet: {e}");
            1
        }
    }
}

/// `facet --launch <id>`: used by per-account menu entries. Starts or focuses
/// the account's Desktop instance, relabels its windows, then exits.
pub fn run_launch_cli(id: &str) -> i32 {
    let home = home_dir();
    let state = AppState::new(Store::new(store_path(&home)), home);
    match commands::launch(&state, id) {
        Ok(_) => {
            if let Ok(acc) = state.store.get(id) {
                if acc.x11_identity {
                    adapter::linux::relabel_windows(&acc, &state.home, Duration::from_secs(30));
                }
            }
            0
        }
        Err(e) => {
            eprintln!("facet: {e}");
            1
        }
    }
}

/// Show and focus the main window; if `switcher`, ask the UI to open the
/// quick switcher.
fn reveal(app: &tauri::AppHandle, switcher: bool) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
    if switcher {
        if let Ok(mut v) = app.state::<AppState>().startup_switcher.lock() {
            *v = true;
        }
        let _ = app.emit("open-switcher", ());
    }
}

/// Tray menu: open Facet, one entry per account (launch or bring to front),
/// quit. Rebuilt when accounts or running state change.
fn tray_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    let state = app.state::<AppState>();
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(
        app,
        "open",
        "Open Facet",
        true,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    let accounts = state.store.load().unwrap_or_default();
    let states = commands::account_states(&state).unwrap_or_default();
    for a in accounts.iter().filter(|a| a.desktop_dir.is_some()) {
        let running = states
            .iter()
            .any(|s| s.id == a.id && s.desktop_pid.is_some());
        let label = if running {
            format!("{} · bring to front", a.name)
        } else {
            format!("Launch {}", a.name)
        };
        menu.append(&MenuItem::with_id(
            app,
            format!("launch:{}", a.id),
            label,
            true,
            None::<&str>,
        )?)?;
    }
    if accounts.is_empty() {
        menu.append(&MenuItem::with_id(
            app,
            "open-empty",
            "Add an account in Facet",
            true,
            None::<&str>,
        )?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "quit",
        "Quit Facet",
        true,
        None::<&str>,
    )?)?;
    Ok(menu)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let home = home_dir();
    let state = AppState::new(Store::new(store_path(&home)), home);
    if std::env::args().any(|a| a == "--switcher") {
        if let Ok(mut v) = state.startup_switcher.lock() {
            *v = true;
        }
    }
    tauri::Builder::default()
        // A second `facet` (or `facet --switcher`) focuses the running one.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            reveal(app, args.iter().any(|a| a == "--switcher"));
        }))
        .manage(state)
        .setup(|app| {
            use tauri::tray::TrayIconBuilder;
            let handle = app.handle().clone();
            let tray = TrayIconBuilder::with_id("facet")
                .icon(app.default_window_icon().cloned().ok_or("no icon")?)
                .tooltip("Facet")
                .menu(&tray_menu(&handle)?)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| {
                    let id = event.id().as_ref();
                    match id {
                        "open" | "open-empty" => reveal(app, false),
                        "quit" => app.exit(0),
                        _ => {
                            if let Some(acc) = id.strip_prefix("launch:") {
                                let app = app.clone();
                                let acc = acc.to_string();
                                std::thread::spawn(move || {
                                    let state = app.state::<AppState>();
                                    if let Err(e) = commands::launch(&state, &acc) {
                                        let _ = app.emit("notice", e.to_string());
                                    }
                                });
                            }
                        }
                    }
                })
                .build(app)?;

            // Push account state (running instances, live sessions) to the UI
            // every two seconds, only when it changes, and keep the tray menu
            // in step. Local files only.
            std::thread::spawn(move || {
                let mut last = None;
                loop {
                    let state = handle.state::<AppState>();
                    let names: Vec<(String, String)> = state
                        .store
                        .load()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|a| (a.id, a.name))
                        .collect();
                    if let Ok(now) = commands::account_states(&state) {
                        let key = Some((now.clone(), names));
                        if last != key {
                            let _ = handle.emit("account-states", &now);
                            if let Ok(menu) = tray_menu(&handle) {
                                let _ = tray.set_menu(Some(menu));
                            }
                            last = key;
                        }
                    }
                    std::thread::sleep(Duration::from_secs(2));
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::list_accounts,
            commands::get_account_states,
            commands::save_account,
            commands::delete_account,
            commands::detect_existing,
            commands::launch_desktop,
            commands::quit_desktop,
            commands::open_terminal,
            commands::install_shim,
            commands::install_launcher,
            commands::launcher_preview,
            commands::set_account_image,
            commands::reorder_accounts,
            commands::rename_account,
            commands::usage_report,
            commands::resume_session,
            commands::export_text,
            commands::ecosystem_inventory,
            commands::timeline_events,
            commands::bridge_draft,
            commands::bridge_save,
            commands::bridge_list,
            commands::bridge_remove,
            commands::bridge_open,
            commands::mcp_add,
            commands::mcp_remove,
            commands::links_state,
            commands::links_set_account,
            commands::links_set_fallback,
            commands::links_enable,
            commands::links_disable,
            commands::links_test,
            commands::ecosystem_scan,
            commands::ecosystem_mark_reviewed,
            commands::check_sign_in,
            commands::run_health,
            commands::list_rules,
            commands::save_rules,
            commands::install_auto_command,
            commands::auto_command_info,
            commands::take_switcher_request,
            commands::control_read,
            commands::control_preview,
            commands::control_apply,
            commands::control_backups,
            commands::control_restore,
            commands::control_restore_preview,
            commands::control_policy,
            commands::control_effective,
            commands::studio_list,
            commands::studio_read,
            commands::studio_lint,
            commands::studio_template,
            commands::studio_save,
            commands::studio_remove,
            commands::studio_place_preview,
            commands::studio_place,
            commands::studio_unplace,
            commands::studio_account_items,
            commands::studio_import,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Facet");
}
