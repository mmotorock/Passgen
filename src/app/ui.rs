use crate::app::config::{CharacterSettings, Config, PinSettings, SegmentedSettings};
use crate::app::history::MAX_ENTRIES;
use crate::app::password::{MAX_LENGTH, MAX_WORDS, MIN_LENGTH, MIN_WORDS};
use crate::app::state::{PassGenApp, Style};
use crate::app::styles::{self, GROUPS_RANGE, PIN_LENGTH_RANGE, PRONOUNCEABLE_GROUPS_RANGE};
use crate::app::themes::{self, Theme};
use crate::app::utils::get_config_directory;
use crate::app::utils::{load_icon, load_words};
use eframe::{egui, NativeOptions};
use rfd::FileDialog;
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// The app ID GNOME and other Wayland desktops use to find passgen.desktop and its icon.
pub const APP_ID: &str = "passgen";

const PADDING: f32 = 10.0;
const PANEL_MARGIN: f32 = 8.0; // egui's default inner margin of the central panel
const MIN_WINDOW_WIDTH: f32 = 480.0;
const SIZE_BOX_WIDTH: f32 = 40.0;
const COPY_BUTTON_SIZE: egui::Vec2 = egui::vec2(26.0, 22.0);
const CLIPBOARD_CLEAR_AFTER: Duration = Duration::from_secs(120);
const COPIED_FLASH_DURATION: Duration = Duration::from_millis(1500);
/// After copying, the copied text is highlighted like a selection for this long, then fades out.
const HIGHLIGHT_HOLD: Duration = Duration::from_millis(300);
const HIGHLIGHT_FADE: Duration = Duration::from_millis(300);

impl eframe::App for PassGenApp {
    /// Called each frame to draw the GUI.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        ctx.set_visuals(self.theme().visuals);

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
                            theme: self.config.theme.clone(),
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
                // --- Style Selection ---
                ui.horizontal(|ui| {
                    for style in Style::ALL {
                        ui.selectable_value(&mut self.active_style, style, style.label());
                    }
                });
                ui.separator();

                // --- Options, Generate button and output for the active style ---
                self.draw_style(ui);

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
    /// Draws the active style: a short description, its options, the Generate button and the result.
    fn draw_style(&mut self, ui: &mut egui::Ui) {
        let style = self.active_style;
        ui.add_space(PADDING / 2.0);
        ui.label(egui::RichText::new(description(style)).weak());
        ui.add_space(PADDING);

        match style {
            Style::Random => {
                self.draw_character_sets(ui);
                let default = CharacterSettings::default().length;
                let range = MIN_LENGTH..=MAX_LENGTH;
                size_slider(ui, "Length:", &mut self.config.characters.length, range, default, &mut self.size_input);
            }
            Style::Segmented => {
                self.draw_character_sets(ui);
                let default = SegmentedSettings::default().length;
                let range = MIN_LENGTH..=MAX_LENGTH;
                size_slider(ui, "Length:", &mut self.config.segmented.length, range, default, &mut self.size_input);
            }
            Style::Grouped => {
                radio_row(ui, "Groups:", &mut self.config.grouped.groups, GROUPS_RANGE);
            }
            Style::Pronounceable => {
                radio_row(ui, "Groups:", &mut self.config.pronounceable.groups, PRONOUNCEABLE_GROUPS_RANGE);
                ui.add_space(PADDING / 2.0);
                ui.checkbox(&mut self.config.pronounceable.include_number, "Include a number");
            }
            Style::Pin => {
                let default = PinSettings::default().length;
                size_slider(ui, "Digits:", &mut self.config.pin.length, PIN_LENGTH_RANGE, default, &mut self.size_input);
            }
            Style::Words => self.draw_word_options(ui),
        }

        ui.add_space(PADDING);
        // If the wordlist couldn't be loaded, the reason is more useful than "it's empty".
        let problem = if style == Style::Words && self.words.is_empty() {
            Some(self.wordlist_status.clone())
        } else {
            styles::problem(style, &self.config, self.words.len())
        };
        let (button, output_label) = match style {
            Style::Pin => ("Generate PIN", "Generated PIN:"),
            Style::Words => ("Generate Passphrase", "Generated Passphrase:"),
            _ => ("Generate Password", "Generated Password:"),
        };
        self.draw_generate_button(ui, button, problem);

        ui.add_space(PADDING);
        ui.separator();
        ui.add_space(PADDING);

        ui.label(output_label);
        let output = self.outputs.get(&style).cloned().unwrap_or_default();
        self.draw_output(ui, &output);
    }

    /// Draws the four character sets with their checkboxes (used by Random and Segmented).
    fn draw_character_sets(&mut self, ui: &mut egui::Ui) {
        let sets = &mut self.config.characters;
        egui::Grid::new("char_sets_grid")
            .num_columns(2)
            .spacing([PADDING, PADDING])
            .show(ui, |ui| {
                for (enabled, chars, name) in [
                    (&mut sets.use_lowercase, &mut sets.lowercase, "Lowercase"),
                    (&mut sets.use_uppercase, &mut sets.uppercase, "Uppercase"),
                    (&mut sets.use_numbers, &mut sets.numbers, "Numbers"),
                    (&mut sets.use_special, &mut sets.special, "Special"),
                ] {
                    ui.checkbox(enabled, name);
                    ui.add(egui::TextEdit::singleline(chars).desired_width(f32::INFINITY));
                    ui.end_row();
                }
            });
        ui.add_space(PADDING);
    }

    /// Draws the passphrase options (the Words style).
    fn draw_word_options(&mut self, ui: &mut egui::Ui) {
        radio_row(ui, "Number of words:", &mut self.config.words.count, MIN_WORDS..=MAX_WORDS);
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
    }

    /// Draws the Generate button. If the settings can't work (`problem`), the button is
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
        let success = self.theme().success;
        let highlight = self.highlight_strength(ui.ctx(), password);
        ui.horizontal(|ui| {
            let field = egui::TextEdit::singleline(&mut &*password)
                .font(egui::FontId::monospace(20.0))
                .desired_width(ui.available_width() - 60.0)
                .show(ui);
            if let Some(strength) = highlight {
                let text_rect = field.galley.rect.translate(field.galley_pos.to_vec2());
                paint_highlight(ui, text_rect, field.text_clip_rect, strength, |painter, color| {
                    painter.galley_with_override_text_color(field.galley_pos, field.galley.clone(), color);
                });
            }
            if ui.add(copy_button(copied, success)).on_hover_text("Shortcut: Ctrl+C").clicked() {
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
        let success = self.theme().success;
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
                .add(egui::Label::new(egui::RichText::new(&shown).monospace()).extend())
                .rect;
            if let Some(strength) = self.highlight_strength(ui.ctx(), &entry.password) {
                let font = egui::TextStyle::Monospace.resolve(ui.style());
                paint_highlight(&row_ui, password_rect, text_area, strength, |painter, color| {
                    painter.text(password_rect.left_center(), egui::Align2::LEFT_CENTER, &shown, font, color);
                });
            }

            // Sit right after the password, but never past the edge of the row:
            // a long password slides underneath the button instead.
            let button_x = (password_rect.right() + PADDING).min(row.right() - COPY_BUTTON_SIZE.x);
            let button_rect = egui::Rect::from_min_size(
                egui::pos2(button_x, row.center().y - COPY_BUTTON_SIZE.y / 2.0),
                COPY_BUTTON_SIZE,
            );
            let just_copied = self.was_just_copied(ui.ctx(), &entry.password);
            if ui.put(button_rect, copy_button(just_copied, success)).clicked() {
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
                ui.horizontal(|ui| {
                    ui.label("Color scheme:");
                    egui::ComboBox::from_id_salt("theme")
                        .selected_text(self.theme().name)
                        .show_ui(ui, |ui| {
                            for theme in &self.themes {
                                ui.selectable_value(&mut self.config.theme, theme.id.clone(), &theme.name);
                            }
                        });
                });
                ui.horizontal(|ui| {
                    let dir = themes::themes_directory(&get_config_directory());
                    if ui.button("Open Themes Folder").on_hover_text(dir.to_string_lossy()).clicked() {
                        self.status_message = open_folder(&dir);
                    }
                    if ui.button("Reload Themes").on_hover_text("Pick up themes added or edited in the folder").clicked() {
                        self.reload_themes();
                    }
                });

                ui.add_space(PADDING);
                ui.separator();
                ui.add_space(PADDING);

                ui.heading("Startup");
                ui.horizontal(|ui| {
                    ui.label("Default style:");
                    egui::ComboBox::from_id_salt("default_style")
                        .selected_text(self.config.default_style.label())
                        .show_ui(ui, |ui| {
                            for style in Style::ALL {
                                ui.selectable_value(&mut self.config.default_style, style, style.label());
                            }
                        });
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
    /// on startup, grows if the content does (more history, a taller style) and can't be
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

    /// The selected theme, or Dark if it isn't in the themes folder (any more).
    fn theme(&self) -> Theme {
        themes::find(&self.themes, &self.config.theme)
            .or_else(|| themes::find(&self.themes, themes::DARK))
            .cloned()
            .unwrap_or_else(themes::fallback)
    }

    /// Loads the themes folder again, so new or edited theme files take effect.
    fn reload_themes(&mut self) {
        let (themes, messages) = themes::load_themes(&themes::themes_directory(&get_config_directory()));
        let count = themes.len();
        self.themes = themes;
        self.status_message = if messages.is_empty() {
            format!("Loaded {} themes.", count)
        } else {
            messages.join("\n")
        };
    }

    /// Generates a password for every style so one is ready as soon as the window opens.
    /// Errors are ignored here; each style explains unusable settings itself.
    fn generate_on_startup(&mut self) {
        for style in Style::ALL {
            let _ = self.generate(style);
        }
    }

    /// Generates a new password in `style` and shows it, without copying it.
    fn generate(&mut self, style: Style) -> Result<(), String> {
        let password = styles::generate(style, &self.config, &self.words)?;
        self.outputs.insert(style, password);
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
        if self.generate(Style::Words).is_err() {
            self.outputs.remove(&Style::Words); // The Words style explains why
        }
    }

    /// Generates a new password in the active style and copies it.
    fn generate_and_copy(&mut self, ctx: &egui::Context) {
        match self.generate(self.active_style) {
            Ok(()) => {
                self.generate_error = None;
                self.copy_output(ctx);
            }
            Err(e) => self.generate_error = Some(e),
        }
    }

    /// Copies the password shown for the active style and records it in the history.
    fn copy_output(&mut self, ctx: &egui::Context) {
        let Some(password) = self.outputs.get(&self.active_style).cloned() else {
            return;
        };
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

    /// How strongly to highlight `password` because it was just copied: 1 at first, fading to
    /// nothing, or `None` once the highlight is over (or something else was copied).
    fn highlight_strength(&self, ctx: &egui::Context, password: &str) -> Option<f32> {
        let (copied_at, copied) = self.copied_flash.as_ref()?;
        let elapsed = copied_at.elapsed();
        if copied != password || elapsed >= HIGHLIGHT_HOLD + HIGHLIGHT_FADE {
            return None;
        }
        ctx.request_repaint(); // Keep animating until the highlight is gone
        let fading_for = elapsed.saturating_sub(HIGHLIGHT_HOLD).as_secs_f32();
        Some(1.0 - fading_for / HIGHLIGHT_FADE.as_secs_f32())
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

/// Highlights text the way a selection looks: the theme's selection colour behind it and its
/// selected-text colour for the text, which `draw_text` paints again on top. `strength` (0 to 1)
/// fades both out, revealing the normal text underneath.
fn paint_highlight(
    ui: &egui::Ui,
    text_rect: egui::Rect,
    clip_rect: egui::Rect,
    strength: f32,
    draw_text: impl FnOnce(&egui::Painter, egui::Color32),
) {
    let painter = ui.painter().with_clip_rect(clip_rect.intersect(ui.clip_rect()));
    let selection = ui.visuals().selection;
    painter.rect_filled(text_rect.expand2(egui::vec2(3.0, 1.0)), 2.0, selection.bg_fill.gamma_multiply(strength));
    draw_text(&painter, selection.stroke.color.gamma_multiply(strength));
}

/// Opens `dir` in the system's file manager, returning a status message.
fn open_folder(dir: &std::path::Path) -> String {
    let opener = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    match std::process::Command::new(opener).arg(dir).spawn() {
        Ok(_) => format!("Opened {}", dir.display()),
        Err(e) => format!("Could not open {}: {}", dir.display(), e),
    }
}

/// A one-line explanation of a style, shown above its options.
fn description(style: Style) -> &'static str {
    match style {
        Style::Random => "Every character picked at random from the selected sets.",
        Style::Segmented => "Groups of 3 letters in the same case, each followed by a number or symbol.",
        Style::Grouped => "Blocks of 6 lowercase letters with one capital and one digit, like Apple's passwords.",
        Style::Pronounceable => "Made-up words of easy syllables that you can say out loud.",
        Style::Pin => "Digits only, avoiding obvious patterns like 111111 or 123456.",
        Style::Words => "A memorable passphrase of common words.",
    }
}

/// A row of radio buttons, one for each value in `range`.
fn radio_row(ui: &mut egui::Ui, label: &str, value: &mut usize, range: RangeInclusive<usize>) {
    ui.horizontal(|ui| {
        ui.label(label);
        for option in range {
            ui.radio_value(value, option, option.to_string());
        }
    });
}

/// A slider with a text box for typing the value. Double-clicking the slider resets it
/// to `default`; `input` holds the box's text while it is being edited.
fn size_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut usize,
    range: RangeInclusive<usize>,
    default: usize,
    input: &mut String,
) {
    ui.horizontal(|ui| {
        ui.label(label);

        // The slider fills the row, leaving room for the text box on the right.
        ui.spacing_mut().slider_width = ui.available_width() - SIZE_BOX_WIDTH - ui.spacing().item_spacing.x;
        let slider = ui
            .add(egui::Slider::new(value, range.clone()).show_value(false))
            .on_hover_text(format!("Double-click to reset to {}", default));
        // The slider only senses drags, so detect the double-click from raw input.
        let double_clicked = ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
        if double_clicked && ui.rect_contains_pointer(slider.rect) {
            *value = default;
        }

        let size_box = ui.add(
            egui::TextEdit::singleline(input)
                .desired_width(SIZE_BOX_WIDTH)
                .align(egui::Align2::CENTER_CENTER),
        );
        // Typing a valid value moves the slider straight away; anything else is
        // clamped or reverted once the box loses focus (Enter, Tab or clicking away).
        if let Ok(typed) = input.trim().parse::<usize>() {
            if size_box.changed() && range.contains(&typed) {
                *value = typed;
            }
            if size_box.lost_focus() {
                *value = typed.clamp(*range.start(), *range.end());
            }
        }
        if !size_box.has_focus() {
            *input = value.to_string();
        }
    });
}

/// A button showing a "copy" icon (two overlapping pages), or a ✓ in `success` right after copying.
fn copy_button(just_copied: bool, success: egui::Color32) -> impl egui::Widget {
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
            painter.add(egui::Shape::line(check, egui::Stroke::new(2.0, success)));
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
            .with_inner_size([560.0, 600.0]) // The height is fitted to the content on startup
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
