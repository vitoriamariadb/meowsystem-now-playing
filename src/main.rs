mod album_color;
mod faixa;
mod fonte;
mod i18n;
mod limpeza;
mod media;
mod model;
mod settings;
mod style;
mod tema;
mod ui;
mod window;

use crate::window::Window;

fn main() -> cosmic::iced::Result {
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    i18n::init(&requested_languages);

    cosmic::applet::run::<Window>(())?;

    Ok(())
}
