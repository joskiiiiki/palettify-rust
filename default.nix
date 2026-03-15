{
  lib,
  rustPlatform,
  glib,
  llvmPackages,
  pkg-config,
  rustc,

  nasm,
  cmake,
  clang,
  cargo,
}:
rustPlatform.buildRustPackage rec {
  pname = "palettify-rust";
  version = "0.0.1";

  src = ./.;

  buildInputs = [
    glib
    clang
    rustPlatform.bindgenHook
  ];

  nativeBuildInputs = [
    rustPlatform.bindgenHook
    cmake
    nasm
    pkg-config
    rustc
    cargo
  ];
  cargoLock = {
    lockFile = ./Cargo.lock;
  };
  cargoHash = lib.fakeHash;

  cargoBuildOptions = [
    "--release"
  ];

  LIBCLANG_PATH = "${llvmPackages.libclang.lib}";

  meta = with lib; {
    homepage = "";
    description = "Program for applying palettes";
    license = licenses.mit;
  };
}
