self: { config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.indicator-alert-daemon;
  format = pkgs.formats.json { };
  configFile = format.generate "indicator-alert-daemon.json" {
    ntfy_url = cfg.ntfyUrl;
    interval_type = cfg.intervalType;
    frequency_seconds = cfg.pollFrequency;
    tickers = cfg.tickers;
    db_path = "/var/lib/indicator-alert-daemon/data.db";
  };
in
{
  options.services.indicator-alert-daemon = {
    enable = mkEnableOption "Indicator alert daemon monitoring market conditions";
    ntfyUrl = mkOption {
      type = types.str;
      description = "ntfy server URL for alerts";
    };
    intervalType = mkOption {
      type = types.enum [ "1d" "1wk" ];
      default = "1wk";
      description = "Data sampling interval";
    };
    pollFrequency = mkOption {
      type = types.int;
      default = 86400;
      description = "Polling frequency in seconds";
    };
    tickers = mkOption {
      type = types.listOf types.attrs;
      default = [ ];
      description = ''
        Tickers with indicator configurations. Each entry is an attrset
        with `symbol` (string) and `indicators` (list of indicator configs).
        Each indicator config requires a `type` field, e.g.
        `{ type = "rsi"; threshold = 35.0; }` or
        `{ type = "bollinger_bands"; period = 20; stddev = 2.0; }`.
      '';
    };
  };

  config = mkIf cfg.enable {
    systemd.services.indicator-alert-daemon = {
      description = "Indicator Alert Daemon";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        ExecStart =
          "${self.packages.${pkgs.system}.default}/bin/indicator-alert-daemon ${configFile}";
        Restart = "always";
        RestartSec = "30";

        DynamicUser = true;
        StateDirectory = "indicator-alert-daemon";
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
