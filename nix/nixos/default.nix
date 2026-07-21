self: { config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.indicator-alert-daemon;
  format = pkgs.formats.json { };
  tickersJson = map
    (
      t:
      {
        inherit (t) symbol indicators;
      }
      // optionalAttrs (t.intervalType != null) { interval_type = t.intervalType; }
    )
    cfg.tickers;
  configFile = format.generate "indicator-alert-daemon.json" (
    {
      ntfy_url = cfg.ntfyUrl;
      interval_type = cfg.intervalType;
      frequency_seconds = cfg.pollFrequency;
      ticker_delay_ms = cfg.tickerDelayMs;
      tickers = tickersJson;
    }
    // optionalAttrs (cfg.dbPath != null) { db_path = cfg.dbPath; }
  );
in
{
  options.services.indicator-alert-daemon = {
    enable = mkEnableOption "Indicator alert daemon monitoring market conditions";
    ntfyUrl = mkOption {
      type = types.str;
      description = "ntfy server URL for alerts";
    };
    intervalType = mkOption {
      type = types.enum [
        "1d"
        "1wk"
      ];
      default = "1wk";
      description = "Default data sampling interval (overridable per ticker)";
    };
    pollFrequency = mkOption {
      type = types.int;
      default = 86400;
      description = "Polling frequency in seconds";
    };
    tickerDelayMs = mkOption {
      type = types.int;
      default = 1500;
      description = "Delay between ticker fetches in milliseconds";
    };
    dbPath = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Path to the SQLite database file (default: under /var/lib/indicator-alert-daemon)";
    };
    tickers = mkOption {
      type = types.listOf (
        types.submodule {
          options = {
            symbol = mkOption {
              type = types.str;
              description = "Ticker symbol";
            };
            intervalType = mkOption {
              type = types.nullOr (
                types.enum [
                  "1d"
                  "1wk"
                ]
              );
              default = null;
              description = "Per-ticker sampling interval; defaults to services.indicator-alert-daemon.intervalType";
            };
            indicators = mkOption {
              type = types.listOf (types.attrsOf types.anything);
              description = ''
                List of indicator configurations. Each entry is an attrset
                with at least a `type` field, e.g.
                `{ type = "rsi"; threshold = 35.0; period = 14; }` or
                `{ type = "rsi"; threshold = 70.0; direction = "above"; }` or
                `{ type = "bollinger_bands"; period = 20; stddev = 2.0; }`.
              '';
            };
          };
        }
      );
      default = [ ];
      description = "Tickers with indicator configurations";
    };
  };

  config = mkIf cfg.enable {
    systemd.services.indicator-alert-daemon = {
      description = "Indicator Alert Daemon";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        ExecStart = "${self.packages.${pkgs.system}.default}/bin/indicator-alert-daemon --config ${configFile}";
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
