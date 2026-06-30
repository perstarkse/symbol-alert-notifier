{ inputs, ... }: {
  imports = [
    inputs.git-hooks-nix.flakeModule
  ];

  perSystem = { config, pkgs, rustToolchain, system, ... }:
    let
      fenixPkgs = inputs.fenix.packages.${system};
      llvmTools = fenixPkgs.stable.llvm-tools-preview;
      rustTarget = pkgs.stdenv.hostPlatform.rust.rustcTarget or pkgs.stdenv.hostPlatform.config;
      llvmToolsBin = "${llvmTools}/lib/rustlib/${rustTarget}/bin";
    in
    {
      pre-commit = {
        settings = {
          hooks = {
            rustfmt.enable = true;
          };
        };
      };

      devShells.default = pkgs.mkShell {
        name = "symbol-alert-notifier-shell";
        inputsFrom = [ config.pre-commit.devShell ];

        packages = [
          rustToolchain
          pkgs.cargo-llvm-cov
          pkgs.git
          pkgs.curl
        ];

        env = {
          LLVM_COV = "${llvmToolsBin}/llvm-cov";
          LLVM_PROFDATA = "${llvmToolsBin}/llvm-profdata";
        };

        shellHook = ''
          echo "======================================================="
          echo "  Symbol Alert Notifier Developer Shell                "
          echo "  Toolchain: Fenix Rust Stable                         "
          echo "  Run 'cargo test' or 'cargo llvm-cov' for checks       "
          echo "======================================================="
        '';
      };
    };
}
