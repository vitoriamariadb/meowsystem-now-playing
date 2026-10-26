//! Nome legível a partir do que o player publica.
//!
//! VLC, mpv e Celluloid publicam o NOME DO ARQUIVO como `xesam:title` quando o
//! arquivo não tem tag, e alguns players não publicam título nenhum, só a
//! `xesam:url`. Aqui o nome de release vira o nome de uma pessoa ler.

use std::path::Path;

use url::Url;

/// Extensões de mídia que saem do nome. Fora desta lista o ponto é do nome.
const EXTENSOES: &[&str] = &[
    "3gp", "aac", "aif", "aiff", "alac", "ape", "avi", "dsf", "flac", "flv", "m2ts", "m4a", "m4v",
    "mka", "mkv", "mov", "mp3", "mp4", "mpeg", "mpg", "oga", "ogg", "ogv", "opus", "ts", "wav",
    "webm", "wma", "wmv", "wv",
];

/// Marcas que só existem em nome de release: onde a primeira aparece, o nome
/// acabou. Valem em qualquer grafia.
const MARCAS_FORTES: &[&str] = &[
    "4k", "8k", "av1", "avc", "bdremux", "bdrip", "blu-ray", "bluray", "brrip", "dvdrip", "dvdscr",
    "h264", "h265", "hdrip", "hdtv", "hevc", "remux", "uhd", "vp9", "web-dl", "webdl", "webrip",
    "x264", "x265", "xvid", "divx", "yts", "rarbg", "eztv",
];

/// Marcas que também são palavras («Opus», «Web», «Extended»): só contam num
/// nome em formato de release (separado por ponto ou sublinhado) ou em caixa
/// alta, para não cortar o título de uma música.
const MARCAS_FRACAS: &[&str] = &[
    "aac",
    "ac3",
    "amzn",
    "atmos",
    "atvp",
    "dd",
    "ddp",
    "dsnp",
    "dts",
    "dual",
    "dub",
    "dubbed",
    "dv",
    "eac3",
    "extended",
    "flac",
    "hd",
    "hdr",
    "hdr10",
    "hmax",
    "hulu",
    "imax",
    "internal",
    "leg",
    "legendado",
    "limited",
    "mp3",
    "multi",
    "nf",
    "opus",
    "proper",
    "remastered",
    "repack",
    "sdr",
    "subbed",
    "truehd",
    "unrated",
    "web",
];

/// O que players publicam quando não sabem o nome. Nunca vai para a tela.
const VAZIOS: &[&str] = &[
    "-",
    "?",
    "desconhecida",
    "desconhecido",
    "n/a",
    "none",
    "not provided",
    "null",
    "undefined",
    "unknown",
    "unknown artist",
    "unknown title",
];

/// `true` para o que não é nome: vazio, só espaço, ou um marcador de «não sei».
#[must_use]
pub fn e_vazio(valor: &str) -> bool {
    let valor = valor.trim();
    valor.is_empty() || VAZIOS.contains(&valor.to_lowercase().as_str())
}

/// O nome do arquivo de uma `xesam:url`, limpo. `file://` e caminho sempre;
/// `http(s)` só quando o último pedaço tem extensão de mídia, porque
/// `watch?v=…` não é nome de nada.
#[must_use]
pub fn nome_de_url(valor: &str) -> Option<String> {
    let valor = valor.trim();
    let base = if let Ok(url) = Url::parse(valor) {
        match url.scheme() {
            "file" => url
                .to_file_path()
                .ok()?
                .file_name()?
                .to_string_lossy()
                .into_owned(),
            "http" | "https" => {
                let segmento = url.path_segments()?.rfind(|s| !s.is_empty())?;
                let segmento = decodificar(segmento);
                extensao_de_midia(&segmento)?;
                segmento
            }
            _ => return None,
        }
    } else if valor.starts_with('/') {
        Path::new(valor).file_name()?.to_string_lossy().into_owned()
    } else {
        return None;
    };
    limpar(&base)
}

/// `true` quando o título publicado é, na verdade, um nome de arquivo: tem
/// extensão de mídia, é o nome do arquivo da `url`, ou tem cara de release.
#[must_use]
pub fn parece_arquivo(titulo: &str, url: Option<&str>) -> bool {
    let titulo = titulo.trim();
    if extensao_de_midia(titulo).is_some() {
        return true;
    }
    if let Some(base) = url.and_then(nome_cru_de_url) {
        let sem_ext = tirar_extensao(&base);
        if base == titulo || sem_ext == titulo {
            return true;
        }
    }
    let formato_release = !titulo.contains(' ') && (titulo.contains('.') || titulo.contains('_'));
    formato_release
        && titulo
            .split(['.', '_'])
            .any(|t| e_marca(t, true) || episodio(t).is_some())
}

/// Limpa um nome de arquivo: tira extensão, número de faixa, tags de release,
/// grupos entre colchetes; ponto e sublinhado viram espaço; `S01E02` vira
/// `T1 E2`; o ano entre parênteses fica. `None` quando não sobra nada.
#[must_use]
pub fn limpar(nome: &str) -> Option<String> {
    let nome = decodificar(nome.trim());
    let mut s = tirar_extensao(&nome).to_owned();
    s = tirar_numero_de_faixa(&s);
    for (de, para) in [
        ("h.264", "h264"),
        ("h.265", "h265"),
        ("x.264", "x264"),
        ("x.265", "x265"),
    ] {
        s = trocar_sem_caixa(&s, de, para);
    }
    s = tirar_colchetes(&s);
    s = s.replace('_', " ");
    let pontos = s.matches('.').count();
    let modo_release = pontos > 0 && pontos >= s.matches(' ').count();
    if modo_release {
        s = s.replace('.', " ");
    }
    s = tratar_parenteses(&s);

    let tokens: Vec<&str> = s.split_whitespace().collect();
    let corte = tokens
        .iter()
        .position(|t| e_marca(t, modo_release))
        .unwrap_or(tokens.len());
    let mut saida: Vec<String> = Vec::with_capacity(corte);
    for (i, token) in tokens[..corte].iter().enumerate() {
        if let Some(ep) = episodio(token) {
            saida.push(ep);
            continue;
        }
        let antes_do_corte = i + 1 == corte && corte < tokens.len();
        let antes_do_episodio = tokens.get(i + 1).is_some_and(|t| episodio(t).is_some());
        let no_fim_de_release = modo_release && i + 1 == tokens.len();
        if i > 0 && e_ano(token) && (antes_do_corte || antes_do_episodio || no_fim_de_release) {
            saida.push(format!("({token})"));
        } else {
            saida.push((*token).to_owned());
        }
    }
    if saida.is_empty() {
        saida = tokens
            .iter()
            .filter(|t| !e_marca(t, true))
            .map(|t| (*t).to_owned())
            .collect();
    }

    let junto = saida.join(" ");
    let limpo = junto
        .trim_matches(|c: char| c.is_whitespace() || matches!(c, '-' | '–' | '—' | '·' | '|' | ','))
        .to_owned();
    (!limpo.is_empty()).then_some(limpo)
}

fn nome_cru_de_url(valor: &str) -> Option<String> {
    let url = Url::parse(valor.trim()).ok()?;
    match url.scheme() {
        "file" => Some(
            url.to_file_path()
                .ok()?
                .file_name()?
                .to_string_lossy()
                .into_owned(),
        ),
        "http" | "https" => url
            .path_segments()?
            .rfind(|s| !s.is_empty())
            .map(decodificar),
        _ => None,
    }
}

fn extensao_de_midia(nome: &str) -> Option<&str> {
    let (_, ext) = nome.rsplit_once('.')?;
    let baixa = ext.to_ascii_lowercase();
    EXTENSOES.contains(&baixa.as_str()).then_some(ext)
}

fn tirar_extensao(nome: &str) -> &str {
    match extensao_de_midia(nome) {
        Some(ext) => &nome[..nome.len() - ext.len() - 1],
        None => nome,
    }
}

fn decodificar(valor: &str) -> String {
    if valor.contains('%') {
        url::form_urlencoded::parse(format!("x={}", valor.replace('+', "%2B")).as_bytes())
            .next()
            .map_or_else(|| valor.to_owned(), |(_, v)| v.into_owned())
    } else {
        valor.to_owned()
    }
}

/// `01 - Faixa` e `01. Faixa`: o número só sai com separador explícito, para
/// que `1917` e `24` continuem sendo nomes.
fn tirar_numero_de_faixa(s: &str) -> String {
    let digitos = s.chars().take_while(char::is_ascii_digit).count();
    if (1..=2).contains(&digitos) {
        let resto = &s[digitos..];
        for sep in [" - ", ". ", " – "] {
            if let Some(depois) = resto.strip_prefix(sep)
                && !depois.trim().is_empty()
            {
                return depois.to_owned();
            }
        }
    }
    s.to_owned()
}

fn trocar_sem_caixa(s: &str, de: &str, para: &str) -> String {
    let baixa = s.to_ascii_lowercase();
    let mut saida = String::with_capacity(s.len());
    let mut i = 0;
    while let Some(pos) = baixa[i..].find(de) {
        saida.push_str(&s[i..i + pos]);
        saida.push_str(para);
        i += pos + de.len();
    }
    saida.push_str(&s[i..]);
    saida
}

/// `[Grupo]`, `[1080p]`, `[ABCD1234]` e `{…}` nunca são nome.
fn tirar_colchetes(s: &str) -> String {
    let mut saida = String::with_capacity(s.len());
    let mut fundo = 0usize;
    for c in s.chars() {
        match c {
            '[' | '{' => {
                fundo += 1;
                saida.push(' ');
            }
            ']' | '}' if fundo > 0 => fundo -= 1,
            _ if fundo == 0 => saida.push(c),
            _ => {}
        }
    }
    saida
}

/// Parênteses: ano fica, só-tags sai, o resto («Live», «Remix») fica.
fn tratar_parenteses(s: &str) -> String {
    let mut saida = String::with_capacity(s.len());
    let mut resto = s;
    while let Some(abre) = resto.find('(') {
        let Some(fecha) = resto[abre..].find(')').map(|f| abre + f) else {
            break;
        };
        saida.push_str(&resto[..abre]);
        let dentro = resto[abre + 1..fecha].trim();
        let tokens: Vec<&str> = dentro
            .split([' ', '.', ','])
            .filter(|t| !t.is_empty())
            .collect();
        let so_marcas = !tokens.is_empty() && tokens.iter().all(|t| e_marca(t, true));
        if !so_marcas && !dentro.is_empty() {
            saida.push('(');
            saida.push_str(dentro);
            saida.push(')');
        }
        resto = &resto[fecha + 1..];
    }
    saida.push_str(resto);
    saida
}

fn e_ano(token: &str) -> bool {
    token.len() == 4
        && token.chars().all(|c| c.is_ascii_digit())
        && (token.starts_with("19") || token.starts_with("20"))
}

fn e_marca(token: &str, modo_release: bool) -> bool {
    let token = token.trim_matches(|c: char| matches!(c, '(' | ')' | ',' | '-'));
    if token.is_empty() {
        return false;
    }
    let baixa = token.to_ascii_lowercase();
    if e_resolucao(&baixa) || e_profundidade(&baixa) {
        return true;
    }
    let cabeca = baixa.split('-').next().unwrap_or(&baixa);
    if MARCAS_FORTES.contains(&baixa.as_str()) || MARCAS_FORTES.contains(&cabeca) {
        return true;
    }
    let caixa_alta = token.chars().any(|c| c.is_ascii_alphabetic())
        && token
            .chars()
            .all(|c| !c.is_alphabetic() || c.is_uppercase());
    if !(modo_release || caixa_alta) {
        return false;
    }
    let sem_digitos = cabeca.trim_end_matches(|c: char| c.is_ascii_digit());
    MARCAS_FRACAS.contains(&cabeca) || MARCAS_FRACAS.contains(&sem_digitos)
}

fn e_resolucao(baixa: &str) -> bool {
    let Some(numero) = baixa.strip_suffix('p').or_else(|| baixa.strip_suffix('i')) else {
        return false;
    };
    matches!(
        numero,
        "360" | "480" | "540" | "576" | "720" | "1080" | "1440" | "2160" | "4320"
    )
}

fn e_profundidade(baixa: &str) -> bool {
    matches!(baixa, "8bit" | "10bit" | "12bit")
}

/// `S01E02` → `T1 E2`, `S01E01E02` → `T1 E1-2`, `1x05` → `T1 E5`, `S03` → `T3`.
fn episodio(token: &str) -> Option<String> {
    let baixa = token.to_ascii_lowercase();
    let numero = |s: &str| -> Option<u32> {
        (!s.is_empty() && s.len() <= 3 && s.chars().all(|c| c.is_ascii_digit()))
            .then(|| s.parse().ok())
            .flatten()
    };

    if let Some(resto) = baixa.strip_prefix('s') {
        let mut partes = resto.split('e');
        let temporada = numero(partes.next()?)?;
        let episodios: Vec<u32> = partes.map(numero).collect::<Option<_>>()?;
        if resto.len() > 2 && episodios.is_empty() {
            return None;
        }
        return Some(match episodios.as_slice() {
            [] => format!("T{temporada}"),
            [um] => format!("T{temporada} E{um}"),
            [primeiro, .., ultimo] => format!("T{temporada} E{primeiro}-{ultimo}"),
        });
    }
    let (temporada, ep) = baixa.split_once('x')?;
    if temporada.len() > 2 || ep.len() < 2 {
        return None;
    }
    Some(format!("T{} E{}", numero(temporada)?, numero(ep)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nomes de arquivo como aparecem de verdade em VLC, mpv e Celluloid.
    const CASOS: &[(&str, &str)] = &[
        (
            "The.Office.US.S02E03.720p.WEB-DL.x264-GROUP.mkv",
            "The Office US T2 E3",
        ),
        (
            "Breaking.Bad.S05E14.Ozymandias.1080p.BluRay.x265-RARBG.mp4",
            "Breaking Bad T5 E14 Ozymandias",
        ),
        (
            "Dune.Part.Two.2024.2160p.WEB-DL.DDP5.1.Atmos.DV.HDR.H.265-FLUX.mkv",
            "Dune Part Two (2024)",
        ),
        (
            "Dune.Part.Two.2024.2160p.WEB-DL.DDP5.1-FLUX.mkv",
            "Dune Part Two (2024)",
        ),
        ("Show.S01E02.1080p.mkv", "Show T1 E2"),
        (
            "[SubsPlease] Sousou no Frieren - 12 (1080p) [ABCD1234].mkv",
            "Sousou no Frieren - 12",
        ),
        (
            "Interstellar (2014) [1080p] [YTS.MX].mp4",
            "Interstellar (2014)",
        ),
        (
            "01 - Daft Punk - One More Time.flac",
            "Daft Punk - One More Time",
        ),
        ("07. Construção.mp3", "Construção"),
        ("minha_musica_favorita.mp3", "minha musica favorita"),
        (
            "Blade.Runner.2049.2017.1080p.BluRay.x264.mkv",
            "Blade Runner 2049 (2017)",
        ),
        ("1917.2019.1080p.WEBRip.x264-RARBG.mp4", "1917 (2019)"),
        (
            "game.of.thrones.s08e06.1080p.web.h264-memento.mkv",
            "game of thrones T8 E6",
        ),
        ("Show Name 1x05 HDTV.avi", "Show Name T1 E5"),
        ("Eric Prydz - Opus.mp3", "Eric Prydz - Opus"),
        ("Video do YouTube.webm", "Video do YouTube"),
        ("Arquivo Sem Extensao", "Arquivo Sem Extensao"),
        (
            "The.Mandalorian.S01.1080p.DSNP.WEB-DL.mkv",
            "The Mandalorian T1",
        ),
        ("Song (Live).ogg", "Song (Live)"),
        ("Movie.Name.2020.mkv", "Movie Name (2020)"),
        (
            "[Erai-raws] Spy x Family - 05 [720p][Multiple Subtitle].mkv",
            "Spy x Family - 05",
        ),
        ("Show.S01E01E02.720p.HDTV.mkv", "Show T1 E1-2"),
        (
            "Concerto_Ao_Vivo_(2019)_[FLAC].flac",
            "Concerto Ao Vivo (2019)",
        ),
        (
            "Fallout.2024.S01E03.The.Head.1080p.AMZN.WEB-DL.DDP5.1.H.264-NTb.mkv",
            "Fallout (2024) T1 E3 The Head",
        ),
        (
            "Cidade de Deus (2002) (1080p BluRay x265 10bit).mkv",
            "Cidade de Deus (2002)",
        ),
        ("24.S01E01.720p.mkv", "24 T1 E1"),
        (
            "Hino Nacional (Remastered 2011).flac",
            "Hino Nacional (Remastered 2011)",
        ),
    ];

    #[test]
    fn tabela_de_nomes_reais() {
        let mut falhas = Vec::new();
        for (entrada, esperado) in CASOS {
            let obtido = limpar(entrada);
            if obtido.as_deref() != Some(*esperado) {
                falhas.push(format!(
                    "{entrada:?}: esperado {esperado:?}, veio {obtido:?}"
                ));
            }
        }
        assert!(falhas.is_empty(), "\n{}", falhas.join("\n"));
    }

    #[test]
    fn url_de_arquivo_vira_nome() {
        assert_eq!(
            nome_de_url("file:///home/vitoria/V%C3%ADdeos/Meu.Filme.2021.1080p.WEB-DL.mkv")
                .as_deref(),
            Some("Meu Filme (2021)")
        );
        assert_eq!(
            nome_de_url("/home/vitoria/Música/02 - Faixa Dois.opus").as_deref(),
            Some("Faixa Dois")
        );
        assert_eq!(
            nome_de_url("https://exemplo.com/midia/Live.Set.2023.mp3").as_deref(),
            Some("Live Set (2023)")
        );
    }

    #[test]
    fn url_de_pagina_nao_e_nome() {
        assert_eq!(
            nome_de_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            None
        );
        assert_eq!(nome_de_url("https://open.spotify.com/track/abc"), None);
        assert_eq!(nome_de_url("nao e url"), None);
    }

    #[test]
    fn marcadores_de_nao_sei_sao_vazios() {
        for v in [
            "Not Provided",
            "Unknown",
            "unknown",
            "  ",
            "",
            "N/A",
            "Unknown Artist",
            "null",
        ] {
            assert!(e_vazio(v), "{v:?} devia ser vazio");
        }
        for v in [
            "Unknown Pleasures",
            "Nothing",
            "Desconhecidos do Rock",
            "Untitled #1",
        ] {
            assert!(!e_vazio(v), "{v:?} é nome de verdade");
        }
    }

    #[test]
    fn reconhece_titulo_que_e_arquivo() {
        assert!(parece_arquivo("filme.mkv", None));
        assert!(parece_arquivo("The.Office.S01E01.720p", None));
        assert!(parece_arquivo(
            "gravacao 03",
            Some("file:///home/v/gravacao%2003.ogg")
        ));
        assert!(!parece_arquivo("One More Time", None));
        assert!(!parece_arquivo("Mr. Robot", None));
        assert!(!parece_arquivo(
            "Vol. 2",
            Some("https://www.youtube.com/watch?v=x")
        ));
    }
}
