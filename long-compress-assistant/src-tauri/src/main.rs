#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    println!("Hello, Tauri!");
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
