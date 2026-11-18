# Passgen - A Rust-Based Password Generator

Passgen is a versatile, secure password and passphrase generator built with Rust and the `egui` library. It provides both a user-friendly graphical interface (GUI) and a scriptable command-line interface (CLI) for all your password needs.



## Features

-   **Dual Mode:** Operates as a full GUI application or as a command-line tool.
-   **Character-Based Generation:**
    -   Generate strong, random passwords from 12 up to 256 characters.
    -   Customize which character sets to use (lowercase, uppercase, numbers, special).
    -   No character appears twice in a row, and each character appears at most 3 times (raised automatically only for very long passwords).
-   **Word-Based Generation (Passphrases):**
    -   Create memorable passphrases using a list of words.
    -   Choose between 3, 4, or 5 words.
    -   Customize capitalization and the separator character.
-   **Password History:**
    -   The last 5 generated passwords (GUI and CLI) are listed at the bottom of the window, hidden until you tick "Show", each with a Copy button.
    -   The history is stored encrypted (ChaCha20-Poly1305) in `history.dat`. The key is kept in the operating system's keyring (Windows Credential Manager, GNOME Keyring/KWallet on Linux, Keychain on macOS), so the file alone is useless to anyone who copies it.
    -   If no keyring is available, the history is kept for the current session only.
-   **Quick to Use:**
    -   A password is ready as soon as the app opens.
    -   Generating a password copies it to the clipboard automatically. The clipboard is cleared after 2 minutes (only if it still holds that password).
    -   Keyboard shortcuts: `Space` generates a new password, `Ctrl+C` copies the one on screen.
    -   Shows an estimated time to crack each password, assuming an attacker making 100 billion guesses per second.
-   **Full Configuration:**
    -   All settings are saved automatically to a `config.toml` file as you change them.
    -   Choose which tab (Characters or Words) the app opens on.
    -   Custom wordlists can be used by changing the path in the settings.
    -   Supports both Light and Dark themes.
-   **Cross-Platform:** Built with Rust, it compiles to a single, native executable.
-   **Secure:** Uses the `rand` crate for cryptographically secure random number generation.

## Prerequisites

To build this project from the source, you will need:

1.  **The Rust Toolchain:** Install Rust from [rustup.rs](https://rustup.rs/).
2.  **Windows Build Tools (for Windows users):**
    -   You must use the **MSVC toolchain**. You can install it by running `rustup toolchain install stable-msvc`.
    -   You also need the **Visual Studio Build Tools** with the "Desktop development with C++" workload and the latest **Windows SDK** component installed. This is required to compile resources and embed the application icon.

## Installation & Running

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/mmotorock/Passgen.git
    ```
   
2.  **Navigate to the project directory:**
    ```bash
    cd Passgen
    ```

3.  **Build the project in release mode:**
    This command will compile the application with full optimizations and create a standalone executable.
    ```bash
    cargo build --release
    ```

4.  **Run the application:**
    -   The final executable will be located in the `target/release/` directory.
    -   On Windows, this will be `Passgen.exe`.

## Usage

### GUI Mode

Simply double-click the executable to launch the graphical user interface. The application creates a default `config.toml` on first launch.

All settings, the wordlist and the history live in the config folder:

| Platform | Config folder |
|---|---|
| Windows | `config\` next to `Passgen.exe` |
| Linux / macOS | `~/.config/passgen/` (or `$XDG_CONFIG_HOME/passgen/`) |

Copy `words.txt` from this repository into that folder to use the word generator.

### Command-Line Mode (CLI)

You can also run the application from your terminal to quickly generate and copy a password.

-   **Generate a 16-character password:**
    ```bash
    ./Passgen -n 16
    ```

-   **Generate a 4-word passphrase:**
    ```bash
    ./Passgen -w 4
    ```

-   **View the help manual:**
    ```bash
    ./Passgen --help
    ```