self: { config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.rsi-alert-daemon;
  format = pkgs.formats.json { };
  configFile = format.generate "rsi-alert-daemon.json" {
    ntfy_url = cfg.ntfyUrl;
    interval_type = cfg.timeframe;
    frequency_seconds = cfg.pollFrequency;
    tickers = map (t: { symbol = t.symbol; threshold = t.threshold; }) cfg.watchlist;
  };
in
{
  options.services.rsi-alert-daemon = {
    enable = mkEnableOption "RSI alert daemon monitoring macro boundaries";
    ntfyUrl = mkOption {
      type = types.str;
      description = "ntfy server URL for alerts";
    };
    timeframe = mkOption {
      type = types.enum [ "1d" "1wk" ];
      default = "1wk";
      description = "Data sampling interval";
    };
    pollFrequency = mkOption {
      type = types.int;
      default = 86400;
      description = "Polling frequency in seconds";
    };
    watchlist = mkOption {
      type = types.listOf (
        types.submodule {
          options = {
            symbol = mkOption {
              type = types.str;
              description = "Market ticker symbol";
            };
            threshold = mkOption {
              type = types.float;
              default = 35.0;
              description = "RSI alert threshold";
            };
          };
        }
      );
      default = [ ];
      description = "Tickers to monitor";
    };
  };

  config = mkIf cfg.enable {
    systemd.services.rsi-alert-daemon = {
      description = "RSI Alert Daemon";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        ExecStart =
          "${self.packages.${pkgs.system}.default}/bin/rsi-alert-daemon ${configFile}";
        Restart = "always";
        RestartSec = "30";

        DynamicUser = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        NoNewPrivileges = true;
        CapabilityBoundingSet = "";
        RestrictRealtime = true;
      };
    };
  };
}
