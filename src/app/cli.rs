use crate::app::config::load_config_map;
use crate::app::password::{generate_char_password, generate_word_password};
use crate::app::state::CharacterSets;
use crate::app::utils::{get_app_directory, load_words_from_file};
use arboard::Clipboard;

/// Launches the command-line interface for character passwords.
pub fn run_char_cli_mode(length: usize) {
    let app_dir = get_app_directory();
    let config_path = app_dir.join("config.toml");
    let config = load_config_map(&config_path);
    let sets = CharacterSets {
        lowercase: config.get("lowercase_chars").cloned().unwrap_or_default(),
        uppercase: config.get("uppercase_chars").cloned().unwrap_or_default(),
        numbers: config.get("number_chars").cloned().unwrap_or_default(),
        special: config.get("special_chars").cloned().unwrap_or_default(),
    };

    match generate_char_password(length, &sets, true, true, true, true) {
        Ok(password) => {
            println!("{}", password);
            if let Ok(mut clipboard) = Clipboard::new() {
                if clipboard.set_text(password).is_ok() {
                    eprintln!("Password copied to clipboard.");
                }
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

/// Launches the command-line interface for word passphrases.
pub fn run_word_cli_mode(count: usize) {
    if !(3..=5).contains(&count) {
        eprintln!("Error: Word count for -w flag must be 3, 4, or 5.");
        return;
    }

    let app_dir = get_app_directory();
    let words_file_path = app_dir.join("words.txt");
    let (words, _) = load_words_from_file(&words_file_path);

    match generate_word_password(count, &words, true, "-", true) {
        Ok(password) => {
            println!("{}", password);
            if let Ok(mut clipboard) = Clipboard::new() {
                if clipboard.set_text(password).is_ok() {
                    eprintln!("Passphrase copied to clipboard.");
                }
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
