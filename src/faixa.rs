//! Título, artista e fonte a partir do que o player publica.
//!
//! A regra que sustenta o resto: nenhum marcador de «não sei» chega à tela.
//! Sem título, vale o nome do arquivo; sem arquivo, o nome do player, mas só
//! enquanto ele toca de verdade.

use crate::fonte::{self, Fonte, Sinais};
use crate::limpeza;
use crate::model::PlaybackState;

/// O que o player publica, já tirado do D-Bus.
#[derive(Clone, Debug, Default)]
pub struct Entrada<'a> {
    pub bus: &'a str,
    pub identidade: &'a str,
    pub desktop: Option<&'a str>,
    pub titulo: Option<&'a str>,
    pub artistas: Vec<&'a str>,
    pub artistas_do_album: Vec<&'a str>,
    pub url: Option<&'a str>,
    pub capa: Option<&'a str>,
    pub estado: PlaybackState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Faixa {
    pub titulo: String,
    /// Vazio quando ninguém disse quem é: a barra mostra só o título.
    pub artista: String,
    pub fonte: Option<Fonte>,
}

/// `None` quando não há nada a mostrar: o applet some da barra.
#[must_use]
pub fn resolver(entrada: &Entrada<'_>) -> Option<Faixa> {
    let titulo_publicado = entrada
        .titulo
        .filter(|t| !limpeza::e_vazio(t))
        .map(str::trim);
    // mpv e VLC tocando um link publicam o próprio link como título.
    let url_no_titulo = titulo_publicado.filter(|t| t.contains("://"));
    let fonte = fonte::detectar(&Sinais {
        bus: entrada.bus,
        identidade: entrada.identidade,
        desktop: entrada.desktop,
        url: entrada.url.or(url_no_titulo),
        capa: entrada.capa,
        titulo: titulo_publicado,
    });

    let mut titulo = titulo_publicado.and_then(|t| titulo_legivel(t, entrada.url));
    if titulo.is_none() {
        titulo = entrada.url.and_then(limpeza::nome_de_url);
    }

    let mut artista = juntar(&entrada.artistas);
    if artista.is_empty() {
        artista = juntar(&entrada.artistas_do_album);
    }

    let titulo = match titulo {
        Some(t) => t,
        None if !artista.is_empty() => std::mem::take(&mut artista),
        None if entrada.estado == PlaybackState::Playing => nome_do_player(entrada)?,
        None => return None,
    };

    Some(Faixa {
        titulo,
        artista,
        fonte,
    })
}

fn titulo_legivel(titulo: &str, url: Option<&str>) -> Option<String> {
    if titulo.contains("://") {
        return limpeza::nome_de_url(titulo);
    }
    let titulo = fonte::tirar_sufixo(titulo);
    if limpeza::parece_arquivo(&titulo, url) {
        return limpeza::limpar(&titulo).or(Some(titulo));
    }
    Some(titulo)
}

fn juntar(nomes: &[&str]) -> String {
    let mut vistos: Vec<&str> = Vec::new();
    for nome in nomes.iter().map(|n| n.trim()) {
        if !limpeza::e_vazio(nome) && !vistos.contains(&nome) {
            vistos.push(nome);
        }
    }
    vistos.join(", ")
}

fn nome_do_player(entrada: &Entrada<'_>) -> Option<String> {
    let identidade = entrada.identidade.trim();
    if !limpeza::e_vazio(identidade) {
        return Some(identidade.to_owned());
    }
    let curto = entrada
        .bus
        .strip_prefix("org.mpris.MediaPlayer2.")
        .unwrap_or(entrada.bus);
    let curto = curto.split(".instance").next().unwrap_or(curto).trim();
    (!curto.is_empty()).then(|| curto.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entrada<'a>(bus: &'a str, identidade: &'a str) -> Entrada<'a> {
        Entrada {
            bus,
            identidade,
            estado: PlaybackState::Playing,
            ..Entrada::default()
        }
    }

    fn texto(f: &Faixa) -> String {
        if f.artista.is_empty() {
            f.titulo.clone()
        } else {
            format!("{} · {}", f.titulo, f.artista)
        }
    }

    #[test]
    fn nunca_mostra_not_provided_nem_unknown() {
        let e = Entrada {
            titulo: Some("Not Provided"),
            artistas: vec!["Unknown"],
            url: Some("file:///home/v/V%C3%ADdeos/Meu.Filme.2021.1080p.WEB-DL.mkv"),
            ..entrada("org.mpris.MediaPlayer2.vlc", "VLC media player")
        };
        let f = resolver(&e).unwrap();
        assert_eq!(texto(&f), "Meu Filme (2021)");

        let e = Entrada {
            titulo: Some("Not Provided"),
            artistas: vec!["Unknown"],
            ..entrada("org.mpris.MediaPlayer2.chromium.instance9", "Google Chrome")
        };
        let f = resolver(&e).unwrap();
        assert_eq!(texto(&f), "Google Chrome");
        assert!(!texto(&f).contains("Unknown") && !texto(&f).contains("Not Provided"));
    }

    #[test]
    fn vlc_sem_tag_publica_o_nome_do_arquivo() {
        let e = Entrada {
            titulo: Some("The.Office.US.S02E03.720p.WEB-DL.x264-GROUP.mkv"),
            url: Some("file:///home/v/The.Office.US.S02E03.720p.WEB-DL.x264-GROUP.mkv"),
            ..entrada("org.mpris.MediaPlayer2.vlc", "VLC media player")
        };
        assert_eq!(texto(&resolver(&e).unwrap()), "The Office US T2 E3");
    }

    #[test]
    fn mpv_e_celluloid_sem_titulo_usam_a_url() {
        for (bus, id) in [
            ("org.mpris.MediaPlayer2.mpv", "mpv"),
            (
                "org.mpris.MediaPlayer2.io.github.celluloid_player.Celluloid",
                "Celluloid",
            ),
        ] {
            let e = Entrada {
                url: Some("file:///home/v/M%C3%BAsica/01%20-%20Faixa%20Um.flac"),
                ..entrada(bus, id)
            };
            assert_eq!(texto(&resolver(&e).unwrap()), "Faixa Um", "{bus}");
        }
    }

    #[test]
    fn titulo_com_tag_fica_como_veio() {
        let e = Entrada {
            titulo: Some("One More Time"),
            artistas: vec!["Daft Punk"],
            url: Some("file:///home/v/M%C3%BAsica/01.flac"),
            ..entrada("org.mpris.MediaPlayer2.vlc", "VLC media player")
        };
        assert_eq!(texto(&resolver(&e).unwrap()), "One More Time · Daft Punk");
    }

    #[test]
    fn chrome_parado_so_com_duracao_some() {
        let e = Entrada {
            estado: PlaybackState::Stopped,
            ..entrada("org.mpris.MediaPlayer2.chromium.instance1", "Google Chrome")
        };
        assert_eq!(resolver(&e), None);
        let e = Entrada {
            estado: PlaybackState::Paused,
            ..e
        };
        assert_eq!(resolver(&e), None);
    }

    #[test]
    fn youtube_no_navegador() {
        let e = Entrada {
            titulo: Some("(2) Never Gonna Give You Up - YouTube"),
            artistas: vec!["Rick Astley"],
            ..entrada("org.mpris.MediaPlayer2.chromium.instance5", "Google Chrome")
        };
        let f = resolver(&e).unwrap();
        assert_eq!(f.fonte, Some(Fonte::YouTube));
        assert_eq!(texto(&f), "Never Gonna Give You Up · Rick Astley");
    }

    #[test]
    fn spotify_tem_fonte_e_varios_artistas() {
        let e = Entrada {
            titulo: Some("Under Pressure"),
            artistas: vec!["Queen", "David Bowie", "Queen"],
            capa: Some("https://i.scdn.co/image/ab67616d0000b273"),
            ..entrada("org.mpris.MediaPlayer2.spotify", "Spotify")
        };
        let f = resolver(&e).unwrap();
        assert_eq!(f.fonte, Some(Fonte::Spotify));
        assert_eq!(texto(&f), "Under Pressure · Queen, David Bowie");
    }

    #[test]
    fn so_artista_vira_titulo() {
        let e = Entrada {
            artistas: vec!["Rádio Nacional"],
            ..entrada("org.mpris.MediaPlayer2.vlc", "VLC media player")
        };
        assert_eq!(texto(&resolver(&e).unwrap()), "Rádio Nacional");
    }

    #[test]
    fn artista_do_album_cobre_a_falta_do_artista() {
        let e = Entrada {
            titulo: Some("Faixa"),
            artistas: vec!["Unknown Artist"],
            artistas_do_album: vec!["Banda"],
            ..entrada("org.mpris.MediaPlayer2.vlc", "VLC media player")
        };
        assert_eq!(texto(&resolver(&e).unwrap()), "Faixa · Banda");
    }

    #[test]
    fn titulo_que_e_url_vira_nome() {
        let e = Entrada {
            titulo: Some("https://radio.exemplo.com/stream/Show.Ao.Vivo.2024.mp3"),
            ..entrada("org.mpris.MediaPlayer2.mpv", "mpv")
        };
        assert_eq!(texto(&resolver(&e).unwrap()), "Show Ao Vivo (2024)");

        let e = Entrada {
            titulo: Some("https://www.youtube.com/watch?v=abc"),
            ..entrada("org.mpris.MediaPlayer2.mpv", "mpv")
        };
        let f = resolver(&e).unwrap();
        assert_eq!(texto(&f), "mpv");
        assert_eq!(f.fonte, Some(Fonte::YouTube));
    }

    #[test]
    fn player_sem_identidade_usa_o_bus() {
        let e = entrada("org.mpris.MediaPlayer2.chromium.instance77", "");
        assert_eq!(texto(&resolver(&e).unwrap()), "chromium");
    }
}
