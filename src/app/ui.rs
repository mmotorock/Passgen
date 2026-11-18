use crate::app::config::{CharacterSettings, Config};
use crate::app::history::MAX_ENTRIES;
use crate::app::password::{
    char_settings_problem, generate_char_password, generate_word_password, word_settings_problem, MAX_LENGTH,
    MAX_WORDS, MIN_LENGTH, MIN_WORDS,
};
use crate::app::state::{PassGenApp, Tab, Theme};
use crate::app::utils::{load_icon, load_words};
use eframe::{egui, NativeOptions};
use rfd::FileDialog;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// The app ID GNOME and other Wayland desktops use to find passgen.desktop and its icon.
pub const APP_ID: &str = "passgen";

const PADDING: f32 = 10.0;
const PANEL_MARGIN: f32 = 8.0; // egui's default inner margin of the central panel
const MIN_WINDOW_WIDTH: f32 = 480.0;
const LENGTH_BOX_WIDTH: f32 = 40.0;
const COPY_BUTTON_SIZE: egui::Vec2 = egui::vec2(26.0, 22.0);
const CLIPBOARD_CLEAR_AFTER: Duration = Duration::from_secs(120);
const COPIED_FLASH_DURATION: Duration = Duration::from_millis(1500);
const COPIED_COLOR: egui::Color32 = egui::Color32::from_rgb(80, 180, 100);

impl eframe::App for PassGenApp {
    /// Called each frame to draw the GUI.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Apply the selected theme
        ctx.set_visuals(match self.config.theme {
            Theme::Light => egui::Visuals::light(),
            Theme::Dark => egui::Visuals::dark(),
        });

        // Handled before any widget so Space doesn't also press a focused button.
        self.handle_shortcuts(&ctx);

        // --- Top Menu Bar ---
        egui::Panel::top("top_panel").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Settings", |ui| {
                    if ui.button("Open Settings").clicked() {
                        self.show_settings_window = true;
                        ui.close();
                    }
                    if ui.button("Restore Defaults").clicked() {
                        // Keep the theme and wordlist; reset everything else.
                        self.config = Config {
                            theme: self.config.theme,
                            words_file_path: self.config.words_file_path.clone(),
                            ..Config::default()
                        };
                        self.status_message = "Default settings restored.".to_string();
                        ui.close();
                    }
                });
            });
        });

        // --- Status Bar (bottom panels must be added before the central panel) ---
        let status_bar = egui::Panel::bottom("status_bar").show(ui, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add_space(PADDING);
                ui.label(egui::RichText::new(&self.status_message).italics());
            });
        });

        let content_bottom = egui::CentralPanel::default()
            .show(ui, |ui| {
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
        self.fit_window_to_content(&ctx, content_bottom + PANEL_MARGIN + status_bar.response.rect.height());

        // --- Draw Windows ---
        self.draw_settings_window(&ctx);

        // Settings are saved as soon as they change, so there is nothing to confirm on exit.
        if self.config != self.saved_config {
            if let Err(e) = self.config.save(&self.config_path) {
                self.status_message = e;
            }
            self.saved_config = self.config.clone(); // Don't retry a failed save every frame
            self.generate_error = None; // It may no longer apply to the new settings
        }
        self.clear_clipboard_when_due(&ctx);
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
                    .align(egui::Align2::CENTER_CENTER),
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
        let problem = char_settings_problem(&self.config.characters);
        self.draw_generate_button(ui, "Generate Password", problem);

        ui.add_space(PADDING);
        ui.separator();
        ui.add_space(PADDING);

        ui.label("Generated Password:");
        let output = self.char_password_output.clone();
        self.draw_output(ui, &output);
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
                if response.changed() && self.config.words.separator.chars().count() > 1 {
                    self.config.words.separator = self.config.words.separator.chars().next().unwrap_or_default().to_string();
                }
            });
        });

        ui.add_space(PADDING);
        // If the wordlist couldn't be loaded, the reason is more useful than "it's empty".
        let problem = if self.words.is_empty() {
            Some(self.wordlist_status.clone())
        } else {
            word_settings_problem(&self.config.words, self.words.len())
        };
        self.draw_generate_button(ui, "Generate Passphrase", problem);

        ui.add_space(PADDING);
        ui.separator();
        ui.add_space(PADDING);

        ui.label("Generated Passphrase:");
        let output = self.word_password_output.clone();
        self.draw_output(ui, &output);
    }

    /// Draws a tab's Generate button. If the settings can't work (`problem`), the button is
    /// disabled and the reason is shown beneath it, as is any error from the last attempt.
    fn draw_generate_button(&mut self, ui: &mut egui::Ui, label: &str, problem: Option<String>) {
        let button = ui
            .add_enabled_ui(problem.is_none(), |ui| {
                ui.add_sized([ui.available_width(), 40.0], egui::Button::new(label))
            })
            .inner
            .on_hover_text("Shortcut: Space");
        if button.clicked() {
            self.generate_and_copy(ui.ctx());
        }

        if let Some(message) = problem.or_else(|| self.generate_error.clone()) {
            ui.add_space(PADDING / 2.0);
            let color = ui.visuals().error_fg_color;
            ui.label(egui::RichText::new(format!("⚠ {}", message)).color(color));
        }
    }

    /// Draws a generated password with its copy button.
    fn draw_output(&mut self, ui: &mut egui::Ui, password: &str) {
        let copied = self.was_just_copied(ui.ctx(), password);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut &*password)
                    .font(egui::FontId::monospace(20.0))
                    .desired_width(ui.available_width() - 60.0),
            );
            if ui.add(copy_button(copied)).on_hover_text("Shortcut: Ctrl+C").clicked() {
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
            let mut row_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(row)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
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
                .add(egui::Label::new(egui::RichText::new(shown).monospace()).extend())
                .rect;

            // Sit right after the password, but never past the edge of the row:
            // a long password slides underneath the button instead.
            let button_x = (password_rect.right() + PADDING).min(row.right() - COPY_BUTTON_SIZE.x);
            let button_rect = egui::Rect::from_min_size(
                egui::pos2(button_x, row.center().y - COPY_BUTTON_SIZE.y / 2.0),
                COPY_BUTTON_SIZE,
            );
            let just_copied = self.was_just_copied(ui.ctx(), &entry.password);
            if ui.put(button_rect, copy_button(just_copied)).clicked() {
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
        let current = ctx.content_rect().size();
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(MIN_WINDOW_WIDTH, needed)));
        let first_fit = self.fitted_height == 0.0;
        if first_fit || current.y < needed {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(current.x, needed)));
        }
        self.fitted_height = needed;
    }

    /// Generates passwords for both tabs so one is ready as soon as the window opens.
    /// Errors are ignored here; the tabs explain unusable settings themselves.
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
        self.status_message = status.clone();
        self.wordlist_status = status;
        if self.generate(Tab::Word).is_err() {
            self.word_password_output.clear(); // The Words tab explains why
        }
    }

    /// Generates a new password for the active tab and copies it.
    fn generate_and_copy(&mut self, ctx: &egui::Context) {
        match self.generate(self.active_tab) {
            Ok(()) => {
                self.generate_error = None;
                self.copy_output(ctx);
            }
            Err(e) => self.generate_error = Some(e),
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
        ctx.copy_text(text.clone());
        let now = Instant::now();
        self.copied_flash = Some((now, text.clone()));
        self.clipboard_clear_at = Some((now + CLIPBOARD_CLEAR_AFTER, text));
        self.status_message = format!(
            "Copied to clipboard. It will be cleared in {} minutes.",
            CLIPBOARD_CLEAR_AFTER.as_secs() / 60
        );
    }

    /// Whether `password` was copied moments ago, so its copy button should show a ✓.
    fn was_just_copied(&self, ctx: &egui::Context, password: &str) -> bool {
        let Some((copied_at, copied)) = &self.copied_flash else {
            return false;
        };
        let elapsed = copied_at.elapsed();
        if copied != password || elapsed >= COPIED_FLASH_DURATION {
            return false;
        }
        ctx.request_repaint_after(COPIED_FLASH_DURATION - elapsed); // Switch back to the copy icon
        true
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
    /// except while typing in a text field or with text selected.
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.text_edit_focused() {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Space)) {
            self.generate_and_copy(ctx);
            return;
        }
        let copy_pressed = ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy)));
        let has_selection = ctx.plugin::<egui::text_selection::LabelSelectionState>().lock().has_selection();
        if copy_pressed && !has_selection {
            self.copy_output(ctx);
        }
    }
}

/// A button showing a "copy" icon (two overlapping pages), or a green ✓ right after copying.
fn copy_button(just_copied: bool) -> impl egui::Widget {
    move |ui: &mut egui::Ui| {
        let hover = if just_copied { "Copied!" } else { "Copy to clipboard" };
        let response = ui.add(egui::Button::new("").min_size(COPY_BUTTON_SIZE)).on_hover_text(hover);
        let visuals = ui.style().interact(&response);
        let painter = ui.painter();
        let center = response.rect.center();

        if just_copied {
            let check = vec![
                center + egui::vec2(-5.0, 0.0),
                center + egui::vec2(-1.5, 3.5),
                center + egui::vec2(5.0, -4.0),
            ];
            painter.add(egui::Shape::line(check, egui::Stroke::new(2.0, COPIED_COLOR)));
        } else {
            let corner_radius = egui::CornerRadius::same(2);
            let page = egui::vec2(9.0, 11.0);
            let back = egui::Rect::from_center_size(center + egui::vec2(-2.0, -2.0), page);
            let front = egui::Rect::from_center_size(center + egui::vec2(2.0, 2.0), page);
            painter.rect_stroke(back, corner_radius, visuals.fg_stroke, egui::StrokeKind::Inside);
            painter.rect(front, corner_radius, visuals.weak_bg_fill, visuals.fg_stroke, egui::StrokeKind::Inside);
        }
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
            .with_app_id(APP_ID) // On Wayland the dock icon comes from the matching .desktop file
            .with_icon(icon), // Used on Windows and X11
        ..Default::default()
    };
    eframe::run_native(
        "Password generator",
        options,
        Box::new(|_cc| {
            let mut app = PassGenApp::new();
            app.generate_on_startup();
            Ok(Box::new(app))
        }),
    )
    .expect("Failed to run eframe");
}
