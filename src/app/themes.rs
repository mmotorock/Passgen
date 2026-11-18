//! Colour themes: egui's built-in light and dark themes, plus any `.toml` files in the
//! themes folder. See the files in the repository's `themes/` folder for the format.

use eframe::egui::{Color32, Visuals};
use serde::{Deserialize, Deserializer};
use std::fs;
use std::path::{Path, PathBuf};

/// The IDs of the two themes built into egui.
pub const LIGHT: &str = "light";
pub const DARK: &str = "dark";

/// Themes written to the themes folder when it is first created.
const DEFAULT_THEME_FILES: [(&str, &str); 5] = [
    ("gruvbox-dark", include_str!("../../themes/gruvbox-dark.toml")),
    ("gruvbox-light", include_str!("../../themes/gruvbox-light.toml")),
    ("nord", include_str!("../../themes/nord.toml")),
    ("solarized-dark", include_str!("../../themes/solarized-dark.toml")),
    ("solarized-light", include_str!("../../themes/solarized-light.toml")),
];

/// The check mark colour after copying, when a theme doesn't set `success`.
const DEFAULT_SUCCESS: Color32 = Color32::from_rgb(80, 180, 100);

/// A colour scheme the GUI can use.
#[derive(Clone, Debug)]
pub struct Theme {
    /// The file name without `.toml` (or `light`/`dark`); what config.toml stores.
    pub id: String,
    /// The name shown in the settings.
    pub name: String,
    pub visuals: Visuals,
    /// The colour of the check mark after copying.
    pub success: Color32,
}

/// The contents of a theme file.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeFile {
    name: Option<String>,
    #[serde(default = "default_dark")]
    dark: bool,
    #[serde(default)]
    colors: ThemeColors,
}

fn default_dark() -> bool {
    true
}

/// The colours a theme file can set; see the comments in the default theme files.
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ThemeColors {
    background: Option<Hex>,
    surface: Option<Hex>,
    text: Option<Hex>,
    weak_text: Option<Hex>,
    strong_text: Option<Hex>,
    widget: Option<Hex>,
    widget_hover: Option<Hex>,
    widget_active: Option<Hex>,
    border: Option<Hex>,
    accent: Option<Hex>,
    accent_text: Option<Hex>,
    error: Option<Hex>,
    success: Option<Hex>,
}

/// A colour written as "#rrggbb" or "#rrggbbaa".
#[derive(Clone, Copy)]
struct Hex(Color32);

impl<'de> Deserialize<'de> for Hex {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Color32::from_hex(&text)
            .map(Hex)
            .map_err(|_| serde::de::Error::custom(format!("{:?} is not a colour like \"#88c0d0\"", text)))
    }
}

impl Theme {
    /// egui's built-in light and dark themes.
    fn builtin() -> [Theme; 2] {
        [
            Theme { id: LIGHT.to_string(), name: "Light".to_string(), visuals: Visuals::light(), success: DEFAULT_SUCCESS },
            Theme { id: DARK.to_string(), name: "Dark".to_string(), visuals: Visuals::dark(), success: DEFAULT_SUCCESS },
        ]
    }

    /// Reads a theme from the text of a theme file called `<id>.toml`.
    fn from_toml(id: &str, text: &str) -> Result<Theme, String> {
        let file: ThemeFile = toml::from_str(text).map_err(|e| e.message().to_string())?;
        let colors = file.colors;
        let mut visuals = if file.dark { Visuals::dark() } else { Visuals::light() };
        let widgets = &mut visuals.widgets;

        if let Some(Hex(c)) = colors.background {
            visuals.panel_fill = c;
            visuals.window_fill = c;
            widgets.noninteractive.bg_fill = c;
            widgets.noninteractive.weak_bg_fill = c;
        }
        if let Some(Hex(c)) = colors.surface {
            visuals.extreme_bg_color = c;
            visuals.text_edit_bg_color = Some(c);
            visuals.faint_bg_color = c;
            visuals.code_bg_color = c;
        }
        if let Some(Hex(c)) = colors.text {
            for widget in [&mut widgets.noninteractive, &mut widgets.inactive, &mut widgets.hovered] {
                widget.fg_stroke.color = c;
            }
        }
        if let Some(Hex(c)) = colors.strong_text.or(colors.text) {
            widgets.active.fg_stroke.color = c;
            widgets.open.fg_stroke.color = c;
        }
        if let Some(Hex(c)) = colors.weak_text {
            visuals.weak_text_color = Some(c);
        }
        for (color, widget) in [
            (colors.widget, &mut widgets.inactive),
            (colors.widget_hover, &mut widgets.hovered),
            (colors.widget_hover, &mut widgets.open),
            (colors.widget_active, &mut widgets.active),
        ] {
            if let Some(Hex(c)) = color {
                widget.bg_fill = c;
                widget.weak_bg_fill = c;
            }
        }
        if let Some(Hex(c)) = colors.border {
            widgets.noninteractive.bg_stroke.color = c;
            visuals.window_stroke.color = c;
        }
        if let Some(Hex(c)) = colors.accent {
            visuals.selection.bg_fill = c;
            visuals.hyperlink_color = c;
            widgets.hovered.bg_stroke.color = c;
        }
        if let Some(Hex(c)) = colors.accent_text {
            visuals.selection.stroke.color = c;
        }
        if let Some(Hex(c)) = colors.error {
            visuals.error_fg_color = c;
        }

        Ok(Theme {
            id: id.to_string(),
            name: file.name.unwrap_or_else(|| id.to_string()),
            visuals,
            success: colors.success.map_or(DEFAULT_SUCCESS, |Hex(c)| c),
        })
    }
}

/// egui's dark theme, for when no themes have been loaded.
pub fn fallback() -> Theme {
    let [_, dark] = Theme::builtin();
    dark
}

/// The themes folder inside the config folder.
pub fn themes_directory(config_dir: &Path) -> PathBuf {
    config_dir.join("themes")
}

/// Creates the themes folder with the default themes, but only if it doesn't exist yet,
/// so themes the user deleted stay deleted. Returns a message if something went wrong.
pub fn create_default_themes(dir: &Path) -> Option<String> {
    if dir.exists() {
        return None;
    }
    let result = fs::create_dir_all(dir).and_then(|()| {
        DEFAULT_THEME_FILES
            .iter()
            .try_for_each(|(id, text)| fs::write(dir.join(format!("{}.toml", id)), text))
    });
    result.err().map(|e| format!("Could not create the default themes: {}", e))
}

/// Loads the built-in themes and every `.toml` file in `dir`, sorted by name after Light and
/// Dark. A file named like a built-in theme replaces it. Files that can't be read are skipped
/// and reported in the returned messages.
pub fn load_themes(dir: &Path) -> (Vec<Theme>, Vec<String>) {
    let mut themes = Theme::builtin().to_vec();
    let mut messages = Vec::new();
    let mut files: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
            .collect(),
        Err(_) => Vec::new(), // No themes folder: only the built-in themes
    };
    files.sort();

    let mut from_files = Vec::new();
    for path in files {
        let id = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let theme = fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|text| Theme::from_toml(&id, &text));
        match theme {
            Ok(theme) => match themes.iter_mut().find(|builtin| builtin.id.eq_ignore_ascii_case(&id)) {
                Some(builtin) => *builtin = theme,
                None => from_files.push(theme),
            },
            Err(e) => messages.push(format!("Skipped theme {}.toml: {}", id, e)),
        }
    }
    from_files.sort_by_key(|theme| theme.name.to_lowercase());
    themes.extend(from_files);
    (themes, messages)
}

/// The theme with this ID (ignoring case, so "Dark" from older configs works), if there is one.
pub fn find<'a>(themes: &'a [Theme], id: &str) -> Option<&'a Theme> {
    themes.iter().find(|theme| theme.id.eq_ignore_ascii_case(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("passgen-themes-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn default_theme_files_are_valid() {
        for (id, text) in DEFAULT_THEME_FILES {
            let theme = Theme::from_toml(id, text).unwrap_or_else(|e| panic!("{}: {}", id, e));
            assert_ne!(theme.name, id, "{} should have a display name", id);
        }
    }

    #[test]
    fn applies_colours_and_keeps_the_rest_of_the_base_theme() {
        let theme = Theme::from_toml("mine", "dark = false\n[colors]\nbackground = \"#102030\"\n").unwrap();
        assert_eq!(theme.name, "mine");
        assert_eq!(theme.visuals.panel_fill, Color32::from_rgb(0x10, 0x20, 0x30));
        assert!(!theme.visuals.dark_mode);
        assert_eq!(theme.visuals.selection, Visuals::light().selection);
        assert_eq!(theme.success, DEFAULT_SUCCESS);
    }

    #[test]
    fn rejects_typos_and_bad_colours() {
        assert!(Theme::from_toml("x", "[colors]\nbackgrund = \"#000000\"\n").unwrap_err().contains("backgrund"));
        assert!(Theme::from_toml("x", "[colors]\ntext = \"blue\"\n").unwrap_err().contains("blue"));
    }

    #[test]
    fn creates_defaults_once_and_loads_the_folder() {
        let dir = test_dir("load");
        assert_eq!(create_default_themes(&dir), None);
        fs::remove_file(dir.join("nord.toml")).unwrap();
        create_default_themes(&dir);
        assert!(!dir.join("nord.toml").exists(), "a deleted theme should stay deleted");

        fs::write(dir.join("broken.toml"), "colors = 5").unwrap();
        fs::write(dir.join("dark.toml"), "name = \"My Dark\"\n").unwrap();
        let (themes, messages) = load_themes(&dir);
        let ids: Vec<&str> = themes.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["light", "dark", "gruvbox-dark", "gruvbox-light", "solarized-dark", "solarized-light"]);
        assert_eq!(themes[1].name, "My Dark", "a file can replace a built-in theme");
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("broken.toml"));
        assert_eq!(find(&themes, "Gruvbox-Dark").unwrap().name, "Gruvbox Dark");
        assert!(find(&themes, "missing").is_none());

        fs::remove_dir_all(dir).unwrap();
    }
}
