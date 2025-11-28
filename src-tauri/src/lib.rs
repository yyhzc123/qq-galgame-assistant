use tauri::{AppHandle, Manager, Emitter, WebviewWindow, PhysicalPosition, PhysicalSize, State};
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
use std::sync::Mutex;

struct AppState {
    last_position: Mutex<Option<PhysicalPosition<i32>>>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn analyze(app: AppHandle, state: State<'_, AppState>, silent: bool) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Main window not found")?;
    
    // Save current position if not already saved (or update it)
    if let Ok(pos) = window.outer_position() {
        let mut last_pos = state.last_position.lock().unwrap();
        *last_pos = Some(pos);
    }

    // 1. Hide the window so we don't capture it
    window.hide().map_err(|e| e.to_string())?;
    
    // 2. Wait for the window to disappear
    thread::sleep(Duration::from_millis(300));

    // 3. Capture the now active window
    let base64_image = capture_active_window();

    // 4. Resize and Reposition
    if silent {
        // Restore to widget size/pos immediately
        window.set_size(PhysicalSize::new(100, 100)).map_err(|e| e.to_string())?;
        let last_pos = state.last_position.lock().unwrap();
        if let Some(pos) = *last_pos {
            window.set_position(pos).map_err(|e| e.to_string())?;
        }
    } else {
        // Dialogue Mode
        if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
            let screen_size = monitor.size();
            let width = 800;
            let height = 600; // Taller for vertical options
            let x = (screen_size.width as i32 - width) / 2;
            let y = (screen_size.height as i32 - height) / 2; // Center on screen

            window.set_size(PhysicalSize::new(width as u32, height as u32)).map_err(|e| e.to_string())?;
            window.set_position(PhysicalPosition::new(x, y)).map_err(|e| e.to_string())?;
        }
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
async fn reset_window(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Main window not found")?;
    
    // Resize back to widget size
    window.set_size(PhysicalSize::new(100, 100)).map_err(|e| e.to_string())?;

    // Restore position
    let last_pos = state.last_position.lock().unwrap();
    if let Some(pos) = *last_pos {
        window.set_position(pos).map_err(|e| e.to_string())?;
    } else {
        // Default to bottom right if no saved position
        if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
            let size = monitor.size();
            let x = size.width as i32 - 150;
            let y = size.height as i32 - 150;
            window.set_position(PhysicalPosition::new(x, y)).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

fn capture_active_window() -> Option<String> {
    let active_window = get_active_window().ok()?;
    let windows = Window::all().ok()?;
    
    let process_path_str = active_window.process_path.to_string_lossy();
    println!("Active Window: {} ({})", active_window.title, process_path_str);
    
    let target_window = windows.into_iter().find(|w| {
        w.title() == active_window.title || 
        process_path_str.to_lowercase().contains(&w.app_name().to_lowercase())
    });

    if let Some(window) = target_window {
        println!("Capturing window: {}", window.title());
        let image = window.capture_image().ok()?;
        let mut bytes: Vec<u8> = Vec::new();
        image.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png).ok()?;
        let b64 = general_purpose::STANDARD.encode(&bytes);
        return Some(b64);
    }
    None
}

#[tauri::command]
fn quit() {
    std::process::exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { last_position: Mutex::new(None) })
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
                                 let _ = app.emit("trigger-analyze", ());
                            }
                        })
                        .build(),
                )?;
                
                app.global_shortcut().register(shortcut)?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, analyze, reset_window, quit])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
