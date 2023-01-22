with (import <nixpkgs> { });
mkShell {
  buildInputs = [
    pkgs.SDL2
    pkgs.SDL2_ttf
    pkgs.pkgconfig
  ];
}
