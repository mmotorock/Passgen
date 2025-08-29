use crate::app::state::CharacterSets;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;

/// Generates a password from random characters.
pub fn generate_char_password(
    length: usize,
    sets: &CharacterSets,
    use_lowercase: bool,
    use_uppercase: bool,
    use_numbers: bool,
    use_special: bool,
) -> Result<String, String> {
    let mut password_chars: Vec<char> = Vec::new();
    let mut all_chars: Vec<char> = Vec::new();
    let mut selected_categories = 0;

    if use_lowercase {
        let chars: Vec<char> = sets.lowercase.chars().collect();
        if !chars.is_empty() {
            password_chars.push(*chars.choose(&mut thread_rng()).unwrap());
            all_chars.extend(chars);
            selected_categories += 1;
        }
    }
    if use_uppercase {
        let chars: Vec<char> = sets.uppercase.chars().collect();
        if !chars.is_empty() {
            password_chars.push(*chars.choose(&mut thread_rng()).unwrap());
            all_chars.extend(chars);
            selected_categories += 1;
        }
    }
    if use_numbers {
        let chars: Vec<char> = sets.numbers.chars().collect();
        if !chars.is_empty() {
            password_chars.push(*chars.choose(&mut thread_rng()).unwrap());
            all_chars.extend(chars);
            selected_categories += 1;
        }
    }
    if use_special {
        let chars: Vec<char> = sets.special.chars().collect();
        if !chars.is_empty() {
            password_chars.push(*chars.choose(&mut thread_rng()).unwrap());
            all_chars.extend(chars);
            selected_categories += 1;
        }
    }

    if selected_categories == 0 {
        return Err("You must select at least one character set.".to_string());
    }
    if length < selected_categories {
        return Err(format!(
            "Password length must be at least {} to include one of each selected type.",
            selected_categories
        ));
    }

    let unique_chars_count = all_chars.iter().collect::<std::collections::HashSet<_>>().len();
    if length > unique_chars_count * 3 {
        return Err(
            "Cannot generate: not enough unique characters for the requested length and repetition rule.".to_string()
        );
    }

    for _ in 0..100 {
        let mut candidate_chars = password_chars.clone();
        let remaining_length = length - candidate_chars.len();

        if remaining_length > 0 {
            candidate_chars.extend(all_chars.choose_multiple(&mut thread_rng(), remaining_length));
        }

        candidate_chars.shuffle(&mut thread_rng());

        let mut counts = HashMap::new();
        let mut is_valid = true;
        for &char in &candidate_chars {
            let count = counts.entry(char).or_insert(0);
            *count += 1;
            if *count > 3 {
                is_valid = false;
                break;
            }
        }

        if is_valid {
            return Ok(candidate_chars.into_iter().collect());
        }
    }

    Err("Failed to generate a valid password after multiple attempts.".to_string())
}

/// Generates a passphrase from a list of words, respecting length constraints.
pub fn generate_word_password(
    count: usize,
    words: &[String],
    use_separator: bool,
    separator: &str,
    use_uppercase: bool,
) -> Result<String, String> {
    if words.len() < count {
        return Err(format!(
            "Not enough words in words.txt (found {}, need at least {}).",
            words.len(),
            count
        ));
    }

    let max_length = match count {
        3 => 30,
        4 => 35,
        5 => 40,
        _ => return Err("Invalid word count.".to_string()),
    };

    for _ in 0..100 {
        let chosen_words: Vec<&String> = words.choose_multiple(&mut thread_rng(), count).collect();

        let processed_words: Vec<String> = chosen_words
            .iter()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        if use_uppercase {
                            first.to_uppercase().to_string() + &word[first.len_utf8()..]
                        } else {
                            first.to_lowercase().to_string() + &word[first.len_utf8()..]
                        }
                    }
                }
            })
            .collect();

        let separator_str = if use_separator { separator } else { "" };
        let passphrase = processed_words.join(separator_str);

        if passphrase.len() <= max_length {
            return Ok(passphrase);
        }
    }

    Err(format!(
        "Could not generate a passphrase under {} characters. Check words.txt for long words.",
        max_length
    ))
}
