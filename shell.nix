{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  packages = with pkgs; [
    pkg-config
    alsa-lib
    fontconfig
    freetype
    libxkbcommon
    wayland
    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr
    xorg.libXext
    xorg.libxcb
    xorg.libXrender
    xorg.libXfixes
    xorg.libxkbfile
    mesa
    vulkan-loader
    libGL
    dbus
  ];

  shellHook = ''
    export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath [
      pkgs.alsa-lib
      pkgs.fontconfig
      pkgs.freetype
      pkgs.libxkbcommon
      pkgs.wayland
      pkgs.xorg.libX11
      pkgs.xorg.libXcursor
      pkgs.xorg.libXi
      pkgs.xorg.libXrandr
      pkgs.xorg.libXext
      pkgs.xorg.libxcb
      pkgs.xorg.libXrender
      pkgs.xorg.libXfixes
      pkgs.xorg.libxkbfile
      pkgs.mesa
      pkgs.vulkan-loader
      pkgs.libGL
      pkgs.dbus
    ]}:$LD_LIBRARY_PATH"
  '';
}
