use cosmic::Element;
use cosmic::app::Core;
use cosmic::iced::advanced::text::{Ellipsize, EllipsizeHeightLimit, Wrapping};
use cosmic::iced::{Background, Border, Color};
use cosmic::iced::{ContentFit, Length};
use cosmic::widget::{
    Column, Row, button, button::Catalog, container, icon, mouse_area, scrollable, settings,
    slider, spin_button, text, text_input, toggler,
};

use crate::fl;
use crate::model::{AppModel, PlaybackState, PopupPage};
use crate::settings::{self as config, ColorStyle};
use crate::style::{applet_button, shift_color};
use crate::tema::{self, Tema};
use crate::window::{Ajuste, Message};

#[allow(clippy::too_many_lines)]
pub fn panel<'a>(core: &Core, model: &'a AppModel) -> Element<'a, Message> {
    if !model.has_active_media() {
        return core.applet.autosize_window(text("")).into();
    }

    let size = core.applet.suggested_size(true);
    let padding = core.applet.suggested_padding(true);
    let transport = mouse_area(icon::from_name(transport_icon(model)).size(size.0))
        .on_press(Message::TogglePlayPause);

    let previous = mouse_area(icon::from_name("media-skip-backward-symbolic").size(size.0))
        .on_press(Message::PreviousTrack);
    let next = mouse_area(icon::from_name("media-skip-forward-symbolic").size(size.0))
        .on_press(Message::NextTrack);

    let mut content = Row::new()
        .spacing(padding.0)
        .padding([0, padding.0])
        .align_y(cosmic::iced::alignment::Vertical::Center);

    if let Some(path) = model.now_playing.album_art_path.as_ref() {
        content = content.push(
            icon::icon(icon::from_path(path.clone()))
                .height(Length::Fixed(f32::from(size.0)))
                .width(Length::Fixed(f32::from(size.0)))
                .content_fit(ContentFit::Cover),
        );
    }

    // ORDEM: capa, nome, controles. A capa encosta no título porque as duas
    // falam da MESMA música; os controles vão para a ponta, longe do texto que
    // trunca. Pôr os botões no meio (como a primeira versão fazia) separava a
    // capa da música e empurrava o nome cortado contra a Status Area vizinha.
    //
    // O TAMANHO DA FONTE VEM DO PAINEL, NÃO DO ÍCONE.
    //   O upstream fazia `.size(suggested_size().0 - 1)` — isto é, usava o
    //   tamanho do ÍCONE como tamanho da FONTE. Numa dock de ícone grande a
    //   letra sai gigante ao lado do relógio da topbar. `core.applet.text()` é
    //   o helper do libcosmic que escolhe body/title4/title3/title2 conforme o
    //   painel, e é o que o relógio usa. `MIDIA_FONTE` sobrepõe em px quando ela
    //   quiser cravar.
    //
    // TÍTULO E ARTISTA SÃO DOIS TEXTOS, NÃO UM.
    //   O upstream junta tudo em `now_playing.text` ("Título - Artista"), e aí
    //   não há como dar cor diferente para cada um. Aqui eles são widgets
    //   separados dentro de um Row de largura FIXA.
    //
    // `panel_text_width` É TETO, NÃO TAMANHO — E A DIFERENÇA CUSTOU UMA BARRA.
    //   Duas versões erraram antes desta. A primeira dividia a largura em
    //   porções fixas (62/38): com título curto sobrava um BURACO visível antes
    //   do `·`, e o artista parecia solto, de outra música. A segunda pôs os
    //   textos em `Shrink` com um espaço `Fill` no fim — o buraco sumiu, mas o
    //   Row continuava `Fixed`, então o applet RESERVAVA 440px estivesse o
    //   texto ocupando isso ou não. MEDIDO em 24/08/2026, ao mudá-lo para a
    //   topbar: os 440px empurraram bluetooth, volume e rede para o menu `⋯` de
    //   estouro do painel. Numa dock larga isso não aparecia; numa topbar
    //   cheia, aparece na hora.
    //
    //   Agora o Row é `Shrink` dentro de um `container` com `max_width`: o
    //   applet mede o próprio texto e ocupa só isso, e a largura do `meow.conf`
    //   passa a ser o LIMITE em que o título começa a elipsar. Curto não rouba
    //   espaço de ninguém; longo não empurra os controles para fora.
    let escrever = |conteudo: &'a str, cor: Option<cosmic::iced::Color>| {
        let mut t = core
            .applet
            .text(conteudo)
            .width(Length::Shrink)
            .wrapping(Wrapping::None)
            .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)));
        if let Some(px) = model.settings.panel_font_size {
            t = t.size(px);
        }
        if let Some(fonte) = model.settings.panel_font {
            t = t.font(fonte);
        }
        if let Some(c) = cor {
            t = t.class(cosmic::theme::Text::Color(c));
        }
        t
    };

    let mut nome = Row::new()
        .width(Length::Shrink)
        .align_y(cosmic::iced::alignment::Vertical::Center)
        .push(escrever(
            model.now_playing.title.as_str(),
            model.title_color(),
        ));
    // Sem artista não há separador: «Filme ·» lê como texto cortado.
    if !model.now_playing.artist.is_empty() {
        let mut separador = core.applet.text(" · ").wrapping(Wrapping::None);
        if let Some(px) = model.settings.panel_font_size {
            separador = separador.size(px);
        }
        if let Some(fonte) = model.settings.panel_font {
            separador = separador.font(fonte);
        }
        nome = nome.push(separador).push(escrever(
            model.now_playing.artist.as_str(),
            model.settings.artist_color,
        ));
    }

    let nome = cosmic::widget::container(nome).max_width(model.settings.panel_text_width);

    let mut content = content.push(nome);
    // Os botões só entram se ela quiser. Ver `read_panel_controls`: na topbar
    // dela o applet de Som já desenha os dele um dedo ao lado, e o par dobrado
    // lê como defeito. O popup continua com os três, sempre.
    if model.settings.panel_controls {
        content = content.push(previous).push(transport).push(next);
    }
    let content = content;

    let album_color = model.album_color;
    let color_style = model.settings.panel_color_style;
    let fundo = model.settings.background_color;
    let button = button::custom(content)
        .width(Length::Shrink)
        .height(Length::Shrink)
        .name(fl!("now-playing"))
        .class(cosmic::theme::Button::Custom {
            active: Box::new(move |focused, theme| {
                let base = theme.active(focused, false, &cosmic::theme::Button::AppletIcon);
                applet_button(base, album_color, color_style, fundo)
            }),
            disabled: Box::new(move |theme| {
                let base = theme.disabled(&cosmic::theme::Button::AppletIcon);
                applet_button(base, album_color, color_style, fundo)
            }),
            hovered: Box::new(move |focused, theme| {
                let base = theme.hovered(focused, false, &cosmic::theme::Button::AppletIcon);
                applet_button(
                    base,
                    album_color.map(|color| shift_color(color, 0.07)),
                    color_style,
                    fundo.map(|color| shift_color(color, 0.05)),
                )
            }),
            pressed: Box::new(move |focused, theme| {
                let base = theme.pressed(focused, false, &cosmic::theme::Button::AppletIcon);
                applet_button(
                    base,
                    album_color.map(|color| shift_color(color, -0.08)),
                    color_style,
                    fundo.map(|color| shift_color(color, -0.05)),
                )
            }),
        })
        .on_press(Message::TogglePopup);

    core.applet.autosize_window(button).into()
}

pub fn popup<'a>(
    core: &Core,
    model: &'a AppModel,
    config: &'a config::Config,
) -> Element<'a, Message> {
    if !model.has_active_media() {
        return core.applet.popup_container(text("")).into();
    }

    let content = match model.popup_page {
        PopupPage::Main => main_popup(core, model),
        PopupPage::PlayerList => player_list(core, model),
        PopupPage::Settings => settings_page(core, model, config),
    };

    if !model.tema.veste_o_cartao() {
        return core.applet.popup_container(content).into();
    }
    let tema = model.tema;
    let cartao = container(content)
        .width(Length::Fill)
        .class(cosmic::theme::Container::custom(move |theme| {
            tema.cartao(theme)
        }));
    core.applet.popup_container(cartao).into()
}

// A LARGURA DO POPUP NÃO É NOSSA — E NÃO É A `MIDIA_LARGURA`.
//   Quem crava é o libcosmic, em `applet::Context::popup_container`:
//   `Limits::NONE.min_width(360.0).max_width(360.0)`. Mínimo IGUAL ao máximo,
//   360px lógicos, sempre. O `size_limits` que o `window.rs` entrega ao
//   positioner (min 200 / max 370) descreve só o retângulo do Wayland e nunca
//   chega à medição dos widgets. E `MIDIA_LARGURA` (440) é o TETO DO TEXTO NA
//   BARRA — `panel_text_width`, usado lá em cima no `panel()` — e não tem
//   caminho nenhum até aqui.
//
//   MEDIDO na captura de 25/08/2026 (tela dela, painel `S`, escala ~1,04): o
//   cartão do popup media 375px de tela e a capa 84px. `375/360 = 1,042` e
//   `84/80 = 1,05` — o MESMO fator. Ou seja: o cartão já ia de borda a borda
//   nos 360 do libcosmic. O que faltava não era largura, era o que ocupasse
//   ela.
//
// POR QUE O CONTEÚDO PARECIA "SÓ NA METADE ESQUERDA"
//   Nada aqui estava furado por falta de `Length::Fill` — o `media_row` já
//   herdava `Fill` do filho (o `push` do iced faz `self.width.enclose(child)`,
//   `iced/widget/src/row.rs:136`), então a coluna de texto já media os 244px
//   que sobram da capa. O buraco era de COMPOSIÇÃO: dos 360px de cartão só o
//   ~172px de tinta à esquerda tinha conteúdo, e a única coisa ancorada na
//   borda direita era a engrenagem, sozinha num andar próprio. Título curto
//   como "All I Do" alinhado à esquerda num nó de 244px lê como buraco.
//
//   O conserto é distribuir, não alargar: a engrenagem desce para a LINHA DO
//   TÍTULO (que já é `Fill`, então ela encosta na mesma margem direita que o
//   resto) e os controles entram DENTRO da coluna de texto, herdando a margem
//   esquerda do título. Some o andar vazio de cima, some a faixa morta debaixo
//   da capa, e a altura da coluna passa a bater com a da capa.
fn main_popup<'a>(core: &Core, model: &'a AppModel) -> Element<'a, Message> {
    let size = core.applet.suggested_size(true);
    let padding = core.applet.suggested_padding(true);
    let artwork_size = size.0.saturating_mul(4);

    let artwork = model.now_playing.album_art_path.as_ref().map_or_else(
        || {
            icon::from_name("audio-x-generic-symbolic")
                .size(artwork_size)
                .icon()
                .height(Length::Fixed(f32::from(artwork_size)))
                .width(Length::Fixed(f32::from(artwork_size)))
                .content_fit(ContentFit::Contain)
        },
        |path| {
            icon::icon(icon::from_path(path.clone()))
                .height(Length::Fixed(f32::from(artwork_size)))
                .width(Length::Fixed(f32::from(artwork_size)))
                .content_fit(ContentFit::Contain)
        },
    );

    // AS AÇÕES (lista de players + engrenagem) MORAM NA LINHA DO TÍTULO.
    //   Antes eram um `Row` inteiro só para elas, com um `text("")` de espaçador
    //   `Fill` e um segundo `text("")` de largura fixa no lugar do botão de
    //   players quando só há um — um ícone solto num andar sem nada, custando
    //   ~44px de altura para não alinhar com coisa nenhuma. Na linha do título
    //   a engrenagem termina exatamente onde o título, o artista e os controles
    //   terminam: uma margem direita só, para tudo. E o placeholder invisível
    //   deixa de existir — o botão de players simplesmente não é empurrado.
    let mut acoes = Row::new()
        .spacing(4)
        .align_y(cosmic::iced::alignment::Vertical::Center);
    if model.players.len() > 1 {
        acoes = acoes.push(accessible_icon_button_without_tooltip(
            "view-list-symbolic",
            size.0,
            fl!("select-player"),
            Message::ShowPlayerList,
        ));
    }
    let acoes = acoes.push(accessible_icon_button_without_tooltip(
        "preferences-system-symbolic",
        size.0,
        fl!("settings"),
        Message::ShowSettings,
    ));

    // `Fill` EXPLÍCITO, e não herdado. O `Row::push` do iced já promove a
    // largura do pai para `Fill` quando um filho é `Fill`, mas isso é efeito
    // colateral de quem se empurra dentro — trocar a ordem dos `push`, ou o
    // título deixar de ser `Fill`, mudaria a largura do Row sem ninguém pedir.
    // Escrito, o contrato fica: esta linha ocupa o que o popup der, o título
    // fica com a sobra e as ações encostam na borda.
    let titulo = Row::new()
        .width(Length::Fill)
        .spacing(8)
        .align_y(cosmic::iced::alignment::Vertical::Center)
        .push(
            text(model.now_playing.title.as_str())
                .size(size.0.saturating_sub(1))
                .width(Length::Fill)
                .wrapping(Wrapping::WordOrGlyph),
        )
        .push(acoes);

    // OS CONTROLES DENTRO DA COLUNA DE TEXTO, E NÃO NUM ANDAR PRÓPRIO.
    //   Num andar próprio eles caíam no `align_x(Center)` da coluna de fora e
    //   ficavam centrados no CARTÃO INTEIRO: medido na captura, o centro dos
    //   três botões batia em 1153px e o centro do cartão em 1156px — três
    //   pixels de diferença, ou seja, alinhados com o popup e não com o texto.
    //   O efeito colateral era a faixa morta debaixo da capa, que é alta
    //   (`artwork_size` = 4x o ícone do painel = 80px na barra `S` dela).
    //   Presos à coluna de texto eles herdam a margem esquerda do título.
    let controles = Row::new()
        .width(Length::Fill)
        .spacing(padding.0)
        .align_y(cosmic::iced::alignment::Vertical::Center)
        .push(accessible_icon_button_without_tooltip(
            "media-skip-backward-symbolic",
            size.0 + 4,
            fl!("previous-track"),
            Message::PreviousTrack,
        ))
        .push(botao_de_tocar(
            transport_icon(model),
            size.0 + 4,
            fl!("toggle-play-pause"),
            model.tema,
        ))
        .push(accessible_icon_button_without_tooltip(
            "media-skip-forward-symbolic",
            size.0 + 4,
            fl!("next-track"),
            Message::NextTrack,
        ));

    let media_info = Column::new()
        .width(Length::Fill)
        .spacing(4)
        .push(titulo)
        .push(
            text(model.now_playing.artist.as_str())
                .size(size.0.saturating_sub(3))
                .width(Length::Fill)
                .wrapping(Wrapping::WordOrGlyph),
        )
        .push(controles);

    // Uma linha só: capa à esquerda, tudo o mais na coluna que ocupa a sobra.
    // O `align_x(Center)` que existia na coluna de fora saiu junto com os
    // andares — ele era o que centrava os controles no cartão, e sem andares
    // soltos não sobrou nada para centralizar.
    Column::new()
        .padding(12)
        .push(
            Row::new()
                .width(Length::Fill)
                .spacing(12)
                .align_y(cosmic::iced::alignment::Vertical::Center)
                .push(artwork)
                .push(media_info),
        )
        .into()
}

fn player_list<'a>(core: &Core, model: &'a AppModel) -> Element<'a, Message> {
    let size = core.applet.suggested_size(true);
    let header = page_header(size.0, fl!("select-player"));
    let list =
        model
            .players
            .iter()
            .enumerate()
            .fold(Column::new().spacing(4), |list, (index, player)| {
                let selected = model.selected_player.as_deref() == Some(player.id.as_str());
                let marker: Element<'_, Message> = if selected {
                    icon::from_name("emblem-ok-symbolic")
                        .size(size.0.saturating_sub(2))
                        .into()
                } else {
                    text("").width(Length::Fixed(f32::from(size.0))).into()
                };
                let list = if index > 0 {
                    list.push(
                        cosmic::widget::divider::horizontal::light()
                            .height(1.0)
                            .width(Length::Fill),
                    )
                } else {
                    list
                };

                list.push(
                    button::custom(
                        Row::new()
                            .spacing(8)
                            .align_y(cosmic::iced::alignment::Vertical::Center)
                            .push(marker)
                            .push(
                                Column::new()
                                    .spacing(1)
                                    .push(text(player.identity.as_str()))
                                    .push(
                                        text(player.track())
                                            .size(size.0.saturating_sub(3))
                                            .wrapping(Wrapping::WordOrGlyph),
                                    ),
                            ),
                    )
                    .width(Length::Fill)
                    .name(format!("{}: {}", fl!("select-player"), player.identity))
                    .selected(selected)
                    .class(cosmic::theme::Button::ListItem([4.0; 4]))
                    .on_press(Message::SelectPlayer(player.id.clone())),
                )
            });

    Column::new()
        .spacing(12)
        .padding(12)
        .push(header)
        .push(list)
        .into()
}

// Lista declarativa de opções: partir em funções só espalharia os rótulos.
#[allow(clippy::too_many_lines)]
fn settings_page<'a>(
    core: &Core,
    model: &'a AppModel,
    config: &'a config::Config,
) -> Element<'a, Message> {
    let size = core.applet.suggested_size(true);
    let atual = &model.settings;

    let alternar = |rotulo: String, valor: bool, ajuste: fn(bool) -> Ajuste| {
        settings::item(
            rotulo,
            toggler(valor).on_toggle(move |v| Message::Ajustar(ajuste(v))),
        )
    };
    let cor = |rotulo: String, valor: &'a str, ajuste: fn(String) -> Ajuste| {
        settings::item(
            rotulo,
            Row::new()
                .spacing(8)
                .align_y(cosmic::iced::alignment::Vertical::Center)
                .push(amostra(config::cor_de_hex(valor)))
                .push(
                    text_input(fl!("theme-default"), valor)
                        .on_input(move |v| Message::Ajustar(ajuste(v)))
                        .width(Length::Fixed(104.0)),
                ),
        )
    };

    let largura = config::largura_valida(config.panel_text_width);
    let fonte_rotulo = config::fonte_valida(config.panel_font_size)
        .map_or_else(|| fl!("automatic"), |px| format!("{px} px"));

    let aparencia = settings::section()
        .title(fl!("appearance"))
        .add(cor(
            fl!("title-color"),
            &config.title_color,
            Ajuste::CorDoTitulo,
        ))
        .add(cor(
            fl!("artist-color"),
            &config.artist_color,
            Ajuste::CorDoArtista,
        ))
        .add(cor(
            fl!("background-color"),
            &config.background_color,
            Ajuste::CorDoFundo,
        ))
        .add(alternar(
            fl!("use-source-colors"),
            atual.source_colors,
            Ajuste::CorDaFonte,
        ))
        .add(alternar(
            fl!("use-album-colors"),
            atual.album_color_enabled,
            Ajuste::CorDoAlbum,
        ))
        .add(alternar(
            fl!("album-color-background"),
            atual.panel_color_style == ColorStyle::Chapado,
            Ajuste::Chapado,
        ));

    let barra = settings::section()
        .title(fl!("panel"))
        .add(settings::item(
            fl!("text-width"),
            Row::new()
                .spacing(8)
                .align_y(cosmic::iced::alignment::Vertical::Center)
                .push(text(format!("{largura} px")))
                .push(
                    slider(config::LARGURA_MIN..=config::LARGURA_MAX, largura, |px| {
                        Message::Ajustar(Ajuste::Largura(px))
                    })
                    .step(10_u32)
                    .width(Length::Fixed(120.0)),
                ),
        ))
        .add(settings::item(
            fl!("font-size"),
            spin_button(
                fonte_rotulo,
                fl!("font-size"),
                config.panel_font_size,
                1,
                0,
                config::FONTE_MAX,
                |px| Message::Ajustar(Ajuste::TamanhoDaFonte(px)),
            ),
        ))
        .add(settings::item(
            fl!("font-family"),
            text_input(fl!("theme-default"), &config.panel_font_family)
                .on_input(|v| Message::Ajustar(Ajuste::FamiliaDaFonte(v)))
                .width(Length::Fixed(140.0)),
        ))
        .add(alternar(
            fl!("panel-controls"),
            atual.panel_controls,
            Ajuste::ControlesNaBarra,
        ))
        .add(alternar(
            fl!("remote-album-art"),
            atual.album_art_remote,
            Ajuste::CapaRemota,
        ));

    Column::new()
        .spacing(12)
        .padding(12)
        .push(page_header(size.0, fl!("settings")))
        .push(
            scrollable(Column::new().spacing(12).push(aparencia).push(barra))
                .height(Length::Shrink),
        )
        .into()
}

/// O quadradinho ao lado do campo de cor: mostra a cor que o texto descreve,
/// ou só a borda quando o campo está vazio (cor do tema).
fn amostra<'a>(cor: Option<Color>) -> Element<'a, Message> {
    container(text(""))
        .width(Length::Fixed(18.0))
        .height(Length::Fixed(18.0))
        .class(cosmic::theme::Container::custom(move |theme| {
            let borda = theme.cosmic().on_bg_color();
            container::Style {
                background: cor.map(Background::Color),
                border: Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: Color::from(borda).scale_alpha(0.4),
                },
                ..container::Style::default()
            }
        }))
        .into()
}

fn page_header(size: u16, title: String) -> Element<'static, Message> {
    Row::new()
        .width(Length::Fill)
        .align_y(cosmic::iced::alignment::Vertical::Center)
        .push(accessible_icon_button_without_tooltip(
            "go-previous-symbolic",
            size,
            fl!("back"),
            Message::ShowMain,
        ))
        .push(text(title).size(size))
        .push(text("").width(Length::Fill))
        .into()
}

fn accessible_icon_button_without_tooltip(
    icon_name: &str,
    size: u16,
    label: String,
    message: Message,
) -> Element<'static, Message> {
    button::icon(icon::from_name(icon_name.to_owned()).size(size))
        .name(label)
        .on_press(message)
        .into()
}

fn botao_de_tocar(
    icon_name: &str,
    size: u16,
    label: String,
    tema: Tema,
) -> Element<'static, Message> {
    let botao = button::icon(icon::from_name(icon_name.to_owned()).size(size))
        .name(label)
        .on_press(Message::TogglePlayPause);
    if tema.acento.is_none() {
        return botao.into();
    }
    botao
        .class(cosmic::theme::Button::Custom {
            active: Box::new(move |focused, theme| {
                tema.no_botao(
                    theme.active(focused, false, &cosmic::theme::Button::Icon),
                    0.0,
                )
            }),
            disabled: Box::new(move |theme| {
                tema.no_botao(theme.disabled(&cosmic::theme::Button::Icon), 0.0)
            }),
            hovered: Box::new(move |focused, theme| {
                tema.no_botao(
                    theme.hovered(focused, false, &cosmic::theme::Button::Icon),
                    tema::REALCE,
                )
            }),
            pressed: Box::new(move |focused, theme| {
                tema.no_botao(
                    theme.pressed(focused, false, &cosmic::theme::Button::Icon),
                    tema::PRESSAO,
                )
            }),
        })
        .into()
}

fn transport_icon(model: &AppModel) -> &'static str {
    match model.now_playing.state {
        PlaybackState::Playing => "media-playback-pause-symbolic",
        PlaybackState::Paused | PlaybackState::Stopped | PlaybackState::Unknown => {
            "media-playback-start-symbolic"
        }
    }
}
