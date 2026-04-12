{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  packages = with pkgs; [
    pkg-config
    ffmpeg
    deno
    alsa-lib
    fontconfig
    freetype
    libxkbcommon
    wayland
    libx11
    libxcursor
    libxi
    libxrandr
    libxext
    libxcb
    libxrender
    libxfixes
    libxkbfile
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
      pkgs.ffmpeg
      pkgs.libxkbcommon
      pkgs.wayland
      pkgs.libx11
      pkgs.libxcursor
      pkgs.libxi
      pkgs.libxrandr
      pkgs.libxext
      pkgs.libxcb
      pkgs.libxrender
      pkgs.libxfixes
      pkgs.libxkbfile
      pkgs.mesa
      pkgs.vulkan-loader
      pkgs.libGL
      pkgs.dbus
    ]}:$LD_LIBRARY_PATH"
  '';
}
