use crate::app::password::{generate_char_password, generate_word_password};
use crate::app::state::{PassGenApp, Tab, Theme};
use crate::app::utils::{load_icon, load_words_from_file, save_password_to_file};
use eframe::{egui, NativeOptions};
use rfd::FileDialog;
use std::path::PathBuf;

const PADDING: f32 = 10.0;

impl eframe::App for PassGenApp {
    /// Called each frame to draw the GUI.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply the selected theme
        ctx.set_visuals(match self.theme {
            Theme::Light => egui::Visuals::light(),
            Theme::Dark => egui::Visuals::dark(),
        });

        // --- Handle Close Request ---
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.get_current_config_as_string() != self.saved_config_state && !self.force_close {
                self.show_exit_confirmation = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }

        // --- Top Menu Bar ---
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Config", |ui| {
                    if ui.button("Save Configuration").clicked() {
                        self.status_message = self.save_config();
                        ui.close_menu();
                    }
                    if ui.button("Reload Configuration").clicked() {
                        let path = self.config_path.clone();
                        self.status_message = self.load_config_from_path(&path);
                        ui.close_menu();
                    }
                    if ui.button("Load Defaults").clicked() {
                        let defaults = Self::default();
                        self.active_tab = defaults.active_tab;
                        self.char_sets = defaults.char_sets;
                        self.use_lowercase = defaults.use_lowercase;
                        self.use_uppercase = defaults.use_uppercase;
                        self.use_numbers = defaults.use_numbers;
                        self.use_special = defaults.use_special;
                        self.char_length = defaults.char_length;
                        self.word_count = defaults.word_count;
                        self.use_separator = defaults.use_separator;
                        self.separator_char = defaults.separator_char;
                        self.use_uppercase_words = defaults.use_uppercase_words;
                        self.status_message = "Defaults loaded. Save to make permanent.".to_string();
                        ui.close_menu();
                    }
                });
                ui.menu_button("Settings", |ui| {
                    if ui.button("Open Settings").clicked() {
                        self.show_settings_window = true;
                        ui.close_menu();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
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
        });

        // --- Status Bar ---
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add_space(PADDING);
                ui.label(egui::RichText::new(&self.status_message).italics());
            });
        });

        // --- Draw Windows ---
        self.draw_settings_window(ctx);
        self.draw_exit_confirmation_window(ctx);
        self.draw_error_dialog(ctx);
    }
}

impl PassGenApp {
    /// Draws the UI for the "Characters" tab.
    fn draw_character_tab(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("char_sets_grid")
            .num_columns(2)
            .spacing([PADDING, PADDING])
            .show(ui, |ui| {
                ui.checkbox(&mut self.use_lowercase, "Lowercase");
                ui.add(egui::TextEdit::singleline(&mut self.char_sets.lowercase).desired_width(f32::INFINITY));
                ui.end_row();
                ui.checkbox(&mut self.use_uppercase, "Uppercase");
                ui.add(egui::TextEdit::singleline(&mut self.char_sets.uppercase).desired_width(f32::INFINITY));
                ui.end_row();
                ui.checkbox(&mut self.use_numbers, "Numbers");
                ui.add(egui::TextEdit::singleline(&mut self.char_sets.numbers).desired_width(f32::INFINITY));
                ui.end_row();
                ui.checkbox(&mut self.use_special, "Special");
                ui.add(egui::TextEdit::singleline(&mut self.char_sets.special).desired_width(f32::INFINITY));
                ui.end_row();
            });

        ui.add_space(PADDING);
        ui.separator();
        ui.add_space(PADDING);

        ui.horizontal(|ui| {
            ui.label("Password Length:");
            ui.add(egui::DragValue::new(&mut self.char_length).clamp_range(12..=128));
        });

        ui.add_space(PADDING);

        if ui.add_sized([ui.available_width(), 40.0], egui::Button::new("Generate Password")).clicked() {
            match generate_char_password(
                self.char_length,
                &self.char_sets,
                self.use_lowercase,
                self.use_uppercase,
                self.use_numbers,
                self.use_special,
            ) {
                Ok(password) => {
                    self.char_password_output = password;
                    self.status_message = save_password_to_file(&self.char_password_output, &self.password_file_path);
                }
                Err(e) => self.error_message = Some(e),
            }
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
            if ui.button("Copy").clicked() {
                if !self.char_password_output.is_empty() {
                    ui.output_mut(|o| o.copied_text = self.char_password_output.clone());
                    self.status_message = "Password copied to clipboard!".to_string();
                }
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
            ui.radio_value(&mut self.word_count, 3, "3");
            ui.radio_value(&mut self.word_count, 4, "4");
            ui.radio_value(&mut self.word_count, 5, "5");
        });
        ui.add_space(PADDING);

        ui.checkbox(&mut self.use_uppercase_words, "Uppercase first character of words");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.use_separator, "Separator character:");
            ui.add_enabled_ui(self.use_separator, |ui| {
                let response = ui.add(egui::TextEdit::singleline(&mut self.separator_char).desired_width(30.0));
                if response.changed() {
                    if self.separator_char.chars().count() > 1 {
                        self.separator_char = self.separator_char.chars().next().unwrap_or_default().to_string();
                    }
                }
            });
        });

        ui.add_space(PADDING);

        if ui.add_sized([ui.available_width(), 40.0], egui::Button::new("Generate Passphrase")).clicked() {
            match generate_word_password(
                self.word_count,
                &self.words,
                self.use_separator,
                &self.separator_char,
                self.use_uppercase_words,
            ) {
                Ok(password) => {
                    self.word_password_output = password;
                    self.status_message = save_password_to_file(&self.word_password_output, &self.password_file_path);
                }
                Err(e) => self.error_message = Some(e),
            }
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
            if ui.button("Copy").clicked() {
                if !self.word_password_output.is_empty() {
                    ui.output_mut(|o| o.copied_text = self.word_password_output.clone());
                    self.status_message = "Passphrase copied to clipboard!".to_string();
                }
            }
        });
    }

    /// Draws the settings window when it is open.
    fn draw_settings_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Settings")
            .open(&mut self.show_settings_window)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.heading("Visuals");
                ui.label("Color Scheme:");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.theme, Theme::Light, "Light");
                    ui.radio_value(&mut self.theme, Theme::Dark, "Dark");
                });

                ui.add_space(PADDING);
                ui.separator();
                ui.add_space(PADDING);

                ui.heading("Word Generator");
                ui.label("Wordlist File Path:");

                let response = ui.add(egui::TextEdit::singleline(&mut self.words_file_path_str).desired_width(f32::INFINITY));
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let new_path = PathBuf::from(&self.words_file_path_str);
                    if new_path != self.words_file_path {
                        self.words_file_path = new_path;
                        let (words, status) = load_words_from_file(&self.words_file_path);
                        self.words = words;
                        self.status_message = status;
                    }
                }

                if ui.button("Browse for Wordlist").clicked() {
                    if let Some(path) = FileDialog::new().add_filter("Text File", &["txt"]).pick_file() {
                        self.words_file_path_str = path.to_string_lossy().to_string();
                        self.words_file_path = path;
                        let (words, status) = load_words_from_file(&self.words_file_path);
                        self.words = words;
                        self.status_message = status;
                    }
                }
            });
    }

    /// Draws the exit confirmation dialog when needed.
    fn draw_exit_confirmation_window(&mut self, ctx: &egui::Context) {
        if self.show_exit_confirmation {
            egui::Window::new("Unsaved Changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("You have unsaved configuration changes. Would you like to save them?");
                    ui.add_space(PADDING);
                    ui.horizontal(|ui| {
                        if ui.button("Yes").clicked() {
                            self.save_config();
                            self.force_close = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("No").clicked() {
                            self.force_close = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_exit_confirmation = false;
                        }
                    });
                });
        }
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

/// Launches the graphical user interface.
pub fn run_gui_mode() {
    let icon = load_icon();
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([540.0, 480.0])
            .with_resizable(false)
            .with_icon(icon.clone()),
        ..Default::default()
    };
    eframe::run_native(
        "Password generator",
        options,
        Box::new(|_cc| Box::new(PassGenApp::new())),
    )
        .expect("Failed to run eframe");
}
