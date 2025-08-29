use eframe::egui;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

/// Gets the directory where the application executable is located.
pub fn get_app_directory() -> PathBuf {
    if let Some(dir) = dirs::executable_dir() {
        dir
    } else {
        std::env::current_dir().unwrap_or_default()
    }
}

/// Loads the icon data from the embedded `icon.ico` file.
pub fn load_icon() -> Arc<egui::IconData> {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(include_bytes!("../../icon.ico"))
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    Arc::new(egui::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    })
}

/// Loads words from the specified file path.
pub fn load_words_from_file(path: &PathBuf) -> (Vec<String>, String) {
    if let Ok(content) = fs::read_to_string(path) {
        let words: Vec<String> = content
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();

        if !words.is_empty() {
            return (words, "Loaded words from words.txt.".to_string());
        }
    }
    (vec![], "words.txt not found or is empty.".to_string())
}

/// Saves the generated password to the specified file path.
pub fn save_password_to_file(password: &str, path: &PathBuf) -> String {
    match fs::write(path, password) {
        Ok(_) => "Password generated and saved to password.txt".to_string(),
        Err(e) => format!("Error saving password.txt: {}", e),
    }
}
