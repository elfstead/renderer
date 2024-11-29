{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
	  inherit system overlays;
	};
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
	rpathLibs = with pkgs; [
	  wayland
	  libxkbcommon
	  # choose either vulkan or opengl
	  vulkan-loader
	  #libGL
	];
      in
      with pkgs;
      rec
      {
        packages.default = (makeRustPlatform {
	  cargo = toolchain;
	  rustc = toolchain;
	}).buildRustPackage {
	  pname = "renderer";
	  version = "0.1";
	  src = lib.cleanSource ./.;
	  cargoLock.lockFile = ./Cargo.lock;
	  nativeBuildInputs = [
	    autoPatchelfHook
	  ];
	  buildInputs = [
	    libgcc
	  ];
	  runtimeDependencies = [
	    wayland
	    libxkbcommon
	    vulkan-loader
	  ];
	};
	# main point of specifying devShell is to add rust-analyzer only for that
	devShells.default = mkShell {
	  inputsFrom = [ packages.default ];
	  packages = [
	    # this ensures all versions are compatible
	    (toolchain.override {
	      extensions = [
	        "rustc"
	        "cargo"
	        "clippy"
	        "rustfmt"
	        "rust-src"
	        "rust-analyzer"
	      ];
	    })
	  ];
	  LD_LIBRARY_PATH = lib.makeLibraryPath rpathLibs;
	};
      }
    );
}
