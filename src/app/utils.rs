use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

const DEFAULT_WORDS: &[&str] = &[
    "apple", "banana", "cherry", "dragon", "eagle", "forest", "guitar", "harmony", "island", "journey",
    "kinetic", "lantern", "mountain", "nebula", "ocean", "puzzle", "quantum", "river", "sunrise", "telescope",
    "universe", "vortex", "waterfall", "xenon", "yellow", "zeppelin",
];

/// Gets the directory where the application executable is located.
/// Falls back to the current working directory if the executable path cannot be determined.
pub fn get_app_directory() -> PathBuf {
    dirs::executable_dir().unwrap_or_else(|| std::env::current_dir().unwrap_or(PathBuf::from(".")))
}

/// Loads an icon from the embedded icon file.
pub fn load_icon() -> egui::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let icon = include_bytes!("../../icon.ico");
        let image = image::load_from_memory(icon)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    egui::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

/// Loads a list of words from a specified text file.
pub fn load_words_from_file(path: &PathBuf) -> (Vec<String>, String) {
    if !path.exists() {
        return (Vec::new(), format!("Wordlist not found at: {:?}", path));
    }
    match fs::read_to_string(path) {
        Ok(content) => {
            let words: Vec<String> = content.lines().map(String::from).collect();
            let status = format!("Successfully loaded {} words from {:?}", words.len(), path.file_name().unwrap_or_default());
            (words, status)
        }
        Err(e) => (Vec::new(), format!("Failed to read wordlist: {}", e)),
    }
}

/// Saves the generated password to password.txt
pub fn save_password_to_file(password: &str, path: &PathBuf) -> String {
    match File::create(path) {
        Ok(mut file) => match file.write_all(password.as_bytes()) {
            Ok(_) => "Password saved to password.txt".to_string(),
            Err(e) => format!("Error saving password: {}", e),
        },
        Err(e) => format!("Error creating password.txt: {}", e),
    }
}

/// Creates the default words.txt file if it doesn't exist.
pub fn create_default_words_file(path: &PathBuf) -> String {
    if !path.exists() {
        let default_content = DEFAULT_WORDS.join("\n");
        match File::create(path) {
            Ok(mut file) => match file.write_all(default_content.as_bytes()) {
                Ok(_) => "Created default words.txt".to_string(),
                Err(e) => format!("Failed to create default words.txt: {}", e),
            },
            Err(e) => format!("Failed to create default words.txt: {}", e),
        }
    } else {
        String::new() // No message if file already exists
    }
}

