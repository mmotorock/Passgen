use crate::app::config::{CharacterSettings, Config, WordSettings, CONFIG_FILE};
use crate::app::history::History;
use crate::app::password::{
    generate_char_password, generate_word_password, MAX_LENGTH, MAX_WORDS, MIN_LENGTH, MIN_WORDS,
};
use crate::app::utils::{get_config_directory, load_words};
use arboard::Clipboard;

/// Launches the command-line interface for character passwords, using the saved settings.
pub fn run_char_cli_mode(length: usize) {
    if !(MIN_LENGTH..=MAX_LENGTH).contains(&length) {
        eprintln!("Error: Length for -n flag must be between {} and {}.", MIN_LENGTH, MAX_LENGTH);
        return;
    }

    let config = load_config();
    let settings = CharacterSettings { length, ..config.characters };
    output(generate_char_password(&settings), "Password");
}

/// Launches the command-line interface for word passphrases, using the saved settings.
pub fn run_word_cli_mode(count: usize) {
    if !(MIN_WORDS..=MAX_WORDS).contains(&count) {
        eprintln!("Error: Word count for -w flag must be between {} and {}.", MIN_WORDS, MAX_WORDS);
        return;
    }

    let config = load_config();
    let (words, word_status) = load_words(&config.words_file_path);
    if words.is_empty() {
        eprintln!("{}", word_status);
    }
    let settings = WordSettings { count, ..config.words };
    output(generate_word_password(&settings, &words), "Passphrase");
}

/// Loads the shared settings, reporting anything noteworthy on stderr.
fn load_config() -> Config {
    let (config, messages) = Config::load(&get_config_directory().join(CONFIG_FILE));
    for message in messages {
        eprintln!("{}", message);
    }
    config
}

/// Prints a generated password, then records it in the history and copies it.
fn output(result: Result<String, String>, kind: &str) {
    match result {
        Ok(password) => {
            println!("{}", password);
            remember(&password);
            if let Ok(mut clipboard) = Clipboard::new() {
                if clipboard.set_text(password).is_ok() {
                    eprintln!("{} copied to clipboard.", kind);
                }
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

/// Adds a CLI-generated password to the shared history, reporting problems on stderr.
fn remember(password: &str) {
    let (mut history, warning) = History::load(get_config_directory().join("history.dat"));
    if let Some(warning) = warning {
        eprintln!("{}", warning);
    }
    if let Err(e) = history.add(password) {
        eprintln!("Saving history failed: {}", e);
    }
}
