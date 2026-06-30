{ inputs, ... }: {
  imports = [
    inputs.treefmt-nix.flakeModule
  ];

  perSystem = { pkgs, rustToolchain, ... }: {
    treefmt = {
      projectRootFile = "flake.nix";
      programs = {
        nixpkgs-fmt.enable = true;
      };
      settings.formatter.rustfmt = {
        command = "${rustToolchain}/bin/rustfmt";
        options = [ "--config" "skip_children=true" "--edition" "2024" ];
        includes = [ "*.rs" ];
      };
    };
  };
}
