# Distribution packaging

Status: the source build works everywhere; the distro packages below are
staged and have not been built on their target systems yet. Release
tarball: `v0.7.0`, sha256 `aaafdc2804a28da836758fdab3a99cab9514d474261c2e4fc3e4532a4860fcd9`.

| Priority | Target | Package manager | Staged file | Status |
|---|---|---|---|---|
| 1 | NixOS | nix | flake outputs + `../nixpkgs-submission/` | flake builds and tests pass; nixpkgs PR pending |
| 2 | Arch Linux | pacman | `arch/PKGBUILD` | draft, needs `makepkg` on Arch |
| 3 | Fedora | dnf | `fedora/tachyon-fetch.spec` | draft, needs `fedpkg`/COPR |
| 4 | Ubuntu, Mint | apt | `debian/` | draft, needs `dpkg-buildpackage` |
| 5 | Gentoo | portage | `gentoo/` | skeleton; `CRATES` list not generated |

The release tarball is `https://github.com/mohamedsalahi/tachyon-fetch/archive/refs/tags/v0.7.0.tar.gz`.
Packaging files live in the repository after the tag, so the checksum
above is not self-referential. For the next release:

1. Tag `vX.Y.Z` and publish the GitHub release.
2. Download the tag tarball, compute `sha256sum`.
3. Update the version and checksum here and in each package recipe.
4. Run the package build on the target distro before publishing.

## Publish routes

- **NixOS**: `nix build .#tachyon-fetch` from this flake; upstream entry in
  `nixpkgs-submission/by-name/ta/tachyon-fetch/`. The nixpkgs PR is not open.
- **Arch**: create an AUR package from `arch/PKGBUILD` and regenerate
  `.SRCINFO` with `makepkg --printsrcinfo > .SRCINFO`. The binary needs
  `fontconfig` at runtime.
- **Fedora**: submit `fedora/tachyon-fetch.spec` to COPR. Upload the
  tarball to the lookaside cache (`fedpkg new-sources`) and verify with
  `fedpkg local` or a COPR build.
- **Ubuntu/Mint**: build the `debian/` tree with `dpkg-buildpackage -us -uc`.
  The Rust toolchain in older Ubuntu/Mint releases predates the crate's
  MSRV (1.88); use a current rustc or a PPA with a newer toolchain, and
  vendor the crates for an offline build before uploading.
- **Gentoo**: create an overlay with `gentoo/`, generate the `CRATES`
  variable from `Cargo.lock` (`cargo ebuild` from `app-portage/cargo-ebuild`),
  then `emerge --ask tachyon-fetch`. The ebuild is a skeleton until then.

## Runtime dependencies

- `fontconfig` (the glyph/font pickers call `fc-list` and `fc-match`).
- A terminal with UTF-8; kitty graphics for the image logo, otherwise the
  ASCII or text logo is used.
- No daemon, network access or writable state is required to render.
