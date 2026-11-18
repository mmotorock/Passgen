use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

/// The wordlist built into the executable, written to the config folder on first launch.
const DEFAULT_WORDS: &str = include_str!("../../words.txt");

/// Gets the directory holding config.toml, words.txt and the history, creating it if needed.
/// On Windows this is a `config` folder next to the executable; elsewhere it is
/// `~/.config/passgen` (or `$XDG_CONFIG_HOME/passgen`).
pub fn get_config_directory() -> PathBuf {
    let dir = if cfg!(windows) {
        std::env::current_exe().ok().and_then(|exe| exe.parent().map(|parent| parent.join("config")))
    } else {
        dirs::config_dir().map(|config| config.join("passgen"))
    }
    .unwrap_or_else(|| PathBuf::from("config"));

    let _ = fs::create_dir_all(&dir);
    dir
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

/// Creates the default words.txt file if it doesn't exist.
pub fn create_default_words_file(path: &PathBuf) -> String {
    if !path.exists() {
        match File::create(path) {
            Ok(mut file) => match file.write_all(DEFAULT_WORDS.as_bytes()) {
                Ok(_) => "Created default words.txt".to_string(),
                Err(e) => format!("Failed to create default words.txt: {}", e),
            },
            Err(e) => format!("Failed to create default words.txt: {}", e),
        }
    } else {
        String::new() // No message if file already exists
    }
}

