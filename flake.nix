{
  # EmoteForge の開発環境を Nix flake で固定する。
  # 実体は Tauri アプリ(src-tauri)+ Rust ワークスペース(core)+ Vite フロント。
  # Tauri の Windows インストーラ生成は CI(release.yml)側で行うため、
  # ここでは「ローカル開発に必要な道具一式」を devShell として提供するに留める。
  description = "EmoteForge — FiveM emote studio (dev environment)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      # 開発機は WSL2(x86_64-linux)だが、他環境でも壊れないよう主要システムを列挙。
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          # rustc/cargo は nixpkgs-unstable が 1.95(プロジェクト要件)を提供する。
          # rust-overlay は使わず nixpkgs の素の toolchain に揃える。
          nativeBuildInputs = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            nodejs_22
            pkg-config
            # ブリッジの構文チェック(luac -p)と drift 検証で使う。
            lua5_4
          ];

          # Tauri v2 の Linux ビルド/実行に必要なシステムライブラリ。
          # Linux 以外では空集合になり devShell には影響しない。
          buildInputs = nixpkgs.lib.optionals pkgs.stdenv.isLinux (
            with pkgs;
            [
              webkitgtk_4_1
              gtk3
              libsoup_3
              librsvg
              libayatana-appindicator
              openssl
              glib
              cairo
              pango
              gdk-pixbuf
              atk
            ]
          );

          # cargo tauri dev は webkit 等を実行時に dlopen するため、
          # NixOS では LD_LIBRARY_PATH を通さないとリンク/起動に失敗する。
          shellHook = nixpkgs.lib.optionalString pkgs.stdenv.isLinux ''
            export LD_LIBRARY_PATH="${
              nixpkgs.lib.makeLibraryPath (
                with pkgs;
                [
                  webkitgtk_4_1
                  gtk3
                  libsoup_3
                  librsvg
                  libayatana-appindicator
                  openssl
                  glib
                  # GTK/WebKit が実行時に dlopen で引く依存。buildInputs と揃える。
                  cairo
                  pango
                  gdk-pixbuf
                  atk
                ]
              )
            }:$LD_LIBRARY_PATH"
          '';
        };
      });

      # `nix fmt` で flake 自身を整形できるようにしておく。
      formatter = forAllSystems (pkgs: pkgs.nixfmt);
    };
}
