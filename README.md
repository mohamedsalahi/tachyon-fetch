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
  <img alt="Release binary" src="https://img.shields.io/badge/BINARY-1.6_MB-8957e5?style=for-the-badge&labelColor=1b1f27">
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
| **Configurator** | `--tui` pages for modules, appearance, palette, thresholds and artwork, with a live preview |
| **Palette engine** | reads a TSV palette table and the active theme state; built-in Solitude fallback |
| **Artwork** | kitty graphics protocol with placement that never reflows text |
| **Album rotation** | `$XDG_DATA_HOME/tachyon/albums/<name>/*.png`, picked deterministically with `--seed` |
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

NOTE  Simplify until it hurts, then stop.
```

<br>

## ◈ Command line

| Flag | Effect |
|---|---|
| `--demo` | render synthetic values instead of host reads |
| `--tui` | open the module and appearance configurator |
| `--plain` | no colour, no image |
| `--static` | disable the reveal animation |
| `--no-logo` | skip the kitty image |
| `--layout <hud\|stack>` | choose the composition |
| `--glyph <char\|random>` | section marker |
| `--album <name>` | pull artwork from an album |
| `--theme <name>` | override the active palette |
| `--variant <dark\|light>` | force the palette variant |
| `--seed <n>` | deterministic layout, glyph and artwork choice |
| `--logo <path>` | explicit PNG for the kitty graphics protocol |
| `--palettes <path>` | alternate palette table |
| `--quotes <path>` | alternate note pool |
| `--config <path>` | alternate module/appearance config |
| `--state <path>` | alternate theme state file |
| `--list-themes` | print the palette table |
| `--convert <src> <dst>` | convert an image to PNG (used by album ingest) |
| `-h`, `-V` | help and version |

<br>

## ◈ The configurator

```sh
tachyon --tui
```

Seven pages, one engine: **Modules · Appearance · Palette · Thresholds · Artwork · Preview · Keys**.

```text
tab / 1-7      switch page          space   toggle module
j k / ↑ ↓      move · scroll        enter   edit · apply
h l / ← →      adjust value         x       clear override
a              add photo            p       copy palette to overrides
s              save                 r       refresh live data
q / ctrl-c     quit
```

The Palette page edits all eight palette slots with an RGB editor and live swatches; Thresholds tunes every warning level against the current reading; Artwork edits the image path, album and placement; Preview renders the real readout with memory and swap sparklines. Every change lands in `fetch.conf` on `s` — the same plain-text file the renderer consumes.

Built on [ratatui](https://ratatui.rs); nothing runs between frames.

<br>

## ◈ Configuration

`~/.config/tachyon/fetch.conf`

```ini
# only these modules render; omit the line for all of them
enabled = os,host,kernel,uptime,session,cpu,memory,swap,disk_root,disk_media,gpu_util,gpu_vram,battery,cpu_temp,gpu_temp

glyph  = ◆
layout = hud
motion = reveal
theme  = Rose Pine
rule   = ─
units  = binary
temp   = c
image  = /home/you/Pictures/board.png
album  = night

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
```

| Resource | Default |
|---|---|
| config | `$XDG_CONFIG_HOME/tachyon/fetch.conf` |
| palettes | `$XDG_DATA_HOME/tachyon/palettes.tsv` |
| quotes | `$XDG_CONFIG_HOME/tachyon/quotes.txt` |
| state | `$XDG_STATE_HOME/tachyon/theme` |
| logo | `$XDG_DATA_HOME/tachyon/logo.png` |
| albums | `$XDG_DATA_HOME/tachyon/albums` |

Environment overrides: `TACHYON_FETCH_CONFIG`, `TACHYON_FETCH_PALETTES`, `TACHYON_FETCH_QUOTES`, `TACHYON_FETCH_STATE`, `TACHYON_FETCH_LOGO`, `TACHYON_FETCH_ALBUMS`.

A palette row is twelve tab-separated fields:

```text
name  variant  bg  fg  dim  accent  accent2  ok  warn  error  layout  motion
```

If no palette table is present, Tachyon falls back to its own Solitude palette. An active Omarchy theme state is honoured when available.

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
- **One engine.** The configurator preview renders through the same layout code as the CLI.
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
