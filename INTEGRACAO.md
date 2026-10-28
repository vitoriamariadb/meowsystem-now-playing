# Integração com o MeowSystem

O contrato entre este applet e quem o veste. Mudar nome, tipo ou caminho de uma chave
aqui é mudar o contrato, e o `MeowSystem` precisa mudar junto.

## Onde está

| o quê | valor |
|---|---|
| repositório | <https://github.com/vitoriamariadb/meowsystem-now-playing> |
| branch | `main` |
| app id | `io.github.meowsystem.NowPlaying` |
| binário | `target/release/meowsystem-now-playing` |
| `.desktop` | `res/io.github.meowsystem.NowPlaying.desktop` |
| ícones | `res/icons/hicolor/<tamanho>/apps/io.github.meowsystem.NowPlaying.svg` |
| upstream | <https://github.com/cosmic-utils/cosmic-ext-applet-now-playing>, base `535e84c` |

## Compilar

```bash
git clone https://github.com/vitoriamariadb/meowsystem-now-playing.git
cd meowsystem-now-playing
git checkout <commit>
cargo build --release --locked
cargo test --release --locked
```

O `Cargo.lock` vem no repositório e prende o libcosmic em `1f6dc991`, o mesmo do
`PINO` antigo. Se o painel da máquina andar, o `applet_pino.sh` do MeowSystem continua
valendo: `cargo update -p libcosmic --precise <rev do painel>` antes do build.

## Configuração

Fica no cosmic-config, uma chave por arquivo, em RON:

```
~/.config/cosmic/io.github.meowsystem.NowPlaying/v1/<chave>
```

O applet observa o diretório: mudar um arquivo muda a barra sem reiniciar nada.
Chave ausente ou ilegível vale o padrão. Valor fora da faixa também volta ao padrão,
e o applet nunca trava por isso.

| chave | tipo RON | padrão | faixa e sentido |
|---|---|---|---|
| `title_color` | string | `""` | `"#rrggbb"`; vazio é a cor do tema |
| `artist_color` | string | `""` | `"#rrggbb"`; vazio é a cor do tema |
| `background_color` | string | `""` | `"#rrggbb"` ou `"#rrggbbaa"`; vazio é o fundo do painel |
| `panel_text_width` | u32 | `440` | 80 a 900, em px lógicos: o teto do texto na barra |
| `panel_font_size` | u32 | `0` | 6 a 48 px; `0` é a escala do painel |
| `panel_font_family` | string | `""` | nome da família (ex. `"JetBrainsMono Nerd Font"`); vazio é a do tema |
| `panel_color_style` | string | `"traco"` | `"traco"` pinta contorno e glifos; `"chapado"` pinta o fundo com a cor da capa |
| `album_color_enabled` | bool | `true` | tingir o botão com a cor da capa |
| `source_colors` | bool | `true` | título na cor do serviço (YouTube, Spotify, Twitch...) quando reconhecido |
| `album_art_remote` | bool | `true` | baixar capa `http(s)`; `false` só usa capa local |
| `panel_controls` | bool | `false` | anterior, tocar/pausar e próxima também na barra |

Quem decide a cor do título, em ordem: a cor do serviço (se `source_colors` e o
serviço foi reconhecido), depois `title_color`, depois o tema.

Cores do serviço: YouTube e YouTube Music `#ff0000`, Spotify `#1ed760`, Twitch
`#9146ff`, SoundCloud `#ff5500`, Deezer `#a238ff`, Bandcamp `#1da0c3`, Apple Music
`#fa243c`, Vimeo `#1ab7ea`.

### Escrever de um script

Strings vão entre aspas duplas; números e booleanos vão crus. Sem quebra de linha
no fim também serve.

```bash
D="${XDG_CONFIG_HOME:-$HOME/.config}/cosmic/io.github.meowsystem.NowPlaying/v1"
mkdir -p "$D"
printf '"%s"' '#cba6f7' > "$D/title_color"
printf '"%s"' '#a6e3a1' > "$D/artist_color"
printf '%s'   440       > "$D/panel_text_width"
printf '%s'   0         > "$D/panel_font_size"
printf '"%s"' traco     > "$D/panel_color_style"
printf '%s'   true      > "$D/album_art_remote"
printf '%s'   false     > "$D/panel_controls"
```

Para escrever atômico, grave num temporário no mesmo diretório e faça `mv`.

O painel de configurações do applet (ícone de engrenagem no popup) grava nas mesmas
chaves. Se o doctor do MeowSystem reescrever uma chave, a escolha feita no painel
some; o doctor precisa decidir quais chaves são dele e quais são de quem usa.

### Legado

Enquanto uma chave nova não existe, o applet lê a equivalente do patch antigo em
`~/.config/cosmic-ext-applet-now-playing/` (um valor por arquivo, sem aspas). Assim
que a chave nova é gravada, o legado daquela chave deixa de valer.

| arquivo antigo | chave nova | conversão |
|---|---|---|
| `panel-text-width` | `panel_text_width` | número, pode ter casas decimais |
| `panel-font-size` | `panel_font_size` | `auto` ou inválido vira `0` |
| `panel-color-style` | `panel_color_style` | `traco` / `chapado` |
| `panel-title-color` | `title_color` | `#rrggbb` |
| `panel-artist-color` | `artist_color` | `#rrggbb` |
| `album-art-remote` | `album_art_remote` | `true`/`false`, `1`/`0`, `sim`/`nao` |
| `panel-controls` | `panel_controls` | idem |
| `album-color-enabled` | `album_color_enabled` | idem |

O cache de capas mudou de `~/.cache/cosmic-ext-applet-now-playing/art` para
`~/.cache/meowsystem-now-playing/art`.

## Tema do pack

O MeowSystem escreve as cores do pack em
`$XDG_CONFIG_HOME/meowsystem/extensoes/io.github.meowsystem.NowPlaying/tema.conf` (ou no caminho de
`$MEOWSYSTEM_TEMA`), uma linha `chave = #rrggbb` ou `#rrggbbaa` por cor. O applet lê ao subir e a
cada abertura do popup. Sem o arquivo, ou com linha que não se entende, valem as cores do COSMIC.

| chave | onde aparece |
|---|---|
| `acento` | o botão de tocar/pausar do popup, e o realce dele ao passar o mouse e ao clicar |
| `fundo` | fundo do cartão do popup |
| `borda` | contorno do cartão do popup |
| `texto` | texto e ícones do popup |

No pino: `TEMA="io.github.meowsystem.NowPlaying"` e
`TEMA_CHAVES="acento:@acento fundo:base borda:surface1 texto:text"`.

## O que o MeowSystem precisa mudar

Nada disto foi feito no MeowSystem; é a lista para a sprint.

### `src/applets/now-playing/PINO`

```
URL=https://github.com/vitoriamariadb/meowsystem-now-playing.git
COMMIT=<commit de main a usar>
LIBCOSMIC=1f6dc991eaa5115a09be2505cc8a323e5b2b0bff
```

O `0001-meow-capa-cor-controles.patch` sai do repositório: o que ele fazia está no
fork.

### `scripts/midia_build.sh`

1. Tirar `PATCH=` e o passo `git -C "$ARVORE" apply "$PATCH"` (e a mensagem de
   regenerar o patch).
2. Tirar `[ -f "$PATCH" ]` da checagem de receita completa.
3. Carimbo: o primeiro campo deixa de ser `_sha "$PATCH"`. O `COMMIT` do PINO já
   identifica o código; basta `COMMIT`, sha do binário e `rustc`. Carimbo de três
   campos contra o gravado de quatro conta como divergência e recompila uma vez, o
   que é o desejado.
4. `recem="$ALVO/release/meowsystem-now-playing"` no lugar de
   `.../cosmic-ext-applet-now-playing`. O destino `~/.local/bin/meow-applet-now-playing`
   pode ficar.

### `scripts/midia.sh`

1. Chaves: trocar a escrita em `~/.config/cosmic-ext-applet-now-playing/<arquivo>`
   pela escrita em RON no cosmic-config, com os nomes da tabela acima. Conversões:
   `FONTE=auto` vira `0`; `CONTROLES=sim|nao` vira `true|false`; cor vira
   `"#rrggbb"` entre aspas.
2. `CACHE` passa a `~/.cache/meowsystem-now-playing`.
3. Com as chaves novas gravadas, o diretório legado e a migração de
   `album-color-enabled` do sandbox do flatpak podem sair (o `--reverter` ainda
   precisa limpar os dois).
4. O `pgrep -fx cosmic-ext-applet-now-playing` que detecta o applet do flatpak
   continua certo para o flatpak; o fork se chama `meowsystem-now-playing`.
5. O `.desktop`, em uma de duas formas:
   - **Transição, sem tocar no painel:** a sombra continua com o basename antigo
     `com.github.DiegoMMR.CosmicExtAppletNowPlaying.desktop` e `Exec=` apontando
     para o binário do fork. O `Icon=` antigo segue resolvendo pelo flatpak. A
     configuração passa a morar no id novo de qualquer jeito. Precisa ser
     conferido na máquina: o painel abre o applet pelo `.desktop`; não foi provado
     que ele aceite um app id diferente do basename.
   - **Final:** instalar `io.github.meowsystem.NowPlaying.desktop` e os ícones do
     `res/` em `~/.local/share`, e trocar no painel a entrada
     `com.github.DiegoMMR.CosmicExtAppletNowPlaying` por
     `io.github.meowsystem.NowPlaying` **na mesma posição** (a ordem dos applets é
     de quem usa). A mudança na lista do painel só vale quando o cosmic-panel
     reinicia.
6. `tests/midia.sh` acompanha os caminhos novos.

## Nomes que aparecem

Com título publicado, a barra mostra `título · artista` (sem artista, só o título).
Sem título, o nome limpo do arquivo da `xesam:url`; sem nada disso, o artista; sem
artista e tocando, o nome do player. Parado e sem nada, o applet some. Marcadores de
«não sei» (`Not Provided`, `Unknown`, `Unknown Artist`, `n/a`...) nunca chegam à
tela.

Limite conhecido: Chrome e Firefox costumam não publicar `xesam:url`. No navegador o
serviço é reconhecido pelo sufixo do título da aba (`- YouTube`, `| Spotify`), pelo
host da capa (`ytimg.com`, `scdn.co`) ou pelo nome do player.
