# Galgame Chat Assistant

This application runs on Windows 11 and helps you reply to QQ/WeChat messages in a Galgame style using Google Gemini AI.

## Setup

1.  Ensure you have **Rust** and **Node.js** installed.
2.  Install dependencies:
    ```bash
    npm install
    cd src-tauri
    cargo build
    cd ..
    ```

## Usage

1.  Run the application:
    ```bash
    npm run tauri dev
    ```
2.  The application window will open. You can minimize it.
3.  Open **QQ** or **WeChat** and open a chat window.
4.  Press **`Ctrl + Shift + Z`** on your keyboard.
5.  The Galgame Assistant overlay will appear with 3 reply options (Tsundere, Sweet, Funny).
6.  **Click** on an option to copy it to your clipboard.
7.  **Paste** (Ctrl+V) the reply into the chat box.

## Configuration

-   The Google AI Studio API Key is hardcoded in `src/App.vue`. You can change it there.
-   The Global Shortcut is `Ctrl+Shift+Z`, defined in `src-tauri/src/lib.rs`.

## Troubleshooting

-   If the overlay doesn't appear, check the console logs for errors.
-   Ensure the application has permissions to capture the screen.
