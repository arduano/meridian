{
  description = "Meridian development shell and Linux package outputs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        lib = pkgs.lib;
        meridianSrc = builtins.path {
          path = ./.;
          name = "meridian";
        };
        midiToolkitSrc = builtins.path {
          path = ../midi-toolkit-rs;
          name = "midi-toolkit-rs";
        };
        xsynthSrc = builtins.path {
          path = ../xsynth;
          name = "xsynth";
        };
        workspaceSrc = pkgs.runCommand "meridian-workspace-src" {} ''
          mkdir -p "$out"
          cp -a ${meridianSrc} "$out/meridian"
          cp -a ${midiToolkitSrc} "$out/midi-toolkit-rs"
          cp -a ${xsynthSrc} "$out/xsynth"
        '';
        commonNativeBuildInputs = with pkgs; [
          pkg-config
        ];
        commonBuildInputs = with pkgs; [
          alsa-lib
          dbus
          fontconfig
          freetype
          libGL
          libxkbcommon
          mesa
          vulkan-loader
          wayland
          xorg.libX11
          xorg.libXcursor
          xorg.libXext
          xorg.libXfixes
          xorg.libXi
          xorg.libXrandr
          xorg.libXrender
          xorg.libxcb
          xorg.libxkbfile
        ];
        mkMeridianPackage = { pname, cargoBuildFlags, installPhase }:
          pkgs.rustPlatform.buildRustPackage {
            inherit pname;
            version = "0.1.0";
            src = workspaceSrc;
            sourceRoot = "source/meridian";
            cargoLock = {
              lockFile = ./Cargo.lock;
            };
            nativeBuildInputs = commonNativeBuildInputs;
            buildInputs = commonBuildInputs;
            inherit cargoBuildFlags installPhase;
          };
        meridianCli = mkMeridianPackage {
          pname = "meridian";
          cargoBuildFlags = [ "-p" "meridian-stdio" ];
          installPhase = ''
            runHook preInstall
            mkdir -p $out/bin
            install -m 0755 "$(find target -path '*/release/meridian-stdio' -type f | head -n 1)" $out/bin/meridian
            runHook postInstall
          '';
        };
        meridianUi = mkMeridianPackage {
          pname = "meridian-ui";
          cargoBuildFlags = [ "-p" "meridian-ui" ];
          installPhase = ''
            runHook preInstall
            mkdir -p $out/bin $out/share/applications $out/share/icons/hicolor/256x256/apps
            install -m 0755 "$(find target -path '*/release/meridian-ui' -type f | head -n 1)" $out/bin/meridian-ui
            install -m 0644 assets/icons/meridian-256.png $out/share/icons/hicolor/256x256/apps/io.github.arduano.meridian.png
            cat > $out/share/applications/io.github.arduano.meridian.desktop <<'EOF'
[Desktop Entry]
Type=Application
Name=Meridian
GenericName=MIDI Visualizer
Comment=Desktop MIDI visualizer frontend for Meridian
Exec=meridian-ui
Icon=io.github.arduano.meridian
Categories=AudioVideo;Audio;Music;
Terminal=false
EOF
            runHook postInstall
          '';
        };
      in {
        packages = {
          meridian = meridianCli;
          "meridian-ui" = meridianUi;
          default = meridianCli;
        };

        apps = {
          default = {
            type = "app";
            program = "${meridianCli}/bin/meridian";
          };
          meridian = {
            type = "app";
            program = "${meridianCli}/bin/meridian";
          };
          "meridian-ui" = {
            type = "app";
            program = "${meridianUi}/bin/meridian-ui";
          };
        };

        devShells.default = pkgs.mkShell {
          packages = commonNativeBuildInputs ++ commonBuildInputs ++ [
            pkgs.cargo
            pkgs.rustc
          ];

          shellHook = ''
            export LD_LIBRARY_PATH="${lib.makeLibraryPath commonBuildInputs}:$LD_LIBRARY_PATH"
          '';
        };
      });
}
