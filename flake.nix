{
  description = "Stream Cave flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "Stream Cave";
          version = "0.1.0";

          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          buildInputs = with pkgs; [
            openssl
            mpv
            yt-dlp
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          UseNextest = true;
          checkFlags = [
            "--skip watcher::twitch_socket"
            "--skip watcher::player"
            "--skip watcher::tasks_handler"
          ];
        };
        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.default ];
          packages = with pkgs; [
            rust-bin.stable.latest.default
            twitch-cli
            jq
            streamlink
            cargo-mutants
            cargo-nextest
            cargo-fuzz
          ];
        };
      }
    );
}
