use std::path::{Path, PathBuf};

use cosmic::iced::{Background, Border, Color, Shadow};
use cosmic::widget::{button, container};

pub const ARQUIVO: &str = "tema.conf";
pub const VARIAVEL: &str = "MEOWSYSTEM_TEMA";
pub const REALCE: f32 = 0.22;
pub const PRESSAO: f32 = 0.36;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tema {
    pub acento: Option<Color>,
    pub fundo: Option<Color>,
    pub borda: Option<Color>,
    pub texto: Option<Color>,
}

pub fn cor(texto: &str) -> Option<Color> {
    let hex = texto.trim().strip_prefix('#')?;
    if !(hex.len() == 6 || hex.len() == 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let canal = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    let alfa = if hex.len() == 8 { canal(6)? } else { 255 };
    Some(Color::from_rgba8(
        canal(0)?,
        canal(2)?,
        canal(4)?,
        f32::from(alfa) / 255.0,
    ))
}

impl Tema {
    #[must_use]
    pub fn ler(texto: &str) -> Self {
        let mut tema = Self::default();
        for linha in texto.lines().map(str::trim) {
            if linha.is_empty() || linha.starts_with('#') {
                continue;
            }
            let Some((chave, valor)) = linha.split_once('=') else {
                continue;
            };
            let Some(valor) = cor(valor) else {
                continue;
            };
            match chave.trim() {
                "acento" => tema.acento = Some(valor),
                "fundo" => tema.fundo = Some(valor),
                "borda" => tema.borda = Some(valor),
                "texto" => tema.texto = Some(valor),
                _ => {}
            }
        }
        tema
    }

    #[must_use]
    pub fn veste_o_cartao(&self) -> bool {
        self.fundo.is_some() || self.borda.is_some() || self.texto.is_some()
    }

    #[must_use]
    pub fn carregar_de(caminho: &Path) -> Self {
        match std::fs::read(caminho) {
            Ok(bytes) => Self::ler(&String::from_utf8_lossy(&bytes)),
            Err(erro) => {
                if erro.kind() != std::io::ErrorKind::NotFound {
                    eprintln!("tema ilegível em {}: {erro}", caminho.display());
                }
                Self::default()
            }
        }
    }

    #[must_use]
    pub fn carregar(app_id: &str) -> Self {
        caminho(app_id)
            .map(|caminho| Self::carregar_de(&caminho))
            .unwrap_or_default()
    }

    #[must_use]
    pub fn cartao(&self, theme: &cosmic::Theme) -> container::Style {
        let cosmic = theme.cosmic();
        let fundo = cosmic.background(theme.transparent);
        let tinta = self.texto.unwrap_or_else(|| fundo.on.into());
        container::Style {
            text_color: Some(tinta),
            background: Some(Background::Color(
                self.fundo.unwrap_or_else(|| fundo.base.into()),
            )),
            border: Border {
                radius: cosmic.corner_radii.radius_m.into(),
                width: 1.0,
                color: self.borda.unwrap_or_else(|| fundo.divider.into()),
            },
            shadow: Shadow::default(),
            icon_color: Some(tinta),
            snap: true,
        }
    }

    pub fn no_botao(&self, mut base: button::Style, realce: f32) -> button::Style {
        let Some(acento) = self.acento else {
            return base;
        };
        base.icon_color = Some(acento);
        if realce > 0.0 {
            base.background = Some(Background::Color(Color {
                a: acento.a * realce,
                ..acento
            }));
        }
        base
    }
}

#[must_use]
pub fn caminho(app_id: &str) -> Option<PathBuf> {
    if let Some(caminho) = std::env::var_os(VARIAVEL).filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(caminho));
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(sob(&base, app_id))
}

fn sob(base: &Path, app_id: &str) -> PathBuf {
    base.join("meowsystem")
        .join("extensoes")
        .join(app_id)
        .join(ARQUIVO)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(c: Color) -> [u8; 3] {
        let [r, g, b, _] = c.into_rgba8();
        [r, g, b]
    }

    #[test]
    fn cor_aceita_seis_e_oito_digitos_e_recusa_o_resto() {
        assert_eq!(cor("#cba6f7").map(rgb), Some([0xcb, 0xa6, 0xf7]));
        assert_eq!(cor(" #1e1e2ef0 ").map(|c| c.into_rgba8()[3]), Some(0xf0));
        for ruim in ["cba6f7", "#cba6f", "#cba6f7a", "#gggggg", "mauve", "", "#"] {
            assert_eq!(cor(ruim), None, "{ruim:?} passou");
        }
    }

    #[test]
    fn o_tema_conf_que_o_meowsystem_escreve_vira_as_quatro_cores() {
        let tema = Tema::ler(
            "fundo = #1e1e2e\n\
             borda = #45475a\n\
             texto = #cdd6f4\n\
             acento = #cba6f7\n",
        );
        assert_eq!(tema.acento.map(rgb), Some([0xcb, 0xa6, 0xf7]));
        assert_eq!(tema.fundo.map(rgb), Some([0x1e, 0x1e, 0x2e]));
        assert_eq!(tema.borda.map(rgb), Some([0x45, 0x47, 0x5a]));
        assert_eq!(tema.texto.map(rgb), Some([0xcd, 0xd6, 0xf4]));
        assert!(tema.veste_o_cartao());
    }

    #[test]
    fn linha_ruim_e_chave_nova_ficam_de_fora_sem_levar_as_boas() {
        let tema = Tema::ler(
            "# comentário\n\
             fundo = mauve\n\
             sem igual\n\
             bloco = #313244\n\
             acento = #ff79c6\n",
        );
        assert_eq!(tema.fundo, None);
        assert_eq!(tema.acento.map(rgb), Some([0xff, 0x79, 0xc6]));
        assert!(!tema.veste_o_cartao());
    }

    #[test]
    fn sem_arquivo_e_o_tema_do_cosmic() {
        let pasta = tempfile::tempdir().unwrap();
        assert_eq!(
            Tema::carregar_de(&pasta.path().join(ARQUIVO)),
            Tema::default()
        );
    }

    #[test]
    fn arquivo_binario_nao_derruba() {
        let pasta = tempfile::tempdir().unwrap();
        let arquivo = pasta.path().join(ARQUIVO);
        std::fs::write(&arquivo, [0xff, 0xfe, 0x00, 0x9f]).unwrap();
        assert_eq!(Tema::carregar_de(&arquivo), Tema::default());
    }

    #[test]
    fn o_caminho_mora_na_pasta_de_extensoes_do_meowsystem() {
        assert_eq!(
            sob(Path::new("/casa/.config"), crate::settings::APP_ID),
            PathBuf::from(
                "/casa/.config/meowsystem/extensoes/io.github.meowsystem.NowPlaying/tema.conf"
            )
        );
    }

    #[test]
    fn o_cartao_veste_o_fundo_e_a_borda_do_pack_e_sem_eles_fica_o_do_cosmic() {
        let theme = cosmic::Theme::dark();
        let tema = Tema::ler("fundo = #1e1e2e\nborda = #45475a\ntexto = #cdd6f4\n");
        let vestido = tema.cartao(&theme);
        assert_eq!(
            vestido.background,
            Some(Background::Color(cor("#1e1e2e").unwrap()))
        );
        assert_eq!(vestido.border.color, cor("#45475a").unwrap());
        assert_eq!(vestido.text_color, cor("#cdd6f4"));

        let cru = Tema::default().cartao(&theme);
        let fundo = theme.cosmic().background(theme.transparent);
        assert_eq!(cru.background, Some(Background::Color(fundo.base.into())));
        assert_eq!(cru.border.color, Color::from(fundo.divider));
    }

    #[test]
    fn o_botao_de_tocar_leva_o_acento_e_o_realce_e_sem_acento_nada_muda() {
        let base = button::Style::default();
        let tema = Tema::ler("acento = #cba6f7\n");

        let parado = tema.no_botao(base, 0.0);
        assert_eq!(parado.icon_color, cor("#cba6f7"));
        assert_eq!(parado.background, base.background);

        let realcado = tema.no_botao(base, REALCE);
        let Some(Background::Color(fundo)) = realcado.background else {
            panic!("o realce não pintou o fundo");
        };
        assert_eq!(rgb(fundo), [0xcb, 0xa6, 0xf7]);
        assert!((fundo.a - REALCE).abs() < 1e-6);

        let cru = Tema::default().no_botao(base, REALCE);
        assert_eq!(cru.icon_color, base.icon_color);
        assert_eq!(cru.background, base.background);
    }
}
