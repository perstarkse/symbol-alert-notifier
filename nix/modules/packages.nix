{
  perSystem = { pkgs, craneLib, ... }: {
    packages.default = craneLib.buildPackage {
      src = craneLib.cleanCargoSource (craneLib.path ../../.);
      strictDeps = true;

      meta = {
        description = "Daemon monitoring market indicators for ticker alerts";
        license = pkgs.lib.licenses.mit;
      };
    };
  };
}
