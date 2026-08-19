mod ai;
mod parser;
mod report;
mod settings;
mod storage;
mod sync;

use std::path::PathBuf;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

/// 数据根目录：~/Library/Application Support/WorkTrace
pub fn data_dir() -> Result<PathBuf, String> {
    let dir = dirs::data_dir()
        .ok_or_else(|| "无法定位系统数据目录".to_string())?
        .join("WorkTrace");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// 更新完成后重启应用
#[tauri::command]
fn restart_app(app: tauri::AppHandle) {
    tauri::process::restart(&app.env());
}

/// 显示主窗口（若已隐藏则重新显示并聚焦）
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // 系统托盘：关闭窗口后仍驻留任务栏，右键菜单可「打开主窗口 / 退出」
            let open_i = MenuItem::with_id(app, "open", "打开主窗口", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_i, &quit_i])?;

            let _tray = TrayIconBuilder::with_id("main-tray")
                .tooltip("工作日迹 WorkTrace")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    // 左键单击托盘图标也打开主窗口（体验更直觉）
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            // 启动后延迟同步（等应用就绪）
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                let _ = sync::sync_now_internal().await;
            });

            // 定时同步：每 60 秒检查一次，累计到配置间隔后触发
            tauri::async_runtime::spawn(async move {
                let mut elapsed: u64 = 0;
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    let interval = settings::get_sync_interval_minutes().unwrap_or(0);
                    if interval == 0 {
                        elapsed = 0;
                        continue;
                    }
                    elapsed += 1;
                    if elapsed >= interval as u64 {
                        elapsed = 0;
                        let _ = sync::sync_now_internal().await;
                    }
                }
            });

            Ok(())
        })
        // 拦截窗口关闭：默认隐藏到托盘，不真正退出
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            ai::transform_record,
            ai::transform_todo,
            ai::test_model,
            parser::parse_attachment,
            report::generate_report,
            report::get_report,
            report::save_report,
            report::export_report,
            settings::list_models,
            settings::save_model,
            settings::delete_model,
            settings::list_webdav_configs,
            settings::save_webdav_config,
            settings::delete_webdav_config,
            settings::get_backup_settings,
            settings::save_backup_settings,
            storage::get_day_record,
            storage::save_day_record,
            storage::confirm_record,
            storage::confirm_todo,
            storage::delete_entry,
            storage::update_entry,
            storage::get_month_active,
            storage::get_year_active,
            sync::sync_now,
            sync::test_webdav,
            restart_app,
        ])
        .build(tauri::generate_context!())
        .expect("error while building WorkTrace")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                // 退出前尽力同步，但限制最长 10 秒，避免网络异常时阻塞主线程导致卡死
                let _ = tauri::async_runtime::block_on(async {
                    let _ = tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        sync::sync_now_internal(),
                    )
                    .await;
                });
            }
        });
}
