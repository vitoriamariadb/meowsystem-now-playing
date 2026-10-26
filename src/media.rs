use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use cosmic::iced::futures::channel::mpsc;
use cosmic::iced::{Subscription, stream::channel};
use mpris::{PlaybackStatus, PlayerFinder};
use url::Url;

use crate::faixa::{self, Entrada};
use crate::model::{MediaSnapshot, NowPlayingData, PlaybackState, PlayerInfo};

const POLL_INTERVAL: Duration = Duration::from_millis(500);
const RECONNECT_INTERVAL: Duration = Duration::from_secs(1);
const ART_DOWNLOAD_TIMEOUT_SECONDS: &str = "8";
const ART_DOWNLOAD_MAX_BYTES: &str = "8388608";
const ART_CACHE_MAX_FILES: usize = 256;

/// `album_art_remote` da configuração. A thread do monitor lê a cada volta, e
/// a interface escreve quando a chave muda: vale sem reiniciar o applet.
static PERMITIR_DOWNLOAD: AtomicBool = AtomicBool::new(true);

pub fn permitir_download(permitir: bool) {
    PERMITIR_DOWNLOAD.store(permitir, Ordering::Relaxed);
}

#[derive(Clone, Copy, Debug)]
pub enum MediaCommand {
    Previous,
    TogglePlayPause,
    Next,
}

#[must_use]
pub fn initial_snapshot() -> MediaSnapshot {
    let finder = match PlayerFinder::new() {
        Ok(finder) => finder,
        Err(error) => {
            eprintln!("unable to connect to MPRIS: {error}");
            return MediaSnapshot::default();
        }
    };

    // Startup runs on the UI thread: consult the cache, never block on the network.
    match scan(&finder, false) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("unable to read initial MPRIS state: {error}");
            MediaSnapshot::default()
        }
    }
}

pub fn subscription() -> Subscription<MediaSnapshot> {
    Subscription::run(monitor)
}

pub fn run_command(selected_player: Option<&str>, command: MediaCommand) -> Result<(), String> {
    let selected_player =
        selected_player.ok_or_else(|| "no media player is selected".to_owned())?;
    let finder =
        PlayerFinder::new().map_err(|error| format!("unable to connect to MPRIS: {error}"))?;
    let players = finder
        .find_all()
        .map_err(|error| format!("unable to list MPRIS players: {error}"))?;
    let player = players
        .iter()
        .find(|player| player.bus_name() == selected_player)
        .ok_or_else(|| format!("selected MPRIS player disappeared: {selected_player}"))?;

    let result = match command {
        MediaCommand::Previous => player.previous(),
        MediaCommand::TogglePlayPause => player.play_pause(),
        MediaCommand::Next => player.next(),
    };

    result.map_err(|error| {
        format!(
            "MPRIS command {command:?} failed for {}: {error}",
            player.identity()
        )
    })
}

fn monitor() -> impl cosmic::iced::futures::Stream<Item = MediaSnapshot> {
    channel(8, |mut output: mpsc::Sender<MediaSnapshot>| async move {
        thread::spawn(move || monitor_loop(&mut output));
    })
}

fn monitor_loop(output: &mut mpsc::Sender<MediaSnapshot>) {
    let mut finder = None;
    let mut last_sent = None;

    loop {
        if output.is_closed() {
            break;
        }

        if finder.is_none() {
            match PlayerFinder::new() {
                Ok(new_finder) => finder = Some(new_finder),
                Err(error) => {
                    eprintln!("unable to connect to MPRIS: {error}");
                    thread::sleep(RECONNECT_INTERVAL);
                    continue;
                }
            }
        }

        let Some(active_finder) = finder.as_ref() else {
            continue;
        };
        let allow_download = PERMITIR_DOWNLOAD.load(Ordering::Relaxed);
        let snapshot = match scan(active_finder, allow_download) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                eprintln!("unable to poll MPRIS players: {error}");
                finder = None;
                thread::sleep(RECONNECT_INTERVAL);
                continue;
            }
        };

        if last_sent.as_ref() != Some(&snapshot) {
            match output.try_send(snapshot.clone()) {
                Ok(()) => last_sent = Some(snapshot),
                Err(error) if error.is_disconnected() => break,
                Err(_) => {}
            }
        }

        thread::sleep(POLL_INTERVAL);
    }
}

fn scan(finder: &PlayerFinder, allow_download: bool) -> Result<MediaSnapshot, mpris::FindingError> {
    let players = finder
        .find_all()?
        .iter()
        // O playerctld espelha outro player: contá-lo duplica a lista e esconde
        // o nome de quem toca de verdade.
        .filter(|player| !player.bus_name().ends_with(".playerctld"))
        .map(|player| player_info(player, allow_download))
        .collect::<Vec<_>>();
    Ok(MediaSnapshot { players })
}

fn player_info(player: &mpris::Player, allow_download: bool) -> PlayerInfo {
    PlayerInfo {
        id: player.bus_name().to_owned(),
        identity: player.identity().to_owned(),
        now_playing: now_playing(player, allow_download),
    }
}

fn now_playing(player: &mpris::Player, allow_download: bool) -> NowPlayingData {
    let state = playback_state(player);
    let metadata = player.get_metadata().ok();
    let desktop = player.get_desktop_entry().ok().flatten();

    let vazio = || NowPlayingData {
        state,
        ..NowPlayingData::nothing_playing()
    };

    let entrada = Entrada {
        bus: player.bus_name(),
        identidade: player.identity(),
        desktop: desktop.as_deref(),
        titulo: metadata.as_ref().and_then(mpris::Metadata::title),
        artistas: metadata
            .as_ref()
            .and_then(mpris::Metadata::artists)
            .unwrap_or_default(),
        artistas_do_album: metadata
            .as_ref()
            .and_then(mpris::Metadata::album_artists)
            .unwrap_or_default(),
        url: metadata.as_ref().and_then(mpris::Metadata::url),
        capa: metadata.as_ref().and_then(mpris::Metadata::art_url),
        estado: state,
    };
    let Some(faixa) = faixa::resolver(&entrada) else {
        return vazio();
    };

    let text = if faixa.artista.is_empty() {
        faixa.titulo.clone()
    } else {
        format!("{} \u{b7} {}", faixa.titulo, faixa.artista)
    };
    NowPlayingData {
        text,
        title: faixa.titulo,
        artist: faixa.artista,
        state,
        album_art_path: entrada.capa.and_then(|url| art_path(url, allow_download)),
        has_usable_metadata: true,
        fonte: faixa.fonte,
    }
}

fn playback_state(player: &mpris::Player) -> PlaybackState {
    match player.get_playback_status() {
        Ok(PlaybackStatus::Playing) => PlaybackState::Playing,
        Ok(PlaybackStatus::Paused) => PlaybackState::Paused,
        Ok(PlaybackStatus::Stopped) => PlaybackState::Stopped,
        Err(_) => PlaybackState::Unknown,
    }
}

/// Local `file://` art, or a remote `http(s)://` cover downloaded once into the
/// user cache. Spotify (and every web-backed player) only ever exposes an https
/// `mpris:artUrl`, which `file_url_to_path` rejects by design.
fn art_path(value: &str, allow_download: bool) -> Option<PathBuf> {
    file_url_to_path(value).or_else(|| remote_url_to_cached_path(value, allow_download))
}

/// Onde a capa remota `value` mora (ou moraria) no cache. `None` quando a URL
/// não é http(s) — é o que mantém `file://` no caminho do upstream e recusa
/// qualquer outro esquema sem executar nada.
fn art_cache_path(value: &str) -> Option<PathBuf> {
    let url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }

    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    Some(
        dirs::cache_dir()?
            .join("meowsystem-now-playing/art")
            .join(format!("{:016x}", hasher.finish())),
    )
}

fn remote_url_to_cached_path(value: &str, allow_download: bool) -> Option<PathBuf> {
    let path = art_cache_path(value)?;
    let directory = path.parent()?.to_path_buf();
    if path.is_file() {
        return Some(path);
    }
    if !allow_download {
        // Cache miss on the UI thread. The monitor thread will fetch it and the
        // next snapshot carries the cover; stalling startup is never worth it.
        return None;
    }

    std::fs::create_dir_all(&directory).ok()?;
    let partial = path.with_extension("part");
    let downloaded = Command::new("curl")
        .arg("--silent")
        .arg("--fail")
        .arg("--location")
        .arg("--max-time")
        .arg(ART_DOWNLOAD_TIMEOUT_SECONDS)
        .arg("--max-filesize")
        .arg(ART_DOWNLOAD_MAX_BYTES)
        .arg("--proto")
        .arg("=http,https")
        .arg("--output")
        .arg(&partial)
        .arg(value)
        .status()
        .is_ok_and(|status| status.success());

    if !downloaded {
        let _ = std::fs::remove_file(&partial);
        return None;
    }

    std::fs::rename(&partial, &path).ok()?;
    prune_art_cache(&directory);
    Some(path)
}

/// The cache is one ~160 KB JPEG per album ever played. Left alone it grows
/// without a ceiling, so keep the newest `ART_CACHE_MAX_FILES` and drop the rest.
fn prune_art_cache(directory: &Path) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };

    let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            if !metadata.is_file() {
                return None;
            }
            Some((metadata.modified().ok()?, entry.path()))
        })
        .collect();

    if files.len() <= ART_CACHE_MAX_FILES {
        return;
    }

    files.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    for (_, stale) in files.drain(ART_CACHE_MAX_FILES..) {
        let _ = std::fs::remove_file(stale);
    }
}

fn file_url_to_path(value: &str) -> Option<PathBuf> {
    if !has_valid_percent_encoding(value) {
        return None;
    }

    Url::parse(value).ok()?.to_file_path().ok()
}

fn has_valid_percent_encoding(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return false;
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{art_cache_path, art_path, file_url_to_path};
    use std::path::PathBuf;

    #[test]
    fn parses_local_file_urls() {
        assert_eq!(
            file_url_to_path("file:///home/user/Music/Album%20Art.png"),
            Some(PathBuf::from("/home/user/Music/Album Art.png"))
        );
        assert_eq!(
            file_url_to_path("file://localhost/home/user/M%C3%BAsica.png"),
            Some(PathBuf::from("/home/user/Música.png"))
        );
    }

    #[test]
    fn parses_unescaped_unicode_file_urls() {
        assert_eq!(
            file_url_to_path("file:///home/user/Música/Portada.png"),
            Some(PathBuf::from("/home/user/Música/Portada.png"))
        );
    }

    #[test]
    fn rejects_remote_non_file_and_malformed_urls() {
        assert_eq!(file_url_to_path("https://example.com/cover.png"), None);
        assert_eq!(
            file_url_to_path("file://example.com/home/user/cover.png"),
            None
        );
        assert_eq!(file_url_to_path("file:///home/user/bad%2.png"), None);
        assert_eq!(file_url_to_path("file:///home/user/bad%ZZ.png"), None);
    }

    // A QUEIXA INTEIRA MORAVA AQUI. Antes deste patch, `album_art_path` era
    // `metadata.art_url().and_then(file_url_to_path)`: toda URL https caía fora
    // e, com ela, a cor do álbum — que é extraída da capa. O Spotify publica
    // `https://i.scdn.co/image/...` e NUNCA um `file://`.
    #[test]
    fn https_deixou_de_ser_descartado() {
        let url = "https://i.scdn.co/image/ab67616d0000b27321dac5d2bf93bb6ad805a87f";
        assert!(
            art_cache_path(url).is_some(),
            "capa https tem de ter lugar no cache"
        );
    }

    #[test]
    fn cache_e_deterministico_e_separa_urls() {
        let a = "https://i.scdn.co/image/aaaa";
        let b = "https://i.scdn.co/image/bbbb";
        assert_eq!(art_cache_path(a), art_cache_path(a));
        assert_ne!(art_cache_path(a), art_cache_path(b));
    }

    #[test]
    fn esquema_estranho_continua_recusado() {
        assert_eq!(art_cache_path("ftp://example.com/cover.png"), None);
        assert_eq!(art_cache_path("data:image/png;base64,AAAA"), None);
        assert_eq!(art_cache_path("nao-e-url"), None);
    }

    // A thread da interface NUNCA baixa: um `i.scdn.co` lento penduraria a
    // subida do applet, e o painel inteiro junto.
    #[test]
    fn sem_download_e_sem_cache_nao_ha_capa() {
        let url = "https://exemplo.invalido/nunca-cacheado-9f3a2b1c.png";
        assert_eq!(art_path(url, false), None);
    }

    #[test]
    fn file_url_nao_passa_pelo_cache() {
        assert_eq!(
            art_path("file:///home/user/Album%20Art.png", false),
            Some(PathBuf::from("/home/user/Album Art.png"))
        );
    }
}
