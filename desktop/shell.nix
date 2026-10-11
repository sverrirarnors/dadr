# Build and run the desktop app on NixOS: `nix-shell desktop/shell.nix`.
# Other Linux distributions install the same libraries from their own
# packages (see desktop/README.md).
{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    pkg-config
    cargo-tauri
    librsvg # rsvg-convert, for `make desktop-icons`
  ];

  buildInputs = with pkgs; [
    webkitgtk_4_1
    libsoup_3
    gtk3
    openssl
  ];

  # GTK's file chooser aborts when its GSettings schemas are missing, and
  # nothing on a NixOS system path provides them to a binary run from here.
  shellHook = ''
    export XDG_DATA_DIRS="${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}''${XDG_DATA_DIRS:+:$XDG_DATA_DIRS}"
  '';
}
