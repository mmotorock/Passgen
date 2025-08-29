# Passgen - A Rust-Based Password Generator

Passgen is a versatile, secure password and passphrase generator built with Rust and the `egui` library. It provides both a user-friendly graphical interface (GUI) and a scriptable command-line interface (CLI) for all your password needs.



## Features

-   **Dual Mode:** Operates as a full GUI application or as a command-line tool.
-   **Character-Based Generation:**
    -   Generate strong, random passwords of a specified length (min 12 characters).
    -   Customize which character sets to use (lowercase, uppercase, numbers, special).
    -   Enforces a maximum repetition of 3 for any single character.
-   **Word-Based Generation (Passphrases):**
    -   Create memorable passphrases using a list of words.
    -   Choose between 3, 4, or 5 words.
    -   Customize capitalization and the separator character.
-   **Full Configuration:**
    -   All settings are saved to a `config.toml` file.
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

Simply double-click the executable to launch the graphical user interface. The application will automatically create a default `config.toml` and `words.txt` file in the same directory if they don't exist.

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