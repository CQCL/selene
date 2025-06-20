{ pkgs, lib, inputs, config, ... }:
{
  config = {
    packages = with pkgs; [
      cmake
      just
      zlib
      libxml2
      ncurses
      rust-cbindgen
      graphviz
      graph-easy
      libffi
    ];

    enterShell = ''
      eval "$(just --completions bash)"
      export LD_LIBRARY_PATH="${lib.makeLibraryPath [ pkgs.stdenv.cc.cc ]}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
    '';

    env = {
      CARGO_NET_GIT_FETCH_WITH_CLI = "true";
      LIBCLANG_PATH = "${pkgs.libclang.lib}";
      LLVM_SYS_140_PREFIX = "${pkgs.llvmPackages_14.libllvm.dev}";
    };


    languages.python = {
      enable = true;
      uv.enable = true;
      uv.sync.enable = true;
      venv.enable = true;
    };

    languages.rust = {
      enable = true;
      rustflags = "-L${config.env.DEVENV_PROFILE}/lib";
      components = [ "rustc" "cargo" "clippy" "rustfmt" "rust-analyzer" ];
    };

    git-hooks.hooks = {
      ruff = {
        enable = true;
        args = ["--config=pyproject.toml"];
        pass_filenames = false;
      };
      ruff-format = {
        enable = true;
        args = ["--config=pyproject.toml"];
        pass_filenames = false;
      };
      clippy = {
        enable = true;
        settings = {
          denyWarnings = true;
        };
      };
      cargo-check = {
        enable = true;
      };
      rustfmt = {
        enable = true;
      };
      mypy = {
        enable = true;
        args = ["--config=pyproject.toml"];
        extraPackages = with pkgs.python3Packages; [
          types-pyyaml
        ];
        excludes = ["selene-sim/python/tests" "selene-compilers/hugr_qis/python/tests"];
      };
    };
  };
}
