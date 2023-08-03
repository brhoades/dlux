{
  description = "dlux hardware brightness daemon";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.05";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }: let
    pkgsFor = system: import nixpkgs {
      inherit system;
    }; in (flake-utils.lib.eachDefaultSystem (system: 
    let pkgs = pkgsFor system; in  {
      devShells.default = with (pkgsFor system); mkShell {
        nativeBuildInputs = [
          rustc
          rustfmt
          clippy
          cargo
          udev
          pkg-config

          llvmPackages.libclang
          llvmPackages.clang
          llvmPackages.clang.libc.dev
        ];
      };

      shellHook = ''
        # libclang.so
        # export LIBCLANG_PATH="$(nix-store -r $(nix-instantiate '<nixpkgs>' -A llvmPackages.libclang))/lib"
        # types.h
        # TYPES_PATH="$(nix-store -r $(nix-instantiate '<nixpkgs>' -A llvmPackages.clang.libc.dev))/include"
        # stddef.h
        # STD_PATH="$(nix-store -r $(nix-instantiate '<nixpkgs>' -A llvmPackages.clang))/resource-root/include"

        # libclang.so
        export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.out}/lib"
        # types.h
        TYPES_PATH="${pkgs.llvmPackages.clang.libc.dev}/include"
        # stddef.h
        STD_PATH="${pkgs.llvmPackages.clang.out}/resource-root/include"

        export BINDGEN_EXTRA_CLANG_ARGS="-I $TYPES_PATH -I $STD_PATH"
 
      '';
    }));
}

