use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// How many generated passwords are remembered.
pub const MAX_ENTRIES: usize = 5;

const KEYRING_SERVICE: &str = "passgen";
const KEYRING_USER: &str = "history-key";
const NONCE_LEN: usize = 12;

/// A single remembered password.
#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub password: String,
    pub created: String,
}

/// The most recently generated passwords, newest first.
///
/// The history file is encrypted with ChaCha20-Poly1305. The key is stored in
/// the operating system's keyring, never next to the file, so copying the
/// config folder alone does not reveal any passwords.
#[derive(Default)]
pub struct History {
    pub entries: Vec<HistoryEntry>,
    path: PathBuf,
    cipher: Option<ChaCha20Poly1305>,
}

impl History {
    /// Loads the history from `path`, returning it along with a status message.
    /// If the keyring is unavailable, the history works for this session only.
    pub fn load(path: PathBuf) -> (Self, String) {
        let cipher = match load_or_create_key() {
            Ok(key) => ChaCha20Poly1305::new(&key),
            Err(e) => {
                let history = Self { path, ..Default::default() };
                return (history, format!("Keyring unavailable, history will not be saved: {}", e));
            }
        };

        let mut history = Self { entries: Vec::new(), path, cipher: Some(cipher) };
        let status = match history.read_file() {
            Ok(0) => String::new(),
            Ok(count) => format!("Loaded {} passwords from history", count),
            Err(e) => format!("Could not read history, starting fresh: {}", e),
        };
        (history, status)
    }

    /// Adds a password to the front of the history and saves it.
    /// Returns a status message for the status bar.
    pub fn add(&mut self, password: &str) -> String {
        self.entries.insert(
            0,
            HistoryEntry {
                password: password.to_string(),
                created: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
            },
        );
        self.entries.truncate(MAX_ENTRIES);
        match self.save() {
            Ok(true) => "Password generated and added to history.".to_string(),
            Ok(false) => "Password generated (history is not saved this session).".to_string(),
            Err(e) => format!("Password generated, but saving history failed: {}", e),
        }
    }

    /// Removes all entries and deletes the history file.
    pub fn clear(&mut self) -> String {
        self.entries.clear();
        match fs::remove_file(&self.path) {
            Ok(_) => "History cleared.".to_string(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => "History cleared.".to_string(),
            Err(e) => format!("Error deleting history file: {}", e),
        }
    }

    /// Encrypts and writes the history. Returns `Ok(false)` if there is no key to save with.
    fn save(&self) -> Result<bool, String> {
        let Some(cipher) = &self.cipher else {
            return Ok(false);
        };
        let plaintext = serde_json::to_vec(&self.entries).map_err(|e| e.to_string())?;
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_slice())
            .map_err(|_| "encryption failed".to_string())?;

        let mut data = nonce.to_vec();
        data.extend(ciphertext);
        fs::write(&self.path, data).map_err(|e| e.to_string())?;
        Ok(true)
    }

    /// Decrypts the history file into `entries`. Returns how many entries were loaded.
    fn read_file(&mut self) -> Result<usize, String> {
        let Some(cipher) = &self.cipher else {
            return Ok(0);
        };
        let data = match fs::read(&self.path) {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(e.to_string()),
        };
        if data.len() < NONCE_LEN {
            return Err("history file is corrupted".to_string());
        }

        let (nonce, ciphertext) = data.split_at(NONCE_LEN);
        let plaintext = cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|_| "file is corrupted or was encrypted with a different key".to_string())?;
        let mut entries: Vec<HistoryEntry> = serde_json::from_slice(&plaintext).map_err(|e| e.to_string())?;
        entries.truncate(MAX_ENTRIES);

        self.entries = entries;
        Ok(self.entries.len())
    }
}

/// Reads the encryption key from the OS keyring, creating one on first use.
fn load_or_create_key() -> Result<Key, String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| e.to_string())?;
    match entry.get_secret() {
        Ok(secret) if secret.len() == 32 => Ok(*Key::from_slice(&secret)),
        Ok(_) => Err("stored key has the wrong length".to_string()),
        Err(keyring::Error::NoEntry) => {
            let key = ChaCha20Poly1305::generate_key(&mut OsRng);
            entry.set_secret(&key).map_err(|e| e.to_string())?;
            Ok(key)
        }
        Err(e) => Err(e.to_string()),
    }
}
