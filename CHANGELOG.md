# Changelog

All notable changes to this project are documented in this file.

## [0.3.0] - 2026-10-26

Primeira versão do fork do MeowSystem.

### Adicionado

- App id `io.github.meowsystem.NowPlaying`, binário `meowsystem-now-playing`, tradução pt-BR.
- Nome limpo do arquivo (`xesam:url`) quando o player não publica título; marcas de
  release saem, ano entre parênteses fica, `S01E02` vira `T1 E2`.
- Cor do título por serviço (YouTube, Spotify, Twitch, SoundCloud e outros).
- Configuração no cosmic-config, observada ao vivo: cores de título, artista e fundo,
  largura, tamanho e família da fonte, estilo da cor da capa, capa remota, controles.
- Painel de configurações com todas essas opções.
- Capa remota, cor por traço e controles opcionais, que antes eram o patch do MeowSystem.

### Mudado

- Marcadores como «Not Provided» e «Unknown» não chegam mais à barra.
- Sem artista, a barra mostra só o título, sem o « · » solto.
- Cache de capas em `~/.cache/meowsystem-now-playing/art`.

## [0.2.0] - 2026-07-28

### Added

- Selection between multiple active media players.
- Play/pause control directly from the media icon in the panel.
- A setting to enable or disable album-inspired panel colors.
- Native frosted-glass blur for the applet popup.
- Spanish localization.

### Changed

- Improved media metadata, album artwork, and player-state handling.
- Updated the existing translations and accessibility labels.
- Updated libcosmic and the Rust dependencies.

## [0.1.0] - 2026-07-15

- Initial release.

[0.2.0]: https://github.com/cosmic-utils/cosmic-ext-applet-now-playing/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/cosmic-utils/cosmic-ext-applet-now-playing/releases/tag/v0.1.0
