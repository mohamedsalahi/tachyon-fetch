# ASCII logos

The fetch picks a logo from `/etc/os-release` (`ID`, then `ID_LIKE`) and
falls back to the Tux mark. Built-in ids: nixos, arch, fedora, ubuntu,
mint, gentoo, debian, opensuse, alpine, void, manjaro, pop, kali, linux.

## Drop in a logo without rebuilding

Write `$XDG_DATA_HOME/tachyon/logos/<id>.txt` (default
`~/.local/share/tachyon/logos/<id>.txt`) and set `logo_art = <id>` or pick
it under Deck → ascii art. `--logos <dir>` points at another directory and
`--list-logos` prints every available id.

The format is plain text:

```text
# colors: 5277C3,7EBAE4,FFFFFF
   $1/\
  $1/  $2/\
  $1/____\
```

- The optional first line declares colour slots as comma-separated
  `RRGGBB` values (with or without `#`). Without it the active palette is
  used.
- `$1` … `$9` switch the colour slot for the text that follows.
- Lines are printed verbatim; trailing blank lines are trimmed.
- The art is used on the left or right depending on `logo_side`.
- `logo_color = native` keeps these colours, `theme` uses the active
  palette, and `derive` lifts the two most saturated slots into the frame
  palette (the default).

## Ship a logo in the binary

Add `<id>.txt` and optionally `<id>-small.txt` here, then add one `Art`
entry to `ARTS` in `src/ascii.rs` with the id and its default colour
slots. The compact file is chosen automatically when the wide art does
not fit the terminal.

`LICENSE-fastfetch` and `NOTICE.md` cover the imported defaults.
