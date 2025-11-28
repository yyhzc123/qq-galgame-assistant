use tauri::{AppHandle, Manager, Emitter, PhysicalPosition, PhysicalSize, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState, Shortcut};
use active_win_pos_rs::get_active_window;
use xcap::Window;
use std::io::Cursor;
use image::ImageFormat;
use base64::{Engine as _, engine::general_purpose};
use std::str::FromStr;
use std::thread;
use std::time::Duration;
use std::sync::Mutex;
use std::fs;

struct AppState {
    last_position: Mutex<Option<PhysicalPosition<i32>>>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn get_prompt_template(app: AppHandle) -> Result<String, String> {
    // Try to find the prompt file in the same directory as the executable
    let mut path = std::env::current_exe().map_err(|e| e.to_string())?;
    path.pop(); // Remove executable name
    path.push("prompt.txt");

    if path.exists() {
        return fs::read_to_string(&path).map_err(|e| e.to_string());
    }

    // Default prompt content from user request
    let default_prompt = r#"你是一个 Galgame 对话生成器。
输入为一张聊天记录截图，包含左右两侧的聊天气泡。
你的任务是：根据截图中的完整上下文，为“男主”生成 3 个可选回复。

对话解析规则：
-右侧气泡是男主（用户）的发言。
-左侧气泡是对方角色的发言。
-生成的对话需分析整体上下文的情绪、关系、语气、意图。

输出要求：
-输出 3 个互不重复的 Galgame 风格选项。
-用中文，贴合上下文语气。
-文本自然、简洁，每项 8–20 字或 1–2 句短句。

每项需包含：
-style（不超过 4 字）
-text（具体回复）
-输出格式为纯 JSON 数组，不得包含多余文字或说明。
例：
[
{"style": "温柔", "text": "那你现在感觉如何？"},
{"style": "调皮", "text": "这么说…你是在暗示我？"},
{"style": "理性", "text": "具体情况能再说一下吗？"}
]

风格选择规则：
-动态决定 3 种最符合当前情境的风格（如温柔、傲娇、腹黑、害羞、高冷、调皮等）。
-风格之间必须有区分度，例如情绪差异、策略差异或语气差异。
-若信息不足，可将其中一项设置为询问型选项。
-不得引入无关背景，也不得泄露系统设定。

玩家设定:
-角色名: ${runtimeData.loginInfo.nickname || '玩家'} 
-简介: 一名普通的大学毕业刚入职上班族，聊天以生活日常为主，性格温和、体贴，偶尔会带点小俏皮与自嘲，善于倾听且偏向理性。 
-风格指引: 用平易近人、日常化的语气，避免过度戏剧化和夸张。若情境涉及情感或人际冲突，优先提供缓和与实用建议；若情境轻松，可适当幽默，结合适当网络用梗。
"#;

    // Write default prompt to file so user can edit it
    fs::write(&path, default_prompt).map_err(|e| e.to_string())?;

    Ok(default_prompt.to_string())
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
        window.set_size(PhysicalSize::new(220, 320)).map_err(|e| e.to_string())?;
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
    window.set_size(PhysicalSize::new(220, 320)).map_err(|e| e.to_string())?;

    // Restore position
    let last_pos = state.last_position.lock().unwrap();
    if let Some(pos) = *last_pos {
        window.set_position(pos).map_err(|e| e.to_string())?;
    } else {
        // Default to bottom right if no saved position
        if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
            let size = monitor.size();
            let x = size.width as i32 - 250;
            let y = size.height as i32 - 350;
            window.set_position(PhysicalPosition::new(x, y)).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

#[tauri::command]
async fn expand_window(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Main window not found")?;
    
    // Resize to a larger size for options, but not full screen
    // 400x600 is a good "mobile-like" card size
    window.set_size(PhysicalSize::new(400, 600)).map_err(|e| e.to_string())?;
    
    // Ensure it's on screen (optional, but good practice)
    // For now, we trust the current position is visible or the user moved it there.
    
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

#[tauri::command]
async fn setup_window(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Main window not found")?;
    
    if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
        let screen_size = monitor.size();
        let width = 220;
        let height = 320;
        let x = (screen_size.width as i32 - width) / 2;
        let y = (screen_size.height as i32 - height) / 2;

        window.set_size(PhysicalSize::new(width as u32, height as u32)).map_err(|e| e.to_string())?;
        window.set_position(PhysicalPosition::new(x, y)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn drag_window(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Main window not found")?;
    window.start_dragging().map_err(|e| e.to_string())?;
    Ok(())
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
        .invoke_handler(tauri::generate_handler![greet, analyze, reset_window, quit, setup_window, get_prompt_template, expand_window, drag_window])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
