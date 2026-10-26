use cosmic::iced::{Background, Color};
use cosmic::widget::button;

use crate::settings::ColorStyle;

/// Tinge o botão da barra com a cor dominante da capa.
///
/// DOIS MODOS, E O PADRÃO MUDOU
///   `Chapado` é o comportamento do upstream: a cor entra no FUNDO do botão.
///   `Traco` — o padrão daqui — não encosta no fundo. Motivo medido: o
///   `vidro.sh` do `MeowSystem` escreve a opacidade do painel, e um fundo opaco
///   neste botão abre um retângulo sólido no meio da dock, que é o oposto da
///   folha visual dela ("o nosso tema é o traço, não o chapado").
///
/// `fundo` é a cor de fundo da configuração: quando existe, vale em qualquer
/// modo e o contraste do traço passa a ser medido contra ela.
pub fn applet_button(
    mut base: button::Style,
    color: Option<Color>,
    style: ColorStyle,
    fundo: Option<Color>,
) -> button::Style {
    if let Some(fundo) = fundo {
        base.background = Some(Background::Color(fundo));
    }
    let Some(album) = color else {
        return base;
    };

    let theme_base = match base.background {
        Some(Background::Color(color)) => color,
        _ => Color::from_rgb8(36, 38, 42),
    };

    match style {
        ColorStyle::Traco => {
            // A cor crua do álbum vive em luminância 0,38-0,62 (é o que o
            // `normalize_album_color` garante). Num painel escuro isso quase
            // sempre passa, mas "quase sempre" produz o pior defeito possível:
            // um traço que existe no código e não existe na tela. Daí o
            // `readable_on`, que só age quando o contraste não fecha.
            let tinta = readable_on(album, theme_base);
            base.border_width = base.border_width.max(1.0);
            base.border_color = with_alpha(tinta, 0.88);
            base.text_color = Some(tinta);
            base.icon_color = Some(tinta);
        }
        ColorStyle::Chapado => {
            let mixed = blend_color(theme_base, album, 0.36);
            let background = with_alpha(mixed, 0.64);
            let border = with_alpha(shift_color(album, -0.05), 0.82);
            let foreground = contrast_text_color(background);

            base.background = Some(Background::Color(background));
            base.border_width = base.border_width.max(1.0);
            base.border_color = border;
            base.text_color = Some(foreground);
            base.icon_color = Some(foreground);
        }
    }

    base
}

#[must_use]
pub fn shift_color(color: Color, amount: f32) -> Color {
    let adjust = |channel: f32| (channel + amount).clamp(0.0, 1.0);
    Color {
        r: adjust(color.r),
        g: adjust(color.g),
        b: adjust(color.b),
        a: color.a,
    }
}

/// Clareia (ou escurece) a cor até ela ter contraste WCAG AA sobre o fundo.
/// Anda para o lado OPOSTO ao do fundo, então serve para tema claro e escuro
/// sem uma segunda regra.
fn readable_on(color: Color, background: Color) -> Color {
    let rumo = if relative_luminance(background) > 0.5 {
        -1.0
    } else {
        1.0
    };

    let mut saida = color;
    for _ in 0..24 {
        if contrast_ratio(saida, background) >= 4.5 {
            break;
        }
        saida = shift_color(saida, 0.03 * rumo);
    }
    saida
}

fn relative_luminance(color: Color) -> f32 {
    fn linear(channel: f32) -> f32 {
        if channel <= 0.03928 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    }

    0.2126f32.mul_add(
        linear(color.r),
        0.7152f32.mul_add(linear(color.g), 0.0722 * linear(color.b)),
    )
}

fn contrast_ratio(a: Color, b: Color) -> f32 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    let (alto, baixo) = if la > lb { (la, lb) } else { (lb, la) };
    (alto + 0.05) / (baixo + 0.05)
}

fn with_alpha(mut color: Color, alpha: f32) -> Color {
    color.a = alpha.clamp(0.0, 1.0);
    color
}

fn blend_color(a: Color, b: Color, ratio: f32) -> Color {
    let ratio = ratio.clamp(0.0, 1.0);
    let inverse = 1.0 - ratio;
    Color {
        r: a.r.mul_add(inverse, b.r * ratio),
        g: a.g.mul_add(inverse, b.g * ratio),
        b: a.b.mul_add(inverse, b.b * ratio),
        a: a.a.mul_add(inverse, b.a * ratio),
    }
}

fn contrast_text_color(background: Color) -> Color {
    let luminance = 0.2126f32.mul_add(
        background.r,
        0.7152f32.mul_add(background.g, 0.0722 * background.b),
    );
    if luminance > 0.58 {
        Color::from_rgb8(17, 17, 17)
    } else {
        Color::WHITE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAINEL_ESCURO: Color = Color {
        r: 0.117,
        g: 0.117,
        b: 0.180,
        a: 1.0,
    };

    #[test]
    fn traco_nao_pinta_o_fundo() {
        let base = button::Style {
            background: Some(Background::Color(PAINEL_ESCURO)),
            ..button::Style::default()
        };
        let saida = applet_button(
            base,
            Some(Color::from_rgb8(200, 80, 120)),
            ColorStyle::Traco,
            None,
        );
        assert_eq!(saida.background, Some(Background::Color(PAINEL_ESCURO)));
        assert!(saida.icon_color.is_some());
    }

    #[test]
    fn chapado_pinta_o_fundo() {
        let base = button::Style {
            background: Some(Background::Color(PAINEL_ESCURO)),
            ..button::Style::default()
        };
        let saida = applet_button(
            base,
            Some(Color::from_rgb8(200, 80, 120)),
            ColorStyle::Chapado,
            None,
        );
        assert_ne!(saida.background, Some(Background::Color(PAINEL_ESCURO)));
    }

    #[test]
    fn traco_garante_contraste_de_album_escuro() {
        let quase_invisivel = Color::from_rgb8(30, 30, 46);
        let tinta = readable_on(quase_invisivel, PAINEL_ESCURO);
        assert!(contrast_ratio(tinta, PAINEL_ESCURO) >= 4.5);
    }

    #[test]
    fn fundo_da_configuracao_vale_sem_capa() {
        let fundo = Color::from_rgb8(0x1e, 0x1e, 0x2e);
        let saida = applet_button(
            button::Style::default(),
            None,
            ColorStyle::Traco,
            Some(fundo),
        );
        assert_eq!(saida.background, Some(Background::Color(fundo)));
    }

    #[test]
    fn sem_cor_de_album_nada_muda() {
        let base = button::Style::default();
        let saida = applet_button(base, None, ColorStyle::Traco, None);
        assert!((saida.border_width - button::Style::default().border_width).abs() < f32::EPSILON);
    }
}
