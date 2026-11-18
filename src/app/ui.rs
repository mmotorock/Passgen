use crate::app::config::{CharacterSettings, Config};
use crate::app::password::{generate_char_password, generate_word_password, MAX_LENGTH, MAX_WORDS, MIN_LENGTH, MIN_WORDS};
use crate::app::state::{PassGenApp, Tab, Theme};
use crate::app::history::MAX_ENTRIES;
use crate::app::utils::{load_icon, load_words};
use eframe::{egui, NativeOptions};
use rfd::FileDialog;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const PADDING: f32 = 10.0;
const PANEL_MARGIN: f32 = 8.0; // egui's default inner margin of the central panel
const MIN_WINDOW_WIDTH: f32 = 480.0;
const LENGTH_BOX_WIDTH: f32 = 40.0;
const COPY_BUTTON_SIZE: egui::Vec2 = egui::vec2(26.0, 22.0);
const CLIPBOARD_CLEAR_AFTER: Duration = Duration::from_secs(120);

impl eframe::App for PassGenApp {
    /// Called each frame to draw the GUI.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply the selected theme
        ctx.set_visuals(match self.config.theme {
            Theme::Light => egui::Visuals::light(),
            Theme::Dark => egui::Visuals::dark(),
        });

        // Handled before any widget so Space doesn't also press a focused button.
        self.handle_shortcuts(ctx);

        // --- Top Menu Bar ---
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Settings", |ui| {
                    if ui.button("Open Settings").clicked() {
                        self.show_settings_window = true;
                        ui.close_menu();
                    }
                    if ui.button("Restore Defaults").clicked() {
                        // Keep the theme and wordlist; reset everything else.
                        self.config = Config {
                            theme: self.config.theme,
                            words_file_path: self.config.words_file_path.clone(),
                            ..Config::default()
                        };
                        self.status_message = "Default settings restored.".to_string();
                        ui.close_menu();
                    }
                });
            });
        });

        // --- Status Bar (bottom panels must be added before the central panel) ---
        let status_bar = egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add_space(PADDING);
                ui.label(egui::RichText::new(&self.status_message).italics());
            });
        });

        let content_bottom = egui::CentralPanel::default()
            .show(ctx, |ui| {
                // --- Tab Selection ---
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.active_tab, Tab::Character, "Characters");
                    ui.selectable_value(&mut self.active_tab, Tab::Word, "Words");
                });
                ui.separator();

                // --- Render Active Tab ---
                match self.active_tab {
                    Tab::Character => self.draw_character_tab(ui),
                    Tab::Word => self.draw_word_tab(ui),
                }

                // --- History, directly under the generated password ---
                ui.add_space(PADDING);
                ui.separator();
                self.draw_history_panel(ui);
                ui.cursor().min.y
            })
            .inner;
        self.fit_window_to_content(ctx, content_bottom + PANEL_MARGIN + status_bar.response.rect.height());

        // --- Draw Windows ---
        self.draw_settings_window(ctx);
        self.draw_error_dialog(ctx);

        // Settings are saved as soon as they change, so there is nothing to confirm on exit.
        if self.config != self.saved_config {
            if let Err(e) = self.config.save(&self.config_path) {
                self.status_message = e;
            }
            self.saved_config = self.config.clone(); // Don't retry a failed save every frame
        }
        self.clear_clipboard_when_due(ctx);
    }
}

impl PassGenApp {
    /// Draws the UI for the "Characters" tab.
    fn draw_character_tab(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("char_sets_grid")
            .num_columns(2)
            .spacing([PADDING, PADDING])
            .show(ui, |ui| {
                ui.checkbox(&mut self.config.characters.use_lowercase, "Lowercase");
                ui.add(egui::TextEdit::singleline(&mut self.config.characters.lowercase).desired_width(f32::INFINITY));
                ui.end_row();
                ui.checkbox(&mut self.config.characters.use_uppercase, "Uppercase");
                ui.add(egui::TextEdit::singleline(&mut self.config.characters.uppercase).desired_width(f32::INFINITY));
                ui.end_row();
                ui.checkbox(&mut self.config.characters.use_numbers, "Numbers");
                ui.add(egui::TextEdit::singleline(&mut self.config.characters.numbers).desired_width(f32::INFINITY));
                ui.end_row();
                ui.checkbox(&mut self.config.characters.use_special, "Special");
                ui.add(egui::TextEdit::singleline(&mut self.config.characters.special).desired_width(f32::INFINITY));
                ui.end_row();
            });

        ui.add_space(PADDING);
        ui.separator();
        ui.add_space(PADDING);

        ui.horizontal(|ui| {
            ui.label("Password Length:");

            // The slider fills the row, leaving room for the text box on the right.
            ui.spacing_mut().slider_width = ui.available_width() - LENGTH_BOX_WIDTH - ui.spacing().item_spacing.x;
            let default_length = CharacterSettings::default().length;
            let slider = ui
                .add(egui::Slider::new(&mut self.config.characters.length, MIN_LENGTH..=MAX_LENGTH).show_value(false))
                .on_hover_text(format!("Double-click to reset to {}", default_length));
            // The slider only senses drags, so detect the double-click from raw input.
            let double_clicked = ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
            if double_clicked && ui.rect_contains_pointer(slider.rect) {
                self.config.characters.length = default_length;
            }

            let length_box = ui.add(
                egui::TextEdit::singleline(&mut self.char_length_input)
                    .desired_width(LENGTH_BOX_WIDTH)
                    .horizontal_align(egui::Align::Center),
            );
            // Typing a valid length moves the slider straight away; anything else is
            // clamped or reverted once the box loses focus (Enter, Tab or clicking away).
            if let Ok(typed) = self.char_length_input.trim().parse::<usize>() {
                if length_box.changed() && (MIN_LENGTH..=MAX_LENGTH).contains(&typed) {
                    self.config.characters.length = typed;
                }
                if length_box.lost_focus() {
                    self.config.characters.length = typed.clamp(MIN_LENGTH, MAX_LENGTH);
                }
            }
            if !length_box.has_focus() {
                self.char_length_input = self.config.characters.length.to_string();
            }
        });

        ui.add_space(PADDING);

        let generate = egui::Button::new("Generate Password");
        if ui.add_sized([ui.available_width(), 40.0], generate).on_hover_text("Shortcut: Space").clicked() {
            self.generate_and_copy(ui.ctx());
        }

        ui.add_space(PADDING);
        ui.separator();
        ui.add_space(PADDING);

        ui.label("Generated Password:");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.char_password_output.as_str())
                    .font(egui::FontId::monospace(20.0))
                    .desired_width(ui.available_width() - 60.0),
            );
            if ui.add(copy_button()).on_hover_text("Shortcut: Ctrl+C").clicked() {
                self.copy_output(ui.ctx());
            }
        });
    }

    /// Draws the UI for the "Words" tab.
    fn draw_word_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_space(PADDING);
        ui.label("Generate a memorable passphrase from a list of words.");
        ui.add_space(PADDING);

        ui.horizontal(|ui| {
            ui.label("Number of words:");
            for count in MIN_WORDS..=MAX_WORDS {
                ui.radio_value(&mut self.config.words.count, count, count.to_string());
            }
        });
        ui.add_space(PADDING);

        ui.checkbox(&mut self.config.words.capitalize, "Uppercase first character of words");
        ui.checkbox(&mut self.config.words.include_number, "Include a number")
            .on_hover_text("Adds a two-digit number at a random position, e.g. Mango-42-Otter-Kettle-Robin");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.config.words.use_separator, "Separator character:");
            ui.add_enabled_ui(self.config.words.use_separator, |ui| {
                let response = ui.add(egui::TextEdit::singleline(&mut self.config.words.separator).desired_width(30.0));
                if response.changed() {
                    if self.config.words.separator.chars().count() > 1 {
                        self.config.words.separator = self.config.words.separator.chars().next().unwrap_or_default().to_string();
                    }
                }
            });
        });

        ui.add_space(PADDING);

        let generate = egui::Button::new("Generate Passphrase");
        if ui.add_sized([ui.available_width(), 40.0], generate).on_hover_text("Shortcut: Space").clicked() {
            self.generate_and_copy(ui.ctx());
        }

        ui.add_space(PADDING);
        ui.separator();
        ui.add_space(PADDING);

        ui.label("Generated Passphrase:");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.word_password_output.as_str())
                    .font(egui::FontId::monospace(20.0))
                    .desired_width(ui.available_width() - 60.0),
            );
            if ui.add(copy_button()).on_hover_text("Shortcut: Ctrl+C").clicked() {
                self.copy_output(ui.ctx());
            }
        });
    }

    /// Draws the list of recently generated passwords.
    fn draw_history_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong(format!("History (last {})", MAX_ENTRIES));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let can_clear = !self.history.entries.is_empty();
                if ui.add_enabled(can_clear, egui::Button::new("Clear")).clicked() {
                    self.status_message = self.history.clear();
                }
                ui.checkbox(&mut self.show_history, "Show");
            });
        });

        if self.history.entries.is_empty() {
            ui.label(egui::RichText::new("No passwords generated yet.").weak());
        }

        let mut copied = None;
        let row_height = COPY_BUTTON_SIZE.y.max(ui.spacing().interact_size.y);
        for entry in &self.history.entries {
            let (row, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_height), egui::Sense::hover());
            let mut row_ui = ui.child_ui(row, egui::Layout::left_to_right(egui::Align::Center));
            // Stop the text (and its text-selection hover area) where the button's
            // furthest position begins, so hovering the button shows a normal cursor.
            let mut text_area = row;
            text_area.max.x -= COPY_BUTTON_SIZE.x;
            row_ui.set_clip_rect(text_area.intersect(ui.clip_rect()));

            row_ui.label(egui::RichText::new(&entry.created).monospace().weak());
            row_ui.add_space(PADDING);
            let shown = if self.show_history {
                entry.password.clone()
            } else {
                "•".repeat(entry.password.chars().count().min(24))
            };
            let password_rect = row_ui
                .add(egui::Label::new(egui::RichText::new(shown).monospace()).wrap(false))
                .rect;

            // Sit right after the password, but never past the edge of the row:
            // a long password slides underneath the button instead.
            let button_x = (password_rect.right() + PADDING).min(row.right() - COPY_BUTTON_SIZE.x);
            let button_rect = egui::Rect::from_min_size(
                egui::pos2(button_x, row.center().y - COPY_BUTTON_SIZE.y / 2.0),
                COPY_BUTTON_SIZE,
            );
            if ui.put(button_rect, copy_button()).clicked() {
                copied = Some(entry.password.clone());
            }
        }
        if let Some(password) = copied {
            self.copy_to_clipboard(ui.ctx(), password);
        }
        ui.add_space(PADDING / 2.0);
    }

    /// Draws the settings window when it is open.
    fn draw_settings_window(&mut self, ctx: &egui::Context) {
        // A local copy, because the window's contents need `self` mutably too.
        let mut open = self.show_settings_window;
        egui::Window::new("Settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.heading("Visuals");
                ui.label("Color Scheme:");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.config.theme, Theme::Light, "Light");
                    ui.radio_value(&mut self.config.theme, Theme::Dark, "Dark");
                });

                ui.add_space(PADDING);
                ui.separator();
                ui.add_space(PADDING);

                ui.heading("Startup");
                ui.label("Default tab:");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.config.default_tab, Tab::Character, "Characters");
                    ui.radio_value(&mut self.config.default_tab, Tab::Word, "Words");
                });

                ui.add_space(PADDING);
                ui.separator();
                ui.add_space(PADDING);

                ui.heading("Word Generator");
                ui.label("Wordlist File Path:");

                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.words_file_path_str)
                        .hint_text("Built-in list of common words")
                        .desired_width(f32::INFINITY),
                );
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let new_path = PathBuf::from(self.words_file_path_str.trim());
                    if new_path != self.config.words_file_path {
                        self.set_wordlist(new_path);
                    }
                }

                ui.horizontal(|ui| {
                    if ui.button("Browse for Wordlist").clicked() {
                        if let Some(path) = FileDialog::new().add_filter("Text File", &["txt"]).pick_file() {
                            self.set_wordlist(path);
                        }
                    }
                    let using_custom = !self.config.words_file_path.as_os_str().is_empty();
                    if ui.add_enabled(using_custom, egui::Button::new("Use Built-in List")).clicked() {
                        self.set_wordlist(PathBuf::new());
                    }
                });
            });
        self.show_settings_window = open;
    }

    /// Draws the error dialog when an error message is present.
    fn draw_error_dialog(&mut self, ctx: &egui::Context) {
        if let Some(error) = &self.error_message.clone() {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .default_width(320.0) // Set a sensible width
                .show(ctx, |ui| {
                    ui.add_space(PADDING);
                    // Center the content within the window
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        // Use a wrapping label for the error message
                        ui.label(egui::RichText::new(error).color(egui::Color32::from_rgb(255, 100, 100)).strong());
                    });
                    ui.add_space(PADDING * 2.0);
                    // Center the button
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        if ui.button("OK").clicked() {
                            self.error_message = None;
                        }
                    });
                    ui.add_space(PADDING);
                });
        }
    }
}

impl PassGenApp {
    /// Keeps the window tall enough for its content without empty space: it is sized to fit
    /// on startup, grows if the content does (more history, a taller tab) and can't be
    /// shrunk below the content. Extra height from resizing ends up below the history.
    fn fit_window_to_content(&mut self, ctx: &egui::Context, needed_height: f32) {
        let needed = needed_height.ceil();
        if (needed - self.fitted_height).abs() < 1.0 {
            return;
        }
        let current = ctx.screen_rect().size();
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(MIN_WINDOW_WIDTH, needed)));
        let first_fit = self.fitted_height == 0.0;
        if first_fit || current.y < needed {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(current.x, needed)));
        }
        self.fitted_height = needed;
    }

    /// Generates passwords for both tabs so one is ready as soon as the window opens.
    /// Errors are ignored here; they are shown when the user presses Generate.
    fn generate_on_startup(&mut self) {
        let _ = self.generate(Tab::Character);
        let _ = self.generate(Tab::Word);
    }

    /// Generates a new password for `tab` and shows it, without copying it.
    fn generate(&mut self, tab: Tab) -> Result<(), String> {
        match tab {
            Tab::Character => {
                self.char_password_output = generate_char_password(&self.config.characters)?;
            }
            Tab::Word => {
                self.word_password_output = generate_word_password(&self.config.words, &self.words)?;
            }
        }
        Ok(())
    }

    /// Switches to the wordlist at `path` (empty for the built-in list) and
    /// generates a fresh passphrase from it.
    fn set_wordlist(&mut self, path: PathBuf) {
        self.words_file_path_str = path.to_string_lossy().to_string();
        self.config.words_file_path = path;
        let (words, status) = load_words(&self.config.words_file_path);
        self.words = words;
        self.status_message = status;
        if let Err(e) = self.generate(Tab::Word) {
            self.word_password_output.clear();
            self.status_message = e;
        }
    }

    /// Generates a new password for the active tab and copies it.
    fn generate_and_copy(&mut self, ctx: &egui::Context) {
        match self.generate(self.active_tab) {
            Ok(()) => self.copy_output(ctx),
            Err(e) => self.error_message = Some(e),
        }
    }

    /// Copies the password shown on the active tab and records it in the history.
    fn copy_output(&mut self, ctx: &egui::Context) {
        let password = match self.active_tab {
            Tab::Character => self.char_password_output.clone(),
            Tab::Word => self.word_password_output.clone(),
        };
        if password.is_empty() {
            return;
        }
        let history_result = self.history.add(&password);
        self.copy_to_clipboard(ctx, password);
        if let Err(e) = history_result {
            self.status_message = format!("Copied, but saving history failed: {}", e);
        }
    }

    /// Puts `text` on the clipboard and schedules it to be cleared.
    fn copy_to_clipboard(&mut self, ctx: &egui::Context, text: String) {
        ctx.output_mut(|o| o.copied_text = text.clone());
        self.clipboard_clear_at = Some((Instant::now() + CLIPBOARD_CLEAR_AFTER, text));
        self.status_message = format!(
            "Copied to clipboard. It will be cleared in {} minutes.",
            CLIPBOARD_CLEAR_AFTER.as_secs() / 60
        );
    }

    /// Clears the clipboard once the timer runs out, unless something else was copied since.
    fn clear_clipboard_when_due(&mut self, ctx: &egui::Context) {
        let Some((due, copied)) = &self.clipboard_clear_at else {
            return;
        };
        let now = Instant::now();
        if now < *due {
            ctx.request_repaint_after(*due - now);
            return;
        }

        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if clipboard.get_text().is_ok_and(|current| &current == copied) {
                self.status_message = match clipboard.clear() {
                    Ok(()) => "Clipboard cleared.".to_string(),
                    Err(e) => format!("Could not clear the clipboard: {}", e),
                };
            }
        }
        self.clipboard_clear_at = None;
    }

    /// Space generates a new password and Ctrl+C copies the current one,
    /// except while typing in a text field, with selected text, or when the error dialog is open.
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() || self.error_message.is_some() {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Space)) {
            self.generate_and_copy(ctx);
            return;
        }
        let copy_pressed = ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy)));
        let has_selection = egui::text_selection::LabelSelectionState::load(ctx).has_selection();
        if copy_pressed && !has_selection {
            self.copy_output(ctx);
        }
    }
}

/// A button showing a "copy" icon: two overlapping pages.
fn copy_button() -> impl egui::Widget {
    |ui: &mut egui::Ui| {
        let response = ui
            .add(egui::Button::new("").min_size(COPY_BUTTON_SIZE))
            .on_hover_text("Copy to clipboard");
        let visuals = ui.style().interact(&response);
        let painter = ui.painter();
        let rounding = egui::Rounding::same(2.0);
        let page = egui::vec2(9.0, 11.0);
        let center = response.rect.center();

        let back = egui::Rect::from_center_size(center + egui::vec2(-2.0, -2.0), page);
        let front = egui::Rect::from_center_size(center + egui::vec2(2.0, 2.0), page);
        painter.rect_stroke(back, rounding, visuals.fg_stroke);
        painter.rect(front, rounding, visuals.weak_bg_fill, visuals.fg_stroke);
        response
    }
}

/// Launches the graphical user interface.
pub fn run_gui_mode() {
    let icon = load_icon();
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([540.0, 600.0]) // The height is fitted to the content on startup
            .with_min_inner_size([MIN_WINDOW_WIDTH, 300.0])
            .with_icon(icon.clone()),
        ..Default::default()
    };
    eframe::run_native(
        "Password generator",
        options,
        Box::new(|_cc| {
            let mut app = PassGenApp::new();
            app.generate_on_startup();
            Box::new(app)
        }),
    )
        .expect("Failed to run eframe");
}
