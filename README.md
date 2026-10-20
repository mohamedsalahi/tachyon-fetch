<div align="center">

<img src="assets/banner.svg" alt="Tachyon — precision systems readout" width="100%">

<br>

### A cockpit-grade system fetch. One process. One pass.

<p>
  <img alt="Rust" src="https://img.shields.io/badge/RUST-2021-ffb86b?style=for-the-badge&logo=rust&logoColor=0b0d10&labelColor=1b1f27">
  <img alt="License MIT" src="https://img.shields.io/badge/LICENSE-MIT-2ea043?style=for-the-badge&labelColor=1b1f27">
  <img alt="Linux" src="https://img.shields.io/badge/LINUX-x86__64-58a6ff?style=for-the-badge&logo=linux&logoColor=0b0d10&labelColor=1b1f27">
</p>
<p>
  <img alt="No runtime tooling" src="https://img.shields.io/badge/RUNTIME_TOOLING-0-6ee7ff?style=for-the-badge&labelColor=1b1f27">
  <img alt="Release binary" src="https://img.shields.io/badge/BINARY-1.9_MB-8957e5?style=for-the-badge&labelColor=1b1f27">
  <img alt="Kitty graphics" src="https://img.shields.io/badge/KITTY-GRAPHICS-f778ba?style=for-the-badge&labelColor=1b1f27">
  <img alt="Built-in TUI" src="https://img.shields.io/badge/TUI-BUILT_IN-39c5cf?style=for-the-badge&labelColor=1b1f27">
  <img alt="Powered by ratatui" src="https://img.shields.io/badge/POWERED_BY-RATATUI-ffb86b?style=for-the-badge&labelColor=1b1f27">
</p>

<br>

</div>

> Read the instruments, not the manual. Tachyon is a system readout that treats your terminal like a flight deck: dense, deliberate, and gone in a single frame.

<br>

## ◈ What it is

Tachyon renders the state of the machine directly from the kernel's own surfaces. It walks `/proc`, `/sys`, DMI tables and hwmon, opens NVML only when it is present, and writes one composed frame. Nothing is shelled out to `uname`, `df`, `ip`, `systemctl` or `nvidia-smi`; there are no helper binaries, no daemons, no caches and no network calls.

It is designed for people who want the truth about their systems at a glance — and then want their prompt back.

<br>

## ◈ Feature set

| | |
|---|---|
| **25 modules** | system, compute, storage, network, processes and power, individually toggleable |
| **One process** | every value is collected in-process; no forks, no pipelines |
| **HUD layout** | two-column flight-deck composition with a `stack` fallback for narrow terms |
| **Reveal motion** | a short staged entrance, or silence with `--static` |
| **Configurator** | `--tui` pages for modules, deck, glyphs, palette, thresholds and artwork, plus a `P` full-screen preview of the real frame |
| **Palette engine** | six built-in themes (Solitude, Vantablack, Hackerman, Retro 82, Rose Pine, Flexoki Light) plus a TSV table and the active theme state |
| **ASCII logos** | built-in distro art (NixOS, Arch, Fedora, Ubuntu, Mint, Gentoo, Debian, openSUSE, Alpine, Void, Manjaro, Pop, Kali), auto-detected and replaceable from a drop-in directory |
| **Logo colours** | keep the distro palette, borrow the active theme, or derive a palette from the logo and apply it to the whole frame (default) |
| **Logo side** | left or right, so the frame uses the terminal's full width |
| **Text logo** | any system font rendered as braille art with a two-tone tint — no image protocol needed |
| **Image logo** | kitty graphics protocol with placement that never reflows text |
| **Glyph system** | one mapped glyph for every fetched object; pin it, randomize it, or draw it from a custom pool; per-section glyphs; browse any installed font and take a glyph or build a pool from it |
| **Quote bench** | 51 attributed lines (Bacon to Clarke); random, off, or pinned, with a filterable picker |
| **Rotation** | artwork and palette rotation with `off · next · shuffle`, a chosen theme set, and `run · hour · day` cadence; only themes with a genuine variant for the active mode can be picked or pooled |
| **Album flow** | `$XDG_DATA_HOME/tachyon/albums/<name>/*.png`, browsed album-first in the configurator |
| **Image ingest** | JPEG/PNG conversion inside the binary — no ImageMagick, no shell |
| **Threshold colour** | capture warns once a metric actually means something |
| **Graceful absence** | no GPU, no battery, no hwmon, no NVML, no palette: the frame still renders |
| **No residue** | no config written unless you ask the configurator to save |

<br>

## ◈ Install

```sh
git clone https://github.com/mohamedsalahi/tachyon-fetch.git
cd tachyon-fetch
cargo build --release
install -Dm755 target/release/tachyon ~/.local/bin/tachyon
```

Or straight from the source tree:

```sh
cargo install --git https://github.com/mohamedsalahi/tachyon-fetch --locked
```

The crate is named `tachyon-fetch`; the installed command is `tachyon`. A recent stable Rust toolchain is the only build requirement.

<br>

## ◈ First frame

```sh
tachyon
```

```text
TACHYON // FLIGHT DECK · SIM                                                        SOLITUDE · DARK
pilot@tuf-a15
───────────────────────────────────────────────────────────────────────────────────────────────────
▸ SYSTEM ──────────────────────────────────────────────────────────────────────────────────────────
OS          NixOS 26.11 (x86_64)
Host        ASUS TUF Gaming A15
Kernel      6.16.0
Uptime      2h 41m
Session     wayland
Shell       fish 4.0.1
Terminal    kitty
Compositor  Umbriel (Wayland)
Load        0.42 0.51 0.47
▸ CORE ────────────────────────────────────────────────────────────────────────────────────────────
CPU        AMD Ryzen 7 (16)
GPU        NVIDIA GeForce RTX 4060 Laptop GPU
GPU util   37%
VRAM       3.2 / 8.0 GiB (40%)
Memory     4.6 / 15.0 GiB (31%)
Swap       0.0 / 8.0 GiB (0%)
Disk (/)   214 GiB / 1.0 TiB (21%)
Media      612 GiB / 1.4 TiB (43%)
Code       18 GiB / 256 GiB (7%)
IP         10.0.0.42
Link       wlan0 · 866 Mb/s
Processes  412
▸ POWER ───────────────────────────────────────────────────────────────────────────────────────────
Battery         87% [Charging]
Battery health  96%
CPU temp        58 °C
GPU temp        52 °C

“We must know. We will know.”
  -David Hilbert
```

<br>

## ◈ Command line

| Flag | Effect |
|---|---|
| `--demo` | render synthetic values instead of host reads |
| `--tui` | open the module and appearance configurator |
| `--plain` | no colour, no image |
| `--static` | disable the reveal animation |
| `--no-logo` | skip the logo |
| `--layout <hud\|stack>` | choose the composition |
| `--glyph <char\|random>` | header glyph |
| `--album <name>` | pull artwork from an album |
| `--theme <name>` | override the active palette |
| `--variant <dark\|light>` | force the palette variant |
| `--seed <n>` | deterministic layout, glyph and artwork choice |
| `--logo <path>` | explicit PNG for the kitty graphics protocol |
| `--palettes <path>` | alternate palette table |
| `--quotes <path>` | alternate note pool |
| `--config <path>` | alternate module/appearance config |
| `--state <path>` | alternate theme state file |
| `--list-themes` | print the built-in and installed palettes |
| `--fonts` | list system font families (fontconfig) |
| `--list-logos` | print built-in and drop-in ascii logo ids |
| `--logos <path>` | ascii logo directory (default `$XDG_DATA_HOME/tachyon/logos`) |
| `--convert <src> <dst>` | convert an image to PNG (used by album ingest) |
| `-h`, `-V` | help and version |

<br>

## ◈ The configurator

```sh
tachyon --tui
```

Seven pages, one engine: **Modules · Deck · Glyphs · Palette · Thresholds · Artwork · Keys**. `P` hides the interface and shows the frame full-screen exactly as the fetch prints it; `P` or `esc` returns.

```text
h l / ← →      previous · next page space   toggle · pin/random · mark
j k / ↑ ↓      move · preview scroll enter   glyph library · open picker · apply
+ -            adjust · cycle glyph J K     reorder the enabled modules
shift+enter    pick a module up, move with j/k, drop (m works everywhere)
P              full-screen preview   esc     back to editing
e / g / G      type a glyph value · glyph library · pool from a font
R              reset (setting · page · config)
a              add photo            p       pin glyph · copy palette
s              save                 r       randomize glyphs · refresh
q / ctrl-c     quit
```

The configurator is the whole config surface, not a subset. **Deck** edits composition, units, temperature, the note (a quote with its author under it, a plain line, an emoticon, or off), the logo mode, the logo text, and a font picker that lists every family fontconfig knows and renders the result live. **Glyphs** maps every fetched object individually with pinned, random and auto modes; `enter` on a glyph opens the Nerd Font glyph library directly (names and code points included; type a name or `nf-` alias to filter); `ctrl-f` switches the source font, so any installed font can supply the pinned glyph or a random pool. Per-section glyphs and a hand-editable pool sit alongside. **Palette** picks from the built-in themes or any installed palette table, edits RGB channels, and defines the rotation set; switching to light mode hides every theme without a real light variant and prunes it from the rotation set. **Artwork** is album-first: choose an album, then choose a frame to pin or rotate through the whole album; rotation mode and cadence are editable in place. `P` renders the real readout, including the actual text logo and artwork, composed exactly as the fetch will print it, with no tabs or panels in the way. `R` resets the current setting, the whole page, or the entire config after confirmation. Comments and unknown keys in `fetch.conf` survive every save. Every change lands in `fetch.conf` on `s` — the same plain-text file the renderer consumes.

Pickers are filter-first: type to narrow, `↑`/`↓` to move, `enter` to take, `esc` to cancel. In the theme-set picker, `space` marks a theme and advances.

Built on [ratatui](https://ratatui.rs); nothing runs between frames.

<br>

## ◈ Configuration

`~/.config/tachyon/fetch.conf`

```ini
# only these modules render; omit the line for all of them
enabled = os,host,kernel,uptime,session,cpu,memory,swap,disk_root,disk_media,gpu_util,gpu_vram,battery,cpu_temp,gpu_temp

# note under the readout: quote, text, emoticon, or off
note_mode = quote
quote     = random    # random pool line, or a pinned full line
note_text =           # used when note_mode = text
emoticon  = (⌐■_■)   # used when note_mode = emoticon
glyph  = ◆            # header glyph; random, or one character
layout = hud
motion = reveal
theme  = Rose Pine
rule   = ─
units  = binary
temp   = c

# logo: distro ascii art, a text logo in any system font, or an image
logo_mode = ascii
logo_art  = auto        # auto-detected from /etc/os-release; or nixos, arch...
logo_color = derive     # derive · theme · native
logo_side = left        # left · right
logo_text = TACHYON
logo_font = JetBrainsMono Nerd Font
logo_size = 110
logo_tint = duo
image     = /home/you/Pictures/board.png
album     = night

memory_warn = 85
swap_warn   = 50
disk_warn   = 90
battery_low = 15
temp_warn   = 85
temp_crit   = 95
gpu_warn    = 95

logo_width = 34
logo_min   = 26
logo_max   = 46
logo_gap   = 3

color.accent = #6ee7ff

[glyphs]
row    = auto          # auto · off · random · one character
pool   = ▸▚◆▰⬢⬡■●
system = ◆
core   = ◆
power  = ◆
memory = ⬢             # pinned
swap   = random        # drawn from the pool per run
disk_root = auto       # the built-in map for this object
# the pool can be typed here or built from any font in the configurator

[rotate]
art     = next         # off · next · shuffle
palette = next
every   = run          # run · hour · day
themes  = Solitude,Hackerman,Rose Pine
```

Fonts are resolved through fontconfig (any installed TTF/OTF family; the glyph browser also reads TTC collections for their character maps, though the text-logo rasterizer skips them). ASCII logos are detected from `/etc/os-release`; drop-in art lives in `$XDG_DATA_HOME/tachyon/logos/<id>.txt` and needs no rebuild (format in `logos/README.md`). `logo_color = native` keeps the distro's own colours, `theme` paints the art with the active palette, and `derive` (the default) lifts the logo's accents into the readout palette. The rotation set is filtered to themes that actually differ between dark and light — a duplicated dark row does not count as a light variant. Rotation applies when the fetch runs — the configurator sets the rules and previews the pinned selection. Every key above is editable inside `--tui`; the file remains the authority. Glyph keys accept `auto` (the built-in map), `random` (from the pool), `off`, or a single character. `logo_mode = image` uses the kitty graphics protocol; `text` renders the word as braille art; `ascii` uses the distro art. Rotation requires a palette set and an album; an explicit `image` or `theme` wins over rotation for that run. Rotation cursors live in `$XDG_STATE_HOME/tachyon/rotation`, never in the config.

| Resource | Default |
|---|---|
| config | `$XDG_CONFIG_HOME/tachyon/fetch.conf` |
| palettes | `$XDG_DATA_HOME/tachyon/palettes.tsv` |
| quotes | `$XDG_CONFIG_HOME/tachyon/quotes.txt` |
| state | `$XDG_STATE_HOME/tachyon/theme` |
| logo | `$XDG_DATA_HOME/tachyon/logo.png` |
| albums | `$XDG_DATA_HOME/tachyon/albums` |

Lines are `key = value` pairs; a line whose first non-space character is `#` is a comment. Inline annotations after a value are not stripped, so keep comments on their own lines.

Environment overrides: `TACHYON_FETCH_CONFIG`, `TACHYON_FETCH_PALETTES`, `TACHYON_FETCH_QUOTES`, `TACHYON_FETCH_STATE`, `TACHYON_FETCH_LOGO`, `TACHYON_FETCH_ALBUMS`.

A palette row is twelve tab-separated fields:

```text
name  variant  bg  fg  dim  accent  accent2  ok  warn  error  layout  motion
```

The six built-in themes are always available; a palette table (for example, one generated by a NixOS/Home Manager theme module) extends them and overrides rows with the same name and variant. An active Omarchy theme state is honoured when available.

<br>

## ◈ Modules

| Group | Identifiers |
|---|---|
| **System** | `os` `host` `kernel` `uptime` `session` `shell` `terminal` `compositor` `load` |
| **Core** | `cpu` `gpu` `gpu_util` `gpu_vram` `memory` `swap` |
| **Storage** | `disk_root` `disk_media` `disk_code` |
| **Network** | `ip` `link` |
| **Process** | `processes` |
| **Power** | `battery` `battery_health` `cpu_temp` `gpu_temp` |

<br>

## ◈ Design notes

- **Direct collection.** `/proc` and `/sys` are parsed in place. DMI and hwmon are discovered, not assumed.
- **NVML on demand.** The library is opened with `dlopen`; if the driver or the symbol is absent, GPU telemetry silently degrades.
- **One write.** The frame is composed in memory and emitted once, so a slow terminal never tears the layout.
- **One engine.** The `P` preview and the configurator render through the same layout code as the CLI.
- **Text logos locally.** Fonts are rasterized in-process with `fontdue`; fontconfig is only consulted when you open the picker or resolve a family.
- **Logo palettes.** ASCII art carries named colour slots; the derived mode lifts its two most saturated colours into the frame, so the logo and the panels agree without hard-coding a theme.
- **Contained unsafe.** `unsafe` is limited to the NVML ABI, termios and the terminal bindings.
- **Nothing persistent except intent.** Renders do not write state; only an explicit configurator save does.
- **No network.** Ever.

<br>

## ◈ Status

Tachyon is in active development. The collection layer is stable; the instrument panel is still being refined. Bug reports and focused patches are welcome.

<br>

<div align="center">

`╺━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╸`

**MIT** © 2026 Mohamed Salahi

<sub>Built for cold mornings and warm terminals.</sub>

</div>
