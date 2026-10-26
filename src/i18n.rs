// SPDX-License-Identifier: GPL-3.0-only

use std::sync::LazyLock;

use i18n_embed::{
    DefaultLocalizer, LanguageLoader, Localizer,
    fluent::{FluentLanguageLoader, fluent_language_loader},
    unic_langid::LanguageIdentifier,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

pub fn init(requested_languages: &[LanguageIdentifier]) {
    if let Err(why) = localizer().select(requested_languages) {
        eprintln!("error while loading fluent localizations: {why}");
    }
}

#[must_use]
pub fn localizer() -> Box<dyn Localizer> {
    Box::from(DefaultLocalizer::new(&*LANGUAGE_LOADER, &Localizations))
}

pub static LANGUAGE_LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader: FluentLanguageLoader = fluent_language_loader!();

    loader
        .load_fallback_language(&Localizations)
        .expect("Error while loading fallback language");

    loader
});

#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::i18n::LANGUAGE_LOADER, $message_id)
    }};

    ($message_id:literal, $($args:expr),*) => {{
        i18n_embed_fl::fl!($crate::i18n::LANGUAGE_LOADER, $message_id, $($args), *)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chaves(lingua: &str) -> Vec<String> {
        let arquivo = Localizations::get(&format!("{lingua}/meowsystem_now_playing.ftl"))
            .expect("arquivo de tradução");
        let texto = String::from_utf8(arquivo.data.into_owned()).expect("utf-8");
        texto
            .lines()
            .filter_map(|l| l.split_once(" = ").map(|(k, _)| k.trim().to_owned()))
            .collect()
    }

    #[test]
    fn pt_br_tem_todas_as_chaves_do_ingles() {
        let pt = chaves("pt-BR");
        let faltam: Vec<_> = chaves("en")
            .into_iter()
            .filter(|k| !pt.contains(k))
            .collect();
        assert!(faltam.is_empty(), "faltam em pt-BR: {faltam:?}");
    }

    #[test]
    fn pt_br_e_carregado_pelo_nome_do_locale() {
        let loader: FluentLanguageLoader = fluent_language_loader!();
        let pt: LanguageIdentifier = "pt-BR".parse().unwrap();
        loader.load_languages(&Localizations, &[pt]).unwrap();
        assert_eq!(loader.get("now-playing"), "Tocando agora");
    }
}
