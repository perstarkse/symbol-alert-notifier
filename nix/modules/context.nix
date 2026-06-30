{ inputs, ... }: {
  perSystem = { system, pkgs, ... }: {
    _module.args =
      let
        rustToolchain = inputs.fenix.packages.${system}.stable.withComponents [
          "cargo"
          "rustc"
          "rustfmt"
          "clippy"
          "llvm-tools-preview"
        ];
      in
      {
        inherit rustToolchain;
        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;
      };
  };
}
