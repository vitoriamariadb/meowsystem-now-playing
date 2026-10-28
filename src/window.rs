use cosmic::app::Core;
use cosmic::cosmic_config;
use cosmic::iced::{Limits, Subscription, window, window::Id};
use cosmic::surface::action::{app_popup, destroy_popup};
use cosmic::{Action, Element, Task};

use crate::media::{self, MediaCommand};
use crate::model::{AppModel, MediaSnapshot, PopupPage};
use crate::settings::{self, AppSettings, ColorStyle};
use crate::tema::Tema;
use crate::ui;

pub struct Window {
    core: Core,
    popup: Option<Id>,
    model: AppModel,
    /// O que está no disco, para a página de configurações mostrar e gravar.
    config: settings::Config,
    config_handle: Option<cosmic_config::Config>,
}

/// Uma opção mudada na página de configurações.
#[derive(Clone, Debug)]
pub enum Ajuste {
    CorDoAlbum(bool),
    CorDaFonte(bool),
    CapaRemota(bool),
    ControlesNaBarra(bool),
    Chapado(bool),
    Largura(u32),
    TamanhoDaFonte(u32),
    FamiliaDaFonte(String),
    CorDoTitulo(String),
    CorDoArtista(String),
    CorDoFundo(String),
}

#[derive(Clone, Debug)]
pub enum Message {
    TogglePopup,
    ShowMain,
    ShowPlayerList,
    ShowSettings,
    Ajustar(Ajuste),
    ConfigChanged,
    SelectPlayer(String),
    PopupClosed(Id),
    MediaChanged(MediaSnapshot),
    PreviousTrack,
    TogglePlayPause,
    NextTrack,
}

impl cosmic::Application for Window {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = settings::APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Action<Self::Message>>) {
        let config_handle = settings::abrir();
        let config = settings::carregar(config_handle.as_ref());
        let app_settings = AppSettings::from(&config);
        media::permitir_download(app_settings.album_art_remote);
        let mut model = AppModel::new(app_settings, media::initial_snapshot());
        model.tema = Tema::carregar(settings::APP_ID);
        (
            Self {
                core,
                popup: None,
                model,
                config,
                config_handle,
            },
            Task::none(),
        )
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn update(&mut self, message: Message) -> Task<Action<Self::Message>> {
        match message {
            Message::TogglePopup => return self.toggle_popup(),
            Message::ShowMain => self.model.set_popup_page(PopupPage::Main),
            Message::ShowPlayerList => self.model.set_popup_page(PopupPage::PlayerList),
            Message::ShowSettings => self.model.set_popup_page(PopupPage::Settings),
            Message::Ajustar(ajuste) => self.ajustar(ajuste),
            Message::ConfigChanged => {
                self.config = settings::carregar(self.config_handle.as_ref());
                self.aplicar_config();
            }
            Message::SelectPlayer(id) => {
                self.model.select_player(id);
                self.model.set_popup_page(PopupPage::Main);
            }
            Message::PopupClosed(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                    self.model.set_popup_page(PopupPage::Main);
                }
            }
            Message::MediaChanged(snapshot) => {
                self.model.apply_snapshot(snapshot);
                if !self.model.has_active_media()
                    && let Some(popup) = self.popup.take()
                {
                    self.model.set_popup_page(PopupPage::Main);
                    return surface_task(destroy_popup(popup));
                }
            }
            Message::PreviousTrack => self.run_media_command(MediaCommand::Previous),
            Message::TogglePlayPause => {
                self.run_media_command(MediaCommand::TogglePlayPause);
            }
            Message::NextTrack => self.run_media_command(MediaCommand::Next),
        }

        Task::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        // A atualização do watcher não traz o legado; relê do disco inteiro.
        Subscription::batch([
            media::subscription().map(Message::MediaChanged),
            self.core
                .watch_config::<settings::Config>(settings::APP_ID)
                .map(|_| Message::ConfigChanged),
        ])
    }

    fn view(&self) -> Element<'_, Message> {
        ui::panel(&self.core, &self.model)
    }

    fn view_window(&self, _id: Id) -> Element<'_, Message> {
        ui::popup(&self.core, &self.model, &self.config)
    }
}

impl Window {
    fn toggle_popup(&mut self) -> Task<Action<Message>> {
        if let Some(popup) = self.popup.take() {
            self.model.set_popup_page(PopupPage::Main);
            return surface_task(destroy_popup(popup));
        }

        self.model.tema = Tema::carregar(settings::APP_ID);

        let Some(parent) = self.core.main_window_id() else {
            eprintln!("unable to open popup: main applet window is unavailable");
            return Task::none();
        };

        surface_task(app_popup::<Window>(
            |_| Default::default(),
            move |state: &mut Window| {
                let popup = Id::unique();
                let mut popup_settings = state
                    .core
                    .applet
                    .get_popup_settings(parent, popup, None, None, None);
                popup_settings.positioner.size_limits = Limits::NONE
                    .max_width(370.0)
                    .min_width(200.0)
                    .min_height(200.0)
                    .max_height(1080.0);
                state.popup = Some(popup);

                popup_settings
            },
            Some(Box::new(|state: &Window| {
                ui::popup(&state.core, &state.model, &state.config).map(cosmic::Action::App)
            })),
        ))
    }

    fn aplicar_config(&mut self) {
        let app_settings = AppSettings::from(&self.config);
        media::permitir_download(app_settings.album_art_remote);
        self.model.set_settings(app_settings);
    }

    fn ajustar(&mut self, ajuste: Ajuste) {
        let Some(handle) = self.config_handle.as_ref() else {
            eprintln!("unable to save settings: user config directory not found");
            return;
        };
        let c = &mut self.config;
        let resultado = match ajuste {
            Ajuste::CorDoAlbum(v) => c.set_album_color_enabled(handle, v),
            Ajuste::CorDaFonte(v) => c.set_source_colors(handle, v),
            Ajuste::CapaRemota(v) => c.set_album_art_remote(handle, v),
            Ajuste::ControlesNaBarra(v) => c.set_panel_controls(handle, v),
            Ajuste::Chapado(v) => {
                let estilo = if v {
                    ColorStyle::Chapado
                } else {
                    ColorStyle::Traco
                };
                c.set_panel_color_style(handle, estilo.como_texto().to_owned())
            }
            Ajuste::Largura(px) => c.set_panel_text_width(handle, settings::largura_valida(px)),
            Ajuste::TamanhoDaFonte(px) => {
                // Abaixo do mínimo é «automático»: subir de 0 pula para o mínimo,
                // descer do mínimo volta a 0.
                let px = match px {
                    0 => 0,
                    px if px < settings::FONTE_MIN && c.panel_font_size == 0 => settings::FONTE_MIN,
                    px if px < settings::FONTE_MIN => 0,
                    px => px.min(settings::FONTE_MAX),
                };
                c.set_panel_font_size(handle, px)
            }
            Ajuste::FamiliaDaFonte(nome) => c.set_panel_font_family(handle, nome),
            Ajuste::CorDoTitulo(hex) => c.set_title_color(handle, hex),
            Ajuste::CorDoArtista(hex) => c.set_artist_color(handle, hex),
            Ajuste::CorDoFundo(hex) => c.set_background_color(handle, hex),
        };
        if let Err(error) = resultado {
            eprintln!("unable to save settings: {error}");
        }
        self.aplicar_config();
    }

    fn run_media_command(&self, command: MediaCommand) {
        if let Err(error) = media::run_command(self.model.selected_player.as_deref(), command) {
            eprintln!("{error}");
        }
    }
}

fn surface_task(action: cosmic::surface::Action) -> Task<Action<Message>> {
    cosmic::task::message(cosmic::Action::Cosmic(cosmic::app::Action::Surface(action)))
}
