use tauri::{AppHandle, Manager, Emitter, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState, Shortcut};
use active_win_pos_rs::get_active_window;
use xcap::Window;
use std::io::Cursor;
use image::ImageFormat;
use base64::{Engine as _, engine::general_purpose};
use std::str::FromStr;
use std::thread;
use std::time::Duration;
use std::path::PathBuf;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn analyze(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Main window not found")?;
    
    // 1. Hide the window so we don't capture it
    window.hide().map_err(|e| e.to_string())?;
    
    // 2. Wait for the window to disappear and the previous window to gain focus
    thread::sleep(Duration::from_millis(300));

    // 3. Capture the now active window (which should be QQ/WeChat)
    let base64_image = capture_active_window();

    // 4. Show the window again and maximize/fullscreen it for the overlay
    if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
        let size = monitor.size();
        window.set_size(tauri::Size::Physical(size.clone())).map_err(|e| e.to_string())?;
        window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x: 0, y: 0 })).map_err(|e| e.to_string())?;
    }

    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;

    if let Some(img) = base64_image {
        app.emit("analyze-chat", img).map_err(|e| e.to_string())?;
    } else {
        app.emit("analyze-error", "Failed to capture window").map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
async fn reset_window(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Main window not found")?;
    window.set_size(tauri::Size::Logical(tauri::LogicalSize { width: 120.0, height: 120.0 })).map_err(|e| e.to_string())?;
    Ok(())
}

fn capture_active_window() -> Option<String> {
    let active_window = get_active_window().ok()?;
    let windows = Window::all().ok()?;
    
    // Use process_path instead of process_name
    let process_path_str = active_window.process_path.to_string_lossy();
    println!("Active Window: {} ({})", active_window.title, process_path_str);
    
    // Find the window that matches the active window
    let target_window = windows.into_iter().find(|w| {
        // Relaxed matching: Title matches OR process_path contains app_name (case insensitive)
        // We check if the active window's process path contains the xcap window's app name
        w.title() == active_window.title || 
        process_path_str.to_lowercase().contains(&w.app_name().to_lowercase())
    });

    if let Some(window) = target_window {
        println!("Capturing window: {}", window.title());
        let image = window.capture_image().ok()?;
        let mut bytes: Vec<u8> = Vec::new();
        // Use ImageFormat::Png
        image.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png).ok()?;
        let b64 = general_purpose::STANDARD.encode(&bytes);
        return Some(b64);
    }
    None
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(desktop)]
            {
                let _app_handle = app.handle().clone();
                let shortcut_str = "Ctrl+Shift+Z";
                let shortcut = Shortcut::from_str(shortcut_str).unwrap();
                
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |app, _shortcut, event| {
                            if event.state == ShortcutState::Pressed {
                                 if let Some(base64_image) = capture_active_window() {
                                    let _ = app.emit("analyze-chat", base64_image);
                                }
                            }
                        })
                        .build(),
                )?;
                
                app.global_shortcut().register(shortcut)?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, analyze, reset_window])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
