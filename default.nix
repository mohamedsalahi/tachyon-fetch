{ pkgs ? import <nixpkgs> { } }:
pkgs.rustPlatform.buildRustPackage {
  pname = "tachyon-fetch";
  version = (pkgs.lib.importTOML ./Cargo.toml).package.version;

  src = pkgs.lib.cleanSourceWith {
    src = ./.;
    filter =
      path: _:
      !(builtins.elem (baseNameOf path) [
        "target"
        "result"
        ".git"
      ]);
  };

  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = [
    pkgs.makeWrapper
    pkgs.versionCheckHook
  ];

  postInstall = ''
    wrapProgram $out/bin/tachyon \
      --suffix PATH : ${pkgs.lib.makeBinPath [ pkgs.fontconfig ]}
  '';

  meta = {
    description = "Cockpit-grade system fetch with a built-in configurator and distro logos";
    homepage = "https://github.com/mohamedsalahi/tachyon-fetch";
    license = pkgs.lib.licenses.mit;
    mainProgram = "tachyon";
    platforms = pkgs.lib.platforms.linux;
  };
}
