//! A configuração do applet: um `cosmic-config` em
//! `~/.config/cosmic/io.github.meowsystem.NowPlaying/v1/`, uma chave por
//! arquivo, em RON. O contrato com quem escreve (o `MeowSystem`) está em
//! `INTEGRACAO.md`; mudar nome ou tipo de chave aqui é mudar o contrato.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use cosmic::cosmic_config::{
    self, ConfigGet, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry,
};
use cosmic::iced::{Color, Font};
use serde::{Deserialize, Serialize};

pub const APP_ID: &str = "io.github.meowsystem.NowPlaying";

pub const LARGURA_PADRAO: u32 = 440;
pub const LARGURA_MIN: u32 = 80;
pub const LARGURA_MAX: u32 = 900;
pub const FONTE_MIN: u32 = 6;
pub const FONTE_MAX: u32 = 48;

/// Onde o patch antigo do `MeowSystem` lia as opções: um arquivo de uma linha
/// por chave. Só vale enquanto a chave nova não existir.
const DIR_LEGADO: &str = "cosmic-ext-applet-now-playing";

/// Como a cor do álbum entra na barra.
///
/// `Chapado` é o comportamento do upstream: a cor entra no FUNDO do botão.
/// `Traco` (padrão) deixa o fundo em paz e pinta contorno e glifos, para não
/// abrir um retângulo opaco num painel translúcido.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ColorStyle {
    #[default]
    Traco,
    Chapado,
}

impl ColorStyle {
    #[must_use]
    pub fn de_texto(valor: &str) -> Self {
        if valor.trim().eq_ignore_ascii_case("chapado") {
            Self::Chapado
        } else {
            Self::Traco
        }
    }

    #[must_use]
    pub const fn como_texto(self) -> &'static str {
        match self {
            Self::Traco => "traco",
            Self::Chapado => "chapado",
        }
    }
}

/// O que mora no disco. Cada campo é uma chave do `cosmic-config`.
// Cada bool é uma chave independente do cosmic-config; um enum de estados não caberia.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CosmicConfigEntry)]
#[version = 1]
#[serde(default)]
pub struct Config {
    /// `"#rrggbb"`; vazio = cor do tema.
    pub title_color: String,
    /// `"#rrggbb"`; vazio = cor do tema.
    pub artist_color: String,
    /// `"#rrggbb"` ou `"#rrggbbaa"`; vazio = fundo do painel.
    pub background_color: String,
    /// Teto, em px lógicos, do texto na barra (80 a 900).
    pub panel_text_width: u32,
    /// Tamanho da letra em px (6 a 48); 0 = a escala do painel.
    pub panel_font_size: u32,
    /// Família da fonte; vazio = a do tema.
    pub panel_font_family: String,
    /// `"traco"` ou `"chapado"`.
    pub panel_color_style: String,
    /// Tingir o botão com a cor da capa.
    pub album_color_enabled: bool,
    /// Pintar o título com a cor da marca do serviço que toca.
    pub source_colors: bool,
    /// Baixar capa `http(s)`.
    pub album_art_remote: bool,
    /// Anterior, tocar/pausar e próxima na barra (o popup sempre os tem).
    pub panel_controls: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            title_color: String::new(),
            artist_color: String::new(),
            background_color: String::new(),
            panel_text_width: LARGURA_PADRAO,
            panel_font_size: 0,
            panel_font_family: String::new(),
            panel_color_style: ColorStyle::Traco.como_texto().to_owned(),
            album_color_enabled: true,
            source_colors: true,
            album_art_remote: true,
            panel_controls: false,
        }
    }
}

/// A configuração já interpretada, no formato que a interface usa.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppSettings {
    pub album_color_enabled: bool,
    pub source_colors: bool,
    pub panel_text_width: f32,
    pub panel_color_style: ColorStyle,
    pub album_art_remote: bool,
    /// `None` = deixa o painel escolher a escala tipográfica dele.
    pub panel_font_size: Option<f32>,
    pub panel_font: Option<Font>,
    pub title_color: Option<Color>,
    pub artist_color: Option<Color>,
    pub background_color: Option<Color>,
    pub panel_controls: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self::from(&Config::default())
    }
}

impl From<&Config> for AppSettings {
    fn from(c: &Config) -> Self {
        Self {
            album_color_enabled: c.album_color_enabled,
            source_colors: c.source_colors,
            #[allow(clippy::cast_precision_loss)]
            panel_text_width: largura_valida(c.panel_text_width) as f32,
            panel_color_style: ColorStyle::de_texto(&c.panel_color_style),
            album_art_remote: c.album_art_remote,
            #[allow(clippy::cast_precision_loss)]
            panel_font_size: fonte_valida(c.panel_font_size).map(|px| px as f32),
            panel_font: fonte_por_nome(&c.panel_font_family),
            title_color: cor_de_hex(&c.title_color),
            artist_color: cor_de_hex(&c.artist_color),
            background_color: cor_de_hex(&c.background_color),
            panel_controls: c.panel_controls,
        }
    }
}

#[must_use]
pub fn largura_valida(px: u32) -> u32 {
    if (LARGURA_MIN..=LARGURA_MAX).contains(&px) {
        px
    } else {
        LARGURA_PADRAO
    }
}

#[must_use]
pub fn fonte_valida(px: u32) -> Option<u32> {
    (FONTE_MIN..=FONTE_MAX).contains(&px).then_some(px)
}

/// `#rrggbb`, `rrggbb` ou `#rrggbbaa`. Qualquer outra coisa (vazio, `auto`,
/// nome de cor) é `None`: quem traduz nome de paleta é quem escreve a chave.
#[must_use]
pub fn cor_de_hex(valor: &str) -> Option<Color> {
    let hex = valor.trim().trim_start_matches('#');
    if !(hex.len() == 6 || hex.len() == 8) || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    let mut cor = Color::from_rgb8(byte(0)?, byte(2)?, byte(4)?);
    if hex.len() == 8 {
        cor.a = f32::from(byte(6)?) / 255.0;
    }
    Some(cor)
}

/// O `iced` só aceita nome de família `'static`. Cada nome distinto é vazado
/// uma vez e reaproveitado, então trocar de fonte não cresce a memória.
fn fonte_por_nome(nome: &str) -> Option<Font> {
    static NOMES: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let nome = nome.trim();
    if nome.is_empty() {
        return None;
    }
    let mut nomes = NOMES.lock().unwrap_or_else(PoisonError::into_inner);
    let fixo = if let Some(n) = nomes.iter().find(|n| **n == nome) {
        *n
    } else {
        let n: &'static str = Box::leak(nome.to_owned().into_boxed_str());
        nomes.push(n);
        n
    };
    Some(Font::with_name(fixo))
}

#[must_use]
pub fn abrir() -> Option<cosmic_config::Config> {
    match cosmic_config::Config::new(APP_ID, Config::VERSION) {
        Ok(config) => Some(config),
        Err(error) => {
            eprintln!("unable to open config {APP_ID}: {error}");
            None
        }
    }
}

#[must_use]
pub fn dir_legado() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join(DIR_LEGADO))
}

/// Lê a configuração; chave ausente cai no arquivo do patch antigo, se houver,
/// e por fim no padrão.
#[must_use]
pub fn carregar(config: Option<&cosmic_config::Config>) -> Config {
    let Some(config) = config else {
        return Config::default();
    };
    carregar_de(config, dir_legado().as_deref())
}

#[must_use]
pub fn carregar_de(config: &cosmic_config::Config, legado: Option<&Path>) -> Config {
    let mut lido = match Config::get_entry(config) {
        Ok(lido) => lido,
        Err((erros, lido)) => {
            for erro in erros.iter().filter(|e| e.is_err() && !e_ausente(e)) {
                eprintln!("config {APP_ID}: {erro}");
            }
            lido
        }
    };
    if let Some(dir) = legado {
        aplicar_legado(&mut lido, config, dir);
    }
    lido
}

fn e_ausente(erro: &cosmic_config::Error) -> bool {
    match erro {
        cosmic_config::Error::NotFound | cosmic_config::Error::NoConfigDirectory => true,
        cosmic_config::Error::GetKey(_, io) => io.kind() == std::io::ErrorKind::NotFound,
        _ => false,
    }
}

fn chave_ausente(config: &cosmic_config::Config, chave: &str) -> bool {
    match config.get::<serde::de::IgnoredAny>(chave) {
        Ok(_) => false,
        Err(erro) => e_ausente(&erro),
    }
}

fn ler_legado(dir: &Path, nome: &str) -> Option<String> {
    fs::read_to_string(dir.join(nome))
        .ok()
        .map(|v| v.trim().to_owned())
}

fn booleano_legado(valor: &str) -> Option<bool> {
    match valor {
        "true" | "1" | "sim" => Some(true),
        "false" | "0" | "nao" | "não" => Some(false),
        _ => None,
    }
}

fn aplicar_legado(lido: &mut Config, config: &cosmic_config::Config, dir: &Path) {
    if !dir.is_dir() {
        return;
    }
    let ausente = |chave: &str| chave_ausente(config, chave);
    let cor = |valor: String| -> Option<String> {
        cor_de_hex(&valor).map(|_| format!("#{}", valor.trim_start_matches('#').to_lowercase()))
    };

    if ausente("panel_text_width")
        && let Some(v) = ler_legado(dir, "panel-text-width").and_then(|v| v.parse::<f32>().ok())
    {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px = v.round().max(0.0) as u32;
        lido.panel_text_width = largura_valida(px);
    }
    if ausente("panel_color_style")
        && let Some(v) = ler_legado(dir, "panel-color-style")
    {
        ColorStyle::de_texto(&v)
            .como_texto()
            .clone_into(&mut lido.panel_color_style);
    }
    if ausente("panel_font_size")
        && let Some(v) = ler_legado(dir, "panel-font-size")
    {
        lido.panel_font_size = v.parse::<f32>().ok().map_or(0, |px| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let px = px.round().max(0.0) as u32;
            fonte_valida(px).unwrap_or(0)
        });
    }
    if ausente("title_color")
        && let Some(v) = ler_legado(dir, "panel-title-color").and_then(cor)
    {
        lido.title_color = v;
    }
    if ausente("artist_color")
        && let Some(v) = ler_legado(dir, "panel-artist-color").and_then(cor)
    {
        lido.artist_color = v;
    }
    let booleanos: [(&str, &str, &mut bool); 3] = [
        (
            "album_art_remote",
            "album-art-remote",
            &mut lido.album_art_remote,
        ),
        ("panel_controls", "panel-controls", &mut lido.panel_controls),
        (
            "album_color_enabled",
            "album-color-enabled",
            &mut lido.album_color_enabled,
        ),
    ];
    for (chave, arquivo, campo) in booleanos {
        if ausente(chave)
            && let Some(v) = ler_legado(dir, arquivo)
                .as_deref()
                .and_then(booleano_legado)
        {
            *campo = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic::cosmic_config::ConfigSet;
    use tempfile::{TempDir, tempdir};

    fn config_temporaria() -> (TempDir, cosmic_config::Config) {
        let dir = tempdir().unwrap();
        let config =
            cosmic_config::Config::with_custom_path(APP_ID, Config::VERSION, dir.path().into())
                .unwrap();
        (dir, config)
    }

    #[test]
    fn sem_nada_no_disco_vale_o_padrao() {
        let (_dir, config) = config_temporaria();
        let lido = carregar_de(&config, None);
        assert_eq!(lido, Config::default());
        let s = AppSettings::from(&lido);
        assert!((s.panel_text_width - 440.0).abs() < f32::EPSILON);
        assert_eq!(s.panel_font_size, None);
        assert_eq!(s.title_color, None);
        assert!(!s.panel_controls);
        assert!(s.source_colors);
    }

    #[test]
    fn le_as_chaves_escritas_em_ron() {
        let (dir, config) = config_temporaria();
        let raiz = dir.path().join("cosmic").join(APP_ID).join("v1");
        fs::write(raiz.join("title_color"), "\"#cba6f7\"").unwrap();
        fs::write(raiz.join("artist_color"), "\"#a6e3a1\"").unwrap();
        fs::write(raiz.join("panel_text_width"), "520").unwrap();
        fs::write(raiz.join("panel_font_size"), "14").unwrap();
        fs::write(raiz.join("panel_color_style"), "\"chapado\"").unwrap();
        fs::write(raiz.join("panel_controls"), "true").unwrap();
        let s = AppSettings::from(&carregar_de(&config, None));
        assert_eq!(s.title_color, Some(Color::from_rgb8(0xcb, 0xa6, 0xf7)));
        assert_eq!(s.artist_color, Some(Color::from_rgb8(0xa6, 0xe3, 0xa1)));
        assert!((s.panel_text_width - 520.0).abs() < f32::EPSILON);
        assert_eq!(s.panel_font_size, Some(14.0));
        assert_eq!(s.panel_color_style, ColorStyle::Chapado);
        assert!(s.panel_controls);
    }

    #[test]
    fn valor_fora_da_faixa_volta_ao_padrao() {
        let (_dir, config) = config_temporaria();
        config.set("panel_text_width", 5000_u32).unwrap();
        config.set("panel_font_size", 3_u32).unwrap();
        config.set("title_color", "mauve").unwrap();
        let s = AppSettings::from(&carregar_de(&config, None));
        assert!((s.panel_text_width - 440.0).abs() < f32::EPSILON);
        assert_eq!(s.panel_font_size, None);
        assert_eq!(s.title_color, None);
    }

    #[test]
    fn setter_grava_e_recarrega() {
        let (_dir, config) = config_temporaria();
        let mut c = Config::default();
        c.set_title_color(&config, "#f38ba8".into()).unwrap();
        c.set_source_colors(&config, false).unwrap();
        let lido = carregar_de(&config, None);
        assert_eq!(lido.title_color, "#f38ba8");
        assert!(!lido.source_colors);
    }

    #[test]
    fn legado_do_patch_vale_ate_a_chave_nova_existir() {
        let (_dir, config) = config_temporaria();
        let legado = tempdir().unwrap();
        for (nome, valor) in [
            ("panel-text-width", "440"),
            ("panel-color-style", "traco"),
            ("album-art-remote", "true"),
            ("panel-font-size", "auto"),
            ("panel-title-color", "#cba6f7"),
            ("panel-artist-color", "#a6e3a1"),
            ("panel-controls", "false"),
            ("album-color-enabled", "false"),
        ] {
            fs::write(legado.path().join(nome), format!("{valor}\n")).unwrap();
        }
        let lido = carregar_de(&config, Some(legado.path()));
        assert_eq!(lido.title_color, "#cba6f7");
        assert_eq!(lido.artist_color, "#a6e3a1");
        assert_eq!(lido.panel_font_size, 0);
        assert!(!lido.album_color_enabled);

        config.set("title_color", "#89b4fa").unwrap();
        let lido = carregar_de(&config, Some(legado.path()));
        assert_eq!(lido.title_color, "#89b4fa", "a chave nova vence o legado");
        assert_eq!(lido.artist_color, "#a6e3a1");
    }

    #[test]
    fn cores_aceitas() {
        assert_eq!(cor_de_hex("#000000"), Some(Color::from_rgb8(0, 0, 0)));
        assert_eq!(cor_de_hex("ff0000"), Some(Color::from_rgb8(255, 0, 0)));
        let translucida = cor_de_hex("#1e1e2e80").unwrap();
        assert!((translucida.a - 128.0 / 255.0).abs() < 1e-6);
        for ruim in ["", "auto", "#fff", "#gggggg", "mauve"] {
            assert_eq!(cor_de_hex(ruim), None, "{ruim:?}");
        }
    }

    #[test]
    fn fonte_vazia_e_a_do_tema() {
        assert_eq!(fonte_por_nome("  "), None);
        assert_eq!(fonte_por_nome("Inter"), fonte_por_nome("Inter"));
    }
}
