# Tocando agora (meowsystem-now-playing)

Applet da barra do COSMIC que mostra o que toca agora em qualquer player MPRIS: VLC,
mpv, Celluloid, Spotify, navegador.

É um fork do [cosmic-ext-applet-now-playing](https://github.com/cosmic-utils/cosmic-ext-applet-now-playing),
de Diego MMR e da comunidade cosmic-utils, feito para o
[MeowSystem](https://github.com/vitoriamariadb/MeowSystem). O popup com capa e
controles, a escolha de player e a cor da capa vêm de lá; o crédito do applet é deles.

O que o fork acrescenta:

- **Sempre um nome.** Sem título publicado, vale o nome do arquivo, limpo:
  `Dune.Part.Two.2024.2160p.WEB-DL.DDP5.1-FLUX.mkv` vira `Dune Part Two (2024)`, e
  `Show.S01E02.1080p.mkv` vira `Show T1 E2`. «Not Provided» e «Unknown» nunca
  aparecem.
- **Cor do serviço.** YouTube em vermelho, Spotify em verde, Twitch, SoundCloud e
  outros, reconhecidos pelo player, pela URL, pela capa ou pelo título da aba. Fora
  disso, as cores do tema.
- **Tema do MeowSystem.** Cor do título, do artista e do fundo, largura do texto,
  tamanho e família da fonte, lidos do cosmic-config e trocados ao vivo.
- **Configurações no próprio applet**, na engrenagem do popup.
- App id `io.github.meowsystem.NowPlaying`, interface em português do Brasil.

![o applet na barra](./res/screenshot-1.png)
![o popup](./res/screenshot-2.png)

## Compilar

```bash
cargo build --release --locked
cargo test --release --locked
```

O binário sai em `target/release/meowsystem-now-playing`.

## Instalar

O MeowSystem compila e instala o applet sozinho. À mão, para todo o sistema:

```bash
just build-release
sudo just install
```

Depois, adicione «Tocando agora» nas configurações do painel.

## Integração

As chaves de configuração, os padrões e o que o MeowSystem precisa mudar para usar
este fork estão em [INTEGRACAO.md](./INTEGRACAO.md).

## Histórico

[CHANGELOG.md](./CHANGELOG.md).

## Licença

GPL-3.0-only, a mesma do upstream: [LICENSE](./LICENSE).
