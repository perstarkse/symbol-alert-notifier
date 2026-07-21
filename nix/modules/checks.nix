{ self, inputs, ... }: {
  perSystem = { system, pkgs, craneLib, self', ... }:
    let
      commonArgs = {
        src = craneLib.cleanCargoSource (craneLib.path ../../.);
        strictDeps = true;
      };

      cargoArtifacts = craneLib.buildDepsOnly commonArgs;
    in
    {
      checks = {
        inherit (self'.packages) default;

        clippy = craneLib.cargoClippy (commonArgs // {
          inherit cargoArtifacts;
          cargoClippyExtraArgs = "--workspace --all-targets --all-features -- -D warnings";
        });

        tests = craneLib.cargoTest (commonArgs // {
          inherit cargoArtifacts;
          cargoTestExtraArgs = "--workspace";
        });

        coverage = craneLib.cargoBuild (commonArgs // {
          inherit cargoArtifacts;
          pname = "indicator-alert-daemon-coverage";
          cargoBuildCommand = "cargo llvm-cov --workspace --all-features --summary-only --fail-under-lines 90";
          nativeBuildInputs = with pkgs; [
            cargo-llvm-cov
          ];
        });

        nixos-module-eval =
          let
            eval = inputs.nixpkgs.lib.nixosSystem {
              inherit system;
              modules = [
                self.nixosModules.default
                {
                  fileSystems."/" = { device = "/dev/null"; fsType = "ext4"; };
                  boot.loader.grub.enable = false;

                  services.indicator-alert-daemon = {
                    enable = true;
                    ntfyUrl = "https://ntfy.sh/test";
                    tickers = [
                      {
                        symbol = "AAPL";
                        indicators = [
                          { type = "rsi"; threshold = 30.0; }
                        ];
                      }
                    ];
                  };
                }
              ];
            };
          in
          pkgs.runCommand "test-module-eval"
            {
              execStart = eval.config.systemd.services.indicator-alert-daemon.serviceConfig.ExecStart;
            } ''
            echo "Evaluated intervalType: ${eval.config.services.indicator-alert-daemon.intervalType}"
            echo "ExecStart: $execStart"
            echo "$execStart" | grep -q -- '--config '
            touch $out
          '';
      };
    };
}
