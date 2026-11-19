use crate::app::config::{Config, CONFIG_FILE};
use crate::app::history::History;
use crate::app::state::{CliArgs, Style};
use crate::app::styles;
use crate::app::utils::{get_config_directory, load_words};
use arboard::Clipboard;
use std::process::ExitCode;

/// Runs the command-line interface: generates `args.count` passwords in the chosen style from
/// the saved settings, with any options in `args` applied on top, and prints one per line.
pub fn run_cli(args: &CliArgs) -> ExitCode {
    let mut config = load_config();
    let style = resolve_style(args, config.default_style);
    if let Err(e) = apply_options(style, args, &mut config) {
        eprintln!("error: {}", e);
        return ExitCode::from(2);
    }

    let words = if style == Style::Words {
        let path = args.wordlist.clone().unwrap_or_else(|| config.words_file_path.clone());
        let (words, word_status) = load_words(&path);
        if words.is_empty() {
            return fail(&word_status);
        }
        words
    } else {
        Vec::new()
    };
    if let Some(problem) = styles::problem(style, &config, words.len()) {
        return fail(&problem);
    }

    let mut passwords = Vec::new();
    for _ in 0..args.count {
        match styles::generate(style, &config, &words) {
            Ok(password) => passwords.push(password),
            Err(e) => return fail(&e),
        }
    }
    for password in &passwords {
        println!("{}", password);
    }

    if !args.no_history {
        remember(&passwords);
    }
    if !args.no_copy {
        copy(&passwords.join("\n"), passwords.len());
    }
    ExitCode::SUCCESS
}

/// The style to generate: the one asked for, or else one that fits the size option given,
/// preferring the default style from the settings.
fn resolve_style(args: &CliArgs, default: Style) -> Style {
    if let Some(style) = args.style {
        return style;
    }
    let fits = |options: &[Style], fallback: Style| if options.contains(&default) { default } else { fallback };
    if args.words.is_some() {
        Style::Words
    } else if args.groups.is_some() {
        fits(&[Style::Grouped, Style::Pronounceable], Style::Grouped)
    } else if args.length.is_some() {
        fits(&[Style::Random, Style::Segmented, Style::Pin], Style::Segmented)
    } else {
        default
    }
}

/// Applies the command-line options to the settings for one run, or explains why an option
/// doesn't fit `style`.
fn apply_options(style: Style, args: &CliArgs, config: &mut Config) -> Result<(), String> {
    use Style::*;

    // Each style takes exactly one kind of size option.
    let size_flag = match style {
        Random | Segmented | Pin => "-n/--length",
        Grouped | Pronounceable => "-g/--groups",
        Words => "-w/--words",
    };
    for (given, flag) in [
        (args.length.is_some(), "-n/--length"),
        (args.groups.is_some(), "-g/--groups"),
        (args.words.is_some(), "-w/--words"),
    ] {
        if given && flag != size_flag {
            return Err(format!("{} doesn't apply to the {} style; use {}", flag, cli_name(style), size_flag));
        }
    }
    let size = args.length.or(args.groups).or(args.words);
    if let Some(size) = size {
        match style {
            Random => config.characters.length = size,
            Segmented => config.segmented.length = size,
            Pin => config.pin.length = size,
            Grouped => config.grouped.groups = size,
            Pronounceable => config.pronounceable.groups = size,
            Words => config.words.count = size,
        }
    }

    // Options that only some styles have.
    let options: [(bool, &str, &[Style]); 11] = [
        (args.no_lowercase, "--no-lowercase", &[Random, Segmented]),
        (args.no_uppercase, "--no-uppercase", &[Random, Segmented]),
        (args.no_numbers, "--no-numbers", &[Random, Segmented]),
        (args.no_special, "--no-special", &[Random, Segmented]),
        (args.number, "--number", &[Pronounceable, Words]),
        (args.no_number, "--no-number", &[Pronounceable, Words]),
        (args.separator.is_some(), "--separator", &[Words]),
        (args.no_separator, "--no-separator", &[Words]),
        (args.capitalize, "--capitalize", &[Words]),
        (args.no_capitalize, "--no-capitalize", &[Words]),
        (args.wordlist.is_some(), "--wordlist", &[Words]),
    ];
    for (given, flag, styles) in options {
        if given && !styles.contains(&style) {
            let names: Vec<&str> = styles.iter().map(|s| cli_name(*s)).collect();
            return Err(format!(
                "{} only applies to the {} style{}, not {}",
                flag,
                names.join(" and "),
                if names.len() > 1 { "s" } else { "" },
                cli_name(style)
            ));
        }
    }

    // These only turn sets off: a set turned off in the settings stays off.
    let sets = &mut config.characters;
    sets.use_lowercase &= !args.no_lowercase;
    sets.use_uppercase &= !args.no_uppercase;
    sets.use_numbers &= !args.no_numbers;
    sets.use_special &= !args.no_special;

    if args.number || args.no_number {
        match style {
            Pronounceable => config.pronounceable.include_number = args.number,
            _ => config.words.include_number = args.number,
        }
    }
    if let Some(separator) = args.separator {
        config.words.use_separator = true;
        config.words.separator = separator.to_string();
    }
    if args.no_separator {
        config.words.use_separator = false;
    }
    if args.capitalize || args.no_capitalize {
        config.words.capitalize = args.capitalize;
    }
    Ok(())
}

/// The style's name as typed on the command line.
fn cli_name(style: Style) -> &'static str {
    match style {
        Style::Random => "random",
        Style::Segmented => "segmented",
        Style::Grouped => "grouped",
        Style::Pronounceable => "pronounceable",
        Style::Pin => "pin",
        Style::Words => "words",
    }
}

/// Loads the shared settings, reporting anything noteworthy on stderr.
fn load_config() -> Config {
    let (config, messages) = Config::load(&get_config_directory().join(CONFIG_FILE));
    for message in messages {
        eprintln!("{}", message);
    }
    config
}

/// Reports an error on stderr.
fn fail(message: &str) -> ExitCode {
    eprintln!("Error: {}", message);
    ExitCode::FAILURE
}

/// Adds CLI-generated passwords to the shared history, reporting problems on stderr.
fn remember(passwords: &[String]) {
    let (mut history, warning) = History::load(get_config_directory().join("history.dat"));
    if let Some(warning) = warning {
        eprintln!("{}", warning);
    }
    for password in passwords {
        if let Err(e) = history.add(password) {
            eprintln!("Saving history failed: {}", e);
            return;
        }
    }
}

/// Copies `text` (one or more passwords, one per line) to the clipboard.
fn copy(text: &str, count: usize) {
    let copied = Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text));
    match copied {
        Ok(()) if count == 1 => eprintln!("Copied to clipboard."),
        Ok(()) => eprintln!("Copied all {} to clipboard.", count),
        Err(e) => eprintln!("Could not copy to clipboard: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, Parser};

    fn parse(args: &str) -> CliArgs {
        CliArgs::try_parse_from(std::iter::once("passgen").chain(args.split_whitespace())).unwrap()
    }

    /// The settings a command line would generate with, starting from `config`.
    fn run_settings(args: &str, mut config: Config) -> Result<(Style, Config), String> {
        let args = parse(args);
        let style = resolve_style(&args, config.default_style);
        apply_options(style, &args, &mut config).map(|()| (style, config))
    }

    #[test]
    fn argument_definitions_are_consistent() {
        CliArgs::command().debug_assert();
    }

    #[test]
    fn picks_a_style_that_fits_the_size_option() {
        let mut config = Config::default(); // default style: segmented
        assert_eq!(run_settings("-n 16", config.clone()).unwrap().0, Style::Segmented);
        assert_eq!(run_settings("-g 3", config.clone()).unwrap().0, Style::Grouped);
        assert_eq!(run_settings("-w 4", config.clone()).unwrap().0, Style::Words);
        assert_eq!(run_settings("-s pin -n 6", config.clone()).unwrap().0, Style::Pin);
        config.default_style = Style::Pin;
        assert_eq!(run_settings("-n 8", config.clone()).unwrap().0, Style::Pin);
        assert_eq!(run_settings("-s random", config).unwrap().0, Style::Random);
    }

    #[test]
    fn styles_can_be_given_by_number() {
        for (position, style) in Style::ALL.into_iter().enumerate() {
            let number = (position + 1).to_string();
            assert_eq!(parse(&format!("-s {}", number)).style, Some(style), "-s {}", number);
            assert_eq!(parse(&format!("-s {}", cli_name(style))).style, Some(style));
        }
        assert!(CliArgs::try_parse_from(["passgen", "-s", "7"]).is_err());
        assert!(CliArgs::try_parse_from(["passgen", "-s", "0"]).is_err());
    }

    #[test]
    fn applies_the_size_to_the_chosen_style() {
        let (_, config) = run_settings("-s pin -n 8", Config::default()).unwrap();
        assert_eq!(config.pin.length, 8);
        assert_eq!(config.segmented.length, Config::default().segmented.length);
        let (_, config) = run_settings("-s pronounceable -g 6 --no-number", Config::default()).unwrap();
        assert_eq!(config.pronounceable.groups, 6);
        assert!(!config.pronounceable.include_number);
    }

    #[test]
    fn character_flags_only_turn_sets_off() {
        let mut saved = Config::default();
        saved.characters.use_numbers = false;
        let (_, config) = run_settings("-s random -n 20 --no-special", saved).unwrap();
        assert!(config.characters.use_lowercase && config.characters.use_uppercase);
        assert!(!config.characters.use_numbers, "a set turned off in the settings stays off");
        assert!(!config.characters.use_special);
    }

    #[test]
    fn word_flags_override_saved_settings() {
        let mut saved = Config::default();
        saved.words.include_number = true;
        let (_, config) = run_settings("-w 5 --separator . --no-number --no-capitalize", saved).unwrap();
        let words = config.words;
        assert_eq!((words.count, words.separator.as_str()), (5, "."));
        assert!(words.use_separator && !words.include_number && !words.capitalize);
    }

    #[test]
    fn rejects_options_that_dont_fit_the_style() {
        for args in ["-s pin -g 3", "-s grouped -n 16", "-s pin --no-special", "-n 16 --number", "-w 4 --no-special", "-s grouped --separator ."] {
            assert!(run_settings(args, Config::default()).is_err(), "{:?} should be rejected", args);
        }
    }

    #[test]
    fn clap_rejects_contradictions() {
        for args in ["--number", "-c 3", "-n 16 -w 4", "-w 4 --number --no-number", "-s nonsense"] {
            let parsed = CliArgs::try_parse_from(std::iter::once("passgen").chain(args.split_whitespace()));
            assert!(parsed.is_err(), "{:?} should be rejected", args);
        }
    }
}
