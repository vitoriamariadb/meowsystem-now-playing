use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use cosmic::iced::Color;

pub const DEFAULT_PANEL_TEXT_WIDTH: f32 = 440.0;

/// Como a cor do álbum entra na barra.
///
/// O upstream só conhece o `Chapado`: ele mistura a cor no FUNDO do botão
/// (`blend_color(theme_base, album, 0.36)` + alpha 0.64). Numa dock cuja
/// opacidade é escrita pelo `vidro.sh` do `MeowSystem`, isso abre um retângulo
/// sólido no meio da barra. O `Traco` — padrão daqui — deixa o fundo em paz e
/// gasta a cor onde ela se lê: contorno e glifos.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ColorStyle {
    #[default]
    Traco,
    Chapado,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppSettings {
    pub album_color_enabled: bool,
    pub panel_text_width: f32,
    pub panel_color_style: ColorStyle,
    pub album_art_remote: bool,
    /// `None` = deixa o painel escolher a escala tipográfica dele.
    pub panel_font_size: Option<f32>,
    pub title_color: Option<Color>,
    pub artist_color: Option<Color>,
    /// Desenhar ⏮⏸⏭ NA BARRA. O popup sempre os tem, independente disto.
    pub panel_controls: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            album_color_enabled: true,
            panel_text_width: DEFAULT_PANEL_TEXT_WIDTH,
            panel_color_style: ColorStyle::default(),
            album_art_remote: true,
            panel_font_size: None,
            title_color: None,
            artist_color: None,
            panel_controls: true,
        }
    }
}

#[must_use]
pub fn settings_path() -> Option<PathBuf> {
    side_file("album-color-enabled")
}

/// Cada opção é um arquivo de UMA linha em `~/.config/cosmic-ext-applet-now-playing/`.
/// Cada opção é um arquivo de UMA linha em `~/.config/cosmic-ext-applet-now-playing/`.
/// É o formato que o upstream já usa para `album-color-enabled`, e é o que deixa
/// o `meow_escrever` comparar por conteúdo e não escrever à toa.
#[must_use]
pub fn side_file(name: &str) -> Option<PathBuf> {
    dirs::config_dir().map(|path| path.join("cosmic-ext-applet-now-playing").join(name))
}

fn read_side(name: &str) -> Option<String> {
    fs::read_to_string(side_file(name)?)
        .ok()
        .map(|value| value.trim().to_owned())
}

fn read_width() -> f32 {
    read_side("panel-text-width")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|width| (80.0..=900.0).contains(width))
        .unwrap_or(DEFAULT_PANEL_TEXT_WIDTH)
}

fn read_color_style() -> ColorStyle {
    match read_side("panel-color-style").as_deref() {
        Some("chapado") => ColorStyle::Chapado,
        _ => ColorStyle::Traco,
    }
}

/// `auto` (ou ausente) devolve `None`, e aí quem manda no tamanho é o
/// `Context::text` do libcosmic — a MESMA escala tipográfica do relógio do
/// painel. O upstream não usava isso: passava `suggested_size().0 - 1`, que é o
/// tamanho do ÍCONE, para o tamanho da FONTE. Numa dock de ícone grande a letra
/// saía do tamanho do ícone, e foi a primeira coisa que ela reparou.
fn read_font_size() -> Option<f32> {
    read_side("panel-font-size")
        .filter(|value| value != "auto" && !value.is_empty())
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|size| (6.0..=48.0).contains(size))
}

/// Um `#rrggbb` por arquivo. Quem resolve nome de cor da paleta Catppuccin para
/// hex é o `midia.sh`, no lado do `MeowSystem`: a paleta é dele, e duplicá-la aqui
/// seria uma segunda verdade para o dia em que ela trocasse de flavor.
fn read_color(name: &str) -> Option<Color> {
    let valor = read_side(name)?;
    let hex = valor.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let n = u32::from_str_radix(hex, 16).ok()?;
    Some(Color::from_rgb8(
        ((n >> 16) & 0xFF) as u8,
        ((n >> 8) & 0xFF) as u8,
        (n & 0xFF) as u8,
    ))
}

/// Os ⏮⏸⏭ na barra. Existe porque o applet de Som do COSMIC JÁ desenha os dele
/// no painel quando há player — e dois pares lado a lado, medidos na topbar dela
/// em 24/08/2026, leem como defeito, não como recurso. Com isto desligado sobra
/// `[capa] música · banda`, que encosta nos controles do vizinho e forma um
/// bloco só. O popup do applet continua com os três botões de qualquer jeito.
#[must_use]
pub fn read_panel_controls() -> bool {
    !matches!(
        read_side("panel-controls").as_deref(),
        Some("false" | "0" | "nao" | "não")
    )
}

/// Desligar isto para o applet de baixar capa nenhuma, SEM recompilar.
/// A falta desse escape foi o motivo de a capa nunca ter existido aqui.
#[must_use]
pub fn read_album_art_remote() -> bool {
    !matches!(
        read_side("album-art-remote").as_deref(),
        Some("false" | "0" | "nao" | "não")
    )
}

pub fn load() -> AppSettings {
    let Some(path) = settings_path() else {
        eprintln!("unable to load settings: user config directory not found");
        return AppSettings::default();
    };

    match load_from(&path) {
        Ok(settings) => settings,
        Err(error) if error.kind() == io::ErrorKind::NotFound => AppSettings {
            panel_text_width: read_width(),
            panel_color_style: read_color_style(),
            album_art_remote: read_album_art_remote(),
            panel_font_size: read_font_size(),
            title_color: read_color("panel-title-color"),
            artist_color: read_color("panel-artist-color"),
            panel_controls: read_panel_controls(),
            ..AppSettings::default()
        },
        Err(error) => {
            eprintln!("unable to load settings from {}: {error}", path.display());
            AppSettings::default()
        }
    }
}

pub fn save(settings: AppSettings) -> io::Result<()> {
    let path = settings_path().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "user config directory not found")
    })?;
    save_to(&path, settings)
}

fn load_from(path: &Path) -> io::Result<AppSettings> {
    let value = fs::read_to_string(path)?;
    let album_color_enabled = match value.trim() {
        "true" => true,
        "false" => false,
        invalid => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid album color setting: {invalid}"),
            ));
        }
    };

    Ok(AppSettings {
        album_color_enabled,
        panel_text_width: read_width(),
        panel_color_style: read_color_style(),
        album_art_remote: read_album_art_remote(),
        panel_font_size: read_font_size(),
        title_color: read_color("panel-title-color"),
        artist_color: read_color("panel-artist-color"),
        panel_controls: read_panel_controls(),
    })
}

fn save_to(path: &Path, settings: AppSettings) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(
        path,
        if settings.album_color_enabled {
            "true\n"
        } else {
            "false\n"
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn round_trips_settings() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("nested/setting");
        let expected = AppSettings {
            album_color_enabled: false,
            ..AppSettings::default()
        };

        save_to(&path, expected).unwrap();

        assert_eq!(load_from(&path).unwrap().album_color_enabled, expected.album_color_enabled);
    }

    #[test]
    fn rejects_invalid_values() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("setting");
        fs::write(&path, "sometimes\n").unwrap();

        assert_eq!(
            load_from(&path).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn reports_write_failures() {
        let directory = tempdir().unwrap();

        assert!(save_to(directory.path(), AppSettings::default()).is_err());
    }
}
