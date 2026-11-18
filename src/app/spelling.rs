//! Spelling passwords out loud with the NATO phonetic alphabet, for reading them over the phone.

/// What kind of characters a run of the spelled-out password contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Lowercase,
    Uppercase,
    Digit,
    Symbol,
}

/// A run of characters of the same kind, spelled out word by word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub kind: Kind,
    pub words: Vec<String>,
}

const NATO: [&str; 26] = [
    "alfa", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india", "juliett", "kilo", "lima",
    "mike", "november", "oscar", "papa", "quebec", "romeo", "sierra", "tango", "uniform", "victor", "whiskey",
    "xray", "yankee", "zulu",
];

const DIGITS: [&str; 10] = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine"];

/// Splits `password` into runs of the same kind, as you'd read it out:
/// `asd2` becomes "alfa sierra delta" (lowercase) and "two" (digit).
pub fn spell_out(password: &str) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for c in password.chars() {
        let (kind, word) = spell(c);
        match runs.last_mut() {
            Some(run) if run.kind == kind => run.words.push(word),
            _ => runs.push(Run { kind, words: vec![word] }),
        }
    }
    runs
}

/// The kind of a character and how to say it.
fn spell(c: char) -> (Kind, String) {
    if c.is_ascii_lowercase() {
        (Kind::Lowercase, NATO[(c as u8 - b'a') as usize].to_string())
    } else if c.is_ascii_uppercase() {
        (Kind::Uppercase, NATO[(c as u8 - b'A') as usize].to_string())
    } else if c.is_ascii_digit() {
        (Kind::Digit, DIGITS[(c as u8 - b'0') as usize].to_string())
    } else {
        (Kind::Symbol, symbol_name(c))
    }
}

/// The spoken name of a symbol.
fn symbol_name(c: char) -> String {
    let name = match c {
        '!' => "exclamation mark",
        '@' => "at",
        '#' => "hash",
        '$' => "dollar",
        '%' => "percent",
        '&' => "ampersand",
        '*' => "asterisk",
        '+' => "plus",
        '=' => "equals",
        '?' => "question mark",
        '-' => "dash",
        '_' => "underscore",
        '.' => "dot",
        ',' => "comma",
        ':' => "colon",
        ';' => "semicolon",
        '/' => "slash",
        '\\' => "backslash",
        '|' => "vertical bar",
        '^' => "caret",
        '~' => "tilde",
        '`' => "backtick",
        '\'' => "apostrophe",
        '"' => "double quote",
        '(' => "open bracket",
        ')' => "close bracket",
        '[' => "open square bracket",
        ']' => "close square bracket",
        '{' => "open curly bracket",
        '}' => "close curly bracket",
        '<' => "less than",
        '>' => "greater than",
        ' ' => "space",
        other => return format!("\"{}\"", other),
    };
    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(kind: Kind, words: &[&str]) -> Run {
        Run { kind, words: words.iter().map(|w| w.to_string()).collect() }
    }

    #[test]
    fn groups_runs_of_the_same_kind() {
        assert_eq!(
            spell_out("asd2qwe+YUI6"),
            vec![
                run(Kind::Lowercase, &["alfa", "sierra", "delta"]),
                run(Kind::Digit, &["two"]),
                run(Kind::Lowercase, &["quebec", "whiskey", "echo"]),
                run(Kind::Symbol, &["plus"]),
                run(Kind::Uppercase, &["yankee", "uniform", "india"]),
                run(Kind::Digit, &["six"]),
            ]
        );
    }

    #[test]
    fn spells_every_character_the_generators_use() {
        let all = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%&*+=?-";
        for run in spell_out(all) {
            assert!(run.words.iter().all(|w| !w.starts_with('"')), "{:?}", run);
        }
        assert_eq!(spell_out("€")[0].words, vec!["\"€\""]);
    }
}
