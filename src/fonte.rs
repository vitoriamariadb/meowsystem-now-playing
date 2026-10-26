//! De onde vem o que toca: o serviço por trás do player, e a cor dele.
//!
//! Os sinais, do mais forte para o mais fraco: o host da `xesam:url`, o host da
//! `mpris:artUrl`, o nome do player (bus, identity, desktop entry) e o sufixo
//! que o navegador põe no título da aba (`"… - YouTube"`).

use cosmic::iced::Color;
use url::Url;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fonte {
    YouTube,
    YouTubeMusic,
    Spotify,
    Twitch,
    SoundCloud,
    Deezer,
    Bandcamp,
    AppleMusic,
    Vimeo,
}

/// O que o player publica e serve para reconhecer a fonte.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sinais<'a> {
    pub bus: &'a str,
    pub identidade: &'a str,
    pub desktop: Option<&'a str>,
    pub url: Option<&'a str>,
    pub capa: Option<&'a str>,
    pub titulo: Option<&'a str>,
}

/// Hosts de cada fonte; casa o host exato ou qualquer subdomínio dele. A ordem
/// importa: `music.youtube.com` antes de `youtube.com`.
const HOSTS: &[(Fonte, &[&str])] = &[
    (Fonte::YouTubeMusic, &["music.youtube.com"]),
    (
        Fonte::YouTube,
        &[
            "youtube.com",
            "youtu.be",
            "ytimg.com",
            "ggpht.com",
            "googlevideo.com",
        ],
    ),
    (
        Fonte::Spotify,
        &["spotify.com", "scdn.co", "spotifycdn.com"],
    ),
    (Fonte::Twitch, &["twitch.tv", "jtvnw.net", "ttvnw.net"]),
    (Fonte::SoundCloud, &["soundcloud.com", "sndcdn.com"]),
    (Fonte::Deezer, &["deezer.com", "dzcdn.net"]),
    (Fonte::Bandcamp, &["bandcamp.com", "bcbits.com"]),
    (Fonte::AppleMusic, &["music.apple.com", "mzstatic.com"]),
    (Fonte::Vimeo, &["vimeo.com", "vimeocdn.com"]),
];

/// Palavras no nome do player (minúsculas, sem espaço, hífen ou ponto).
const PLAYERS: &[(Fonte, &[&str])] = &[
    (
        Fonte::YouTubeMusic,
        &["youtubemusic", "ytmusic", "ytmdesktop"],
    ),
    (Fonte::YouTube, &["youtube", "freetube"]),
    (Fonte::Spotify, &["spotify", "ncspot", "spotube", "psst"]),
    (Fonte::Twitch, &["twitch"]),
    (Fonte::SoundCloud, &["soundcloud"]),
    (Fonte::Deezer, &["deezer"]),
    (Fonte::Bandcamp, &["bandcamp"]),
    (Fonte::AppleMusic, &["applemusic", "cider"]),
    (Fonte::Vimeo, &["vimeo"]),
];

/// O que o navegador acrescenta ao título da aba.
const SUFIXOS: &[(Fonte, &[&str])] = &[
    (Fonte::YouTubeMusic, &[" - YouTube Music"]),
    (Fonte::YouTube, &[" - YouTube"]),
    (Fonte::Twitch, &[" - Twitch"]),
    (
        Fonte::SoundCloud,
        &[" | SoundCloud", " | Free Listening on SoundCloud"],
    ),
    (Fonte::Spotify, &[" | Spotify"]),
    (Fonte::Deezer, &[" - Deezer"]),
    (Fonte::Vimeo, &[" on Vimeo", " | Vimeo"]),
];

impl Fonte {
    /// A cor da marca, em `0xRRGGBB`.
    #[must_use]
    pub const fn hex(self) -> u32 {
        match self {
            Self::YouTube | Self::YouTubeMusic => 0xFF_00_00,
            Self::Spotify => 0x1E_D7_60,
            Self::Twitch => 0x91_46_FF,
            Self::SoundCloud => 0xFF_55_00,
            Self::Deezer => 0xA2_38_FF,
            Self::Bandcamp => 0x1D_A0_C3,
            Self::AppleMusic => 0xFA_24_3C,
            Self::Vimeo => 0x1A_B7_EA,
        }
    }

    #[must_use]
    pub fn cor(self) -> Color {
        let n = self.hex();
        #[allow(clippy::cast_possible_truncation)]
        Color::from_rgb8((n >> 16) as u8, (n >> 8) as u8, n as u8)
    }
}

#[must_use]
pub fn detectar(sinais: &Sinais<'_>) -> Option<Fonte> {
    if let Some(fonte) = sinais.url.and_then(por_host) {
        return Some(fonte);
    }
    if let Some(fonte) = sinais.capa.and_then(por_host) {
        return Some(fonte);
    }
    let nomes = [Some(sinais.bus), Some(sinais.identidade), sinais.desktop];
    for nome in nomes.into_iter().flatten() {
        if let Some(fonte) = por_nome_de_player(nome) {
            return Some(fonte);
        }
    }
    sinais
        .titulo
        .and_then(|t| por_sufixo(t).map(|(fonte, _)| fonte))
}

/// O título sem o sufixo de aba (`"Vídeo - YouTube"` vira `"Vídeo"`) e sem o
/// contador de notificações que a página põe na frente (`"(3) Vídeo"`).
#[must_use]
pub fn tirar_sufixo(titulo: &str) -> String {
    let Some((_, sufixo)) = por_sufixo(titulo) else {
        return titulo.to_owned();
    };
    let mut limpo = &titulo[..titulo.len() - sufixo.len()];
    if let Some(resto) = limpo.strip_prefix('(')
        && let Some((numero, depois)) = resto.split_once(") ")
        && !numero.is_empty()
        && numero.chars().all(|c| c.is_ascii_digit() || c == '+')
    {
        limpo = depois;
    }
    let limpo = limpo.trim();
    if limpo.is_empty() {
        titulo.to_owned()
    } else {
        limpo.to_owned()
    }
}

fn por_host(valor: &str) -> Option<Fonte> {
    let url = Url::parse(valor.trim()).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    HOSTS.iter().find_map(|(fonte, hosts)| {
        hosts
            .iter()
            .any(|h| host == *h || host.ends_with(&format!(".{h}")))
            .then_some(*fonte)
    })
}

fn por_nome_de_player(nome: &str) -> Option<Fonte> {
    let compacto: String = nome
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    PLAYERS.iter().find_map(|(fonte, chaves)| {
        chaves
            .iter()
            .any(|c| compacto.contains(c))
            .then_some(*fonte)
    })
}

fn por_sufixo(titulo: &str) -> Option<(Fonte, &'static str)> {
    SUFIXOS.iter().find_map(|(fonte, sufixos)| {
        sufixos
            .iter()
            .find(|s| titulo.ends_with(**s) && titulo.len() > s.len())
            .map(|s| (*fonte, *s))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sinais<'a>(bus: &'a str, identidade: &'a str) -> Sinais<'a> {
        Sinais {
            bus,
            identidade,
            ..Sinais::default()
        }
    }

    #[test]
    fn spotify_pelo_player() {
        let s = sinais("org.mpris.MediaPlayer2.spotify", "Spotify");
        assert_eq!(detectar(&s), Some(Fonte::Spotify));
    }

    #[test]
    fn spotify_pela_capa_mesmo_em_outro_player() {
        let s = Sinais {
            capa: Some("https://i.scdn.co/image/ab67616d0000b273"),
            ..sinais("org.mpris.MediaPlayer2.chromium.instance42", "Chromium")
        };
        assert_eq!(detectar(&s), Some(Fonte::Spotify));
    }

    #[test]
    fn youtube_pela_url_e_pela_miniatura() {
        let s = Sinais {
            url: Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            ..sinais(
                "org.mpris.MediaPlayer2.firefox.instance_1_42",
                "Mozilla Firefox",
            )
        };
        assert_eq!(detectar(&s), Some(Fonte::YouTube));

        let s = Sinais {
            capa: Some("https://i.ytimg.com/vi/dQw4w9WgXcQ/hqdefault.jpg"),
            ..sinais("org.mpris.MediaPlayer2.mpv", "mpv")
        };
        assert_eq!(detectar(&s), Some(Fonte::YouTube));
    }

    #[test]
    fn youtube_music_nao_vira_youtube() {
        let s = Sinais {
            url: Some("https://music.youtube.com/watch?v=abc"),
            ..sinais("org.mpris.MediaPlayer2.chromium.instance1", "Chromium")
        };
        assert_eq!(detectar(&s), Some(Fonte::YouTubeMusic));
        assert_eq!(
            detectar(&sinais(
                "org.mpris.MediaPlayer2.YoutubeMusic",
                "YouTube Music"
            )),
            Some(Fonte::YouTubeMusic)
        );
    }

    #[test]
    fn twitch_e_soundcloud() {
        let s = Sinais {
            url: Some("https://www.twitch.tv/gaules"),
            ..sinais("org.mpris.MediaPlayer2.chromium.instance7", "Google Chrome")
        };
        assert_eq!(detectar(&s), Some(Fonte::Twitch));
        let s = Sinais {
            capa: Some("https://i1.sndcdn.com/artworks-000-t500x500.jpg"),
            ..sinais(
                "org.mpris.MediaPlayer2.firefox.instance_1_9",
                "Mozilla Firefox",
            )
        };
        assert_eq!(detectar(&s), Some(Fonte::SoundCloud));
    }

    #[test]
    fn titulo_de_aba_denuncia_a_fonte() {
        let s = Sinais {
            titulo: Some("Never Gonna Give You Up - YouTube"),
            ..sinais("org.mpris.MediaPlayer2.chromium.instance3", "Google Chrome")
        };
        assert_eq!(detectar(&s), Some(Fonte::YouTube));
        assert_eq!(
            tirar_sufixo("(3) Never Gonna Give You Up - YouTube"),
            "Never Gonna Give You Up"
        );
        assert_eq!(tirar_sufixo("Lofi Girl - YouTube Music"), "Lofi Girl");
        assert_eq!(tirar_sufixo("Sem sufixo nenhum"), "Sem sufixo nenhum");
        assert_eq!(tirar_sufixo(" - YouTube"), " - YouTube");
    }

    #[test]
    fn player_local_nao_tem_fonte() {
        for (bus, id) in [
            ("org.mpris.MediaPlayer2.vlc", "VLC media player"),
            ("org.mpris.MediaPlayer2.mpv", "mpv"),
            (
                "org.mpris.MediaPlayer2.io.github.celluloid_player.Celluloid",
                "Celluloid",
            ),
            (
                "org.mpris.MediaPlayer2.firefox.instance_1_2",
                "Mozilla Firefox",
            ),
        ] {
            assert_eq!(detectar(&sinais(bus, id)), None, "{bus}");
        }
        let s = Sinais {
            capa: Some("file:///tmp/.org.chromium.Chromium.abc"),
            url: Some("file:///home/v/Vídeos/filme.mkv"),
            ..sinais("org.mpris.MediaPlayer2.vlc", "VLC media player")
        };
        assert_eq!(detectar(&s), None);
    }

    #[test]
    fn host_parecido_nao_engana() {
        let s = Sinais {
            url: Some("https://notyoutube.com.example.org/x"),
            ..Sinais::default()
        };
        assert_eq!(detectar(&s), None);
    }

    #[test]
    fn cores_das_marcas() {
        assert_eq!(Fonte::YouTube.hex(), 0xFF_00_00);
        assert_eq!(Fonte::Spotify.hex(), 0x1E_D7_60);
        assert_eq!(Fonte::Twitch.hex(), 0x91_46_FF);
        assert_eq!(Fonte::SoundCloud.hex(), 0xFF_55_00);
    }
}
