mod ai;
mod parser;
mod report;
mod settings;
mod storage;
mod sync;

use std::path::PathBuf;
use tauri::Manager;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|_app| {
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            ai::transform_record,
            ai::test_model,
            parser::parse_attachment,
            report::generate_report,
            report::get_report,
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
