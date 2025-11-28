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
    let default_prompt = r#"You are a Galgame Assistant. Analyze the chat history in the image.
- Messages on the RIGHT side are from the USER (me).
- Messages on the LEFT side are from OTHERS.

Task: Analyze the **entire conversation context** (both your messages on the right and others' on the left) to understand the flow, relationship, and mood.
Formulate a reply to the latest message based on this full context.

Provide 3 distinct reply options in a Galgame style (in Chinese).
Instead of fixed archetypes, dynamically choose 3 most suitable styles based on the conversation context (e.g., 傲娇, 温柔, 腹黑, 幽默, 害羞, 调皮, 高冷, etc.).

Output ONLY a valid JSON array of objects, where each object has:
- "style": The style name in Chinese (max 4 chars).
- "text": The reply text.

Example JSON format:
[
  {"style": "傲娇", "text": "哼，谁...谁稀罕你回消息啊！"},
  {"style": "温柔", "text": "今天也很辛苦呢，要注意休息哦。"},
  {"style": "腹黑", "text": "呵呵，看来你很懂嘛~"}
]
Do not include markdown formatting.

---
Additional Instructions:
- 语言与风格: 回复简洁明了、符合当前场景与角色设定。长度适中（建议每项不超过 1-2 句、约 8-20 个汉字或相当简短的句子）。
- 情感与语气: 依据上下文保持一致（例如严肃/幽默/冷静/挑衅）；不要自我引用或说明你是 AI。
- 信息限定: 不得引入与当前对话无关的背景信息或新任务；若需要说明信息缺失，可把一项设置为询问类选项（例如“询问更多细节”）。
- 内容安全: 不得包含违法、不当或露骨内容；遵守游戏分级与平台规则。
- 策略覆盖: 当场景适合时，三选项应涵盖不同策略倾向（例如“和平/折中/激进”或“事实/情感/引导”）。
- 输出优先级: 如果上下文中有明确角色身份或目标，请优先让选项贴合该角色目标与身份。

玩家设定（默认，日常生活向）:
- 角色名: ${runtimeData.loginInfo.nickname || '玩家'}
- 简介: 一名普通的上班族（或大学毕业刚入职），生活日常为主，性格温和、体贴，偶尔会带点小俏皮与自嘲，善于倾听且偏向理性。
- 风格指引: 用平易近人、日常化的语气，避免过度戏剧化和夸张。若情境涉及情感或人际冲突，优先提供缓和与实用建议；若情境轻松，可适当幽默。
- 用途: 当没有额外玩家设定时，按此设定生成选项；若场景指定其他玩家设定，以场景指定为准。
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
        window.set_size(PhysicalSize::new(160, 250)).map_err(|e| e.to_string())?;
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
    window.set_size(PhysicalSize::new(160, 250)).map_err(|e| e.to_string())?;

    // Restore position
    let last_pos = state.last_position.lock().unwrap();
    if let Some(pos) = *last_pos {
        window.set_position(pos).map_err(|e| e.to_string())?;
    } else {
        // Default to bottom right if no saved position
        if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
            let size = monitor.size();
            let x = size.width as i32 - 200;
            let y = size.height as i32 - 300;
            window.set_position(PhysicalPosition::new(x, y)).map_err(|e| e.to_string())?;
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
    let default_prompt = r#"You are a Galgame Assistant. Analyze the chat history in the image.
- Messages on the RIGHT side are from the USER (me).
- Messages on the LEFT side are from OTHERS.

Task: Analyze the **entire conversation context** (both your messages on the right and others' on the left) to understand the flow, relationship, and mood.
Formulate a reply to the latest message based on this full context.

Provide 3 distinct reply options in a Galgame style (in Chinese).
Instead of fixed archetypes, dynamically choose 3 most suitable styles based on the conversation context (e.g., 傲娇, 温柔, 腹黑, 幽默, 害羞, 调皮, 高冷, etc.).

Output ONLY a valid JSON array of objects, where each object has:
- "style": The style name in Chinese (max 4 chars).
- "text": The reply text.

Example JSON format:
[
  {"style": "傲娇", "text": "哼，谁...谁稀罕你回消息啊！"},
  {"style": "温柔", "text": "今天也很辛苦呢，要注意休息哦。"},
  {"style": "腹黑", "text": "呵呵，看来你很懂嘛~"}
]
Do not include markdown formatting.

---
Additional Instructions:
- 语言与风格: 回复简洁明了、符合当前场景与角色设定。长度适中（建议每项不超过 1-2 句、约 8-20 个汉字或相当简短的句子）。
- 情感与语气: 依据上下文保持一致（例如严肃/幽默/冷静/挑衅）；不要自我引用或说明你是 AI。
- 信息限定: 不得引入与当前对话无关的背景信息或新任务；若需要说明信息缺失，可把一项设置为询问类选项（例如“询问更多细节”）。
- 内容安全: 不得包含违法、不当或露骨内容；遵守游戏分级与平台规则。
- 策略覆盖: 当场景适合时，三选项应涵盖不同策略倾向（例如“和平/折中/激进”或“事实/情感/引导”）。
- 输出优先级: 如果上下文中有明确角色身份或目标，请优先让选项贴合该角色目标与身份。

玩家设定（默认，日常生活向）:
- 角色名: ${runtimeData.loginInfo.nickname || '玩家'}
- 简介: 一名普通的上班族（或大学毕业刚入职），生活日常为主，性格温和、体贴，偶尔会带点小俏皮与自嘲，善于倾听且偏向理性。
- 风格指引: 用平易近人、日常化的语气，避免过度戏剧化和夸张。若情境涉及情感或人际冲突，优先提供缓和与实用建议；若情境轻松，可适当幽默。
- 用途: 当没有额外玩家设定时，按此设定生成选项；若场景指定其他玩家设定，以场景指定为准。
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
        window.set_size(PhysicalSize::new(160, 250)).map_err(|e| e.to_string())?;
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
    window.set_size(PhysicalSize::new(160, 250)).map_err(|e| e.to_string())?;

    // Restore position
    let last_pos = state.last_position.lock().unwrap();
    if let Some(pos) = *last_pos {
        window.set_position(pos).map_err(|e| e.to_string())?;
    } else {
        // Default to bottom right if no saved position
        if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
            let size = monitor.size();
            let x = size.width as i32 - 200;
            let y = size.height as i32 - 300;
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
        let width = 300;
        let height = 400;
        let x = (screen_size.width as i32 - width) / 2;
        let y = (screen_size.height as i32 - height) / 2;

        window.set_size(PhysicalSize::new(width as u32, height as u32)).map_err(|e| e.to_string())?;
        window.set_position(PhysicalPosition::new(x, y)).map_err(|e| e.to_string())?;
    }
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
        .invoke_handler(tauri::generate_handler![greet, analyze, reset_window, quit, setup_window, get_prompt_template, expand_window])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
