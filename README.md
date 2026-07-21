# indicator-alert-daemon

Monitor financial indicators via Yahoo Finance and fire alerts to [ntfy](https://ntfy.sh/) when conditions are met.

## Features

- **Four indicators** — RSI, MACD, Bollinger Bands, MA Crossover
- **Alert deduplication** — fires only on state transitions, not every poll cycle
- **Graceful shutdown** — handles SIGTERM/SIGINT cleanly
- **SQLite-backed** — persists price data and indicator state for restart safety
- **NixOS module** — drop-in systemd service with sandboxed execution
- **Configurable** — JSON config with env var overrides

## Quick start

```json
{
    "ntfy_url": "https://ntfy.sh/my-topic",
    "interval_type": "1wk",
    "frequency_seconds": 86400,
    "tickers": [
        {
            "symbol": "KOID",
            "indicators": [
                { "type": "rsi", "threshold": 35.0, "period": 14 }
            ]
        }
    ]
}
```

```
indicator-alert-daemon --config example-config.json
indicator-alert-daemon --help
```

Logs at `info` level by default. Set `RUST_LOG=debug` for more detail.

## Installation

### Cargo

```
cargo install indicator-alert-daemon
```

### Nix

```nix
{
  inputs.indicator-alert-daemon.url = "github:your-org/indicator-alert-daemon";

  outputs = { self, nixpkgs, indicator-alert-daemon, ... }: {
    nixosConfigurations.my-box = nixpkgs.lib.nixosSystem {
      modules = [ indicator-alert-daemon.nixosModules.default ];
    };
  };
}
```

## Configuration

All fields in `config.json`:

| Field | Type | Default | Description |
|---|---|---|---|
| `ntfy_url` | string | — | ntfy topic URL (required, `https://` only) |
| `interval_type` | string | `"1wk"` | Default `"1d"` or `"1wk"` (overridable per ticker) |
| `frequency_seconds` | int | `86400` | How often to poll |
| `ticker_delay_ms` | int | `1500` | Delay between ticker API calls |
| `max_retries` | int | `3` | Yahoo Finance retry attempts |
| `retry_base_delay_ms` | int | `500` | Base delay (doubles per attempt) |
| `db_path` | string | — | DB location (see resolution order below) |
| `tickers` | array | — | List of symbols + indicator configs |

Each ticker entry:

| Field | Type | Default | Description |
|---|---|---|---|
| `symbol` | string | — | Yahoo Finance symbol (required) |
| `interval_type` | string | daemon default | Optional `"1d"` or `"1wk"` override for this ticker |
| `indicators` | array | — | Indicator configs (required, non-empty) |

**DB path resolution**: config > `DB_PATH` env > `STATE_DIRECTORY` > `XDG_DATA_HOME` > `~/.local/share/indicator-alert-daemon` > `./data.db`

**Env overrides**: `NTFY_URL`, `DB_PATH`, `INTERVAL_TYPE`, `FREQUENCY_SECONDS`, `TICKER_DELAY_MS`

## Indicators

### RSI

Triggers when RSI crosses the configured side of `threshold`.

| Field | Default | Description |
|---|---|---|
| `period` | `14` | RSI calculation period |
| `threshold` | — | RSI level to compare against (0–100) |
| `direction` | `"below"` | `"below"` alerts when RSI < threshold (buy / oversold); `"above"` alerts when RSI > threshold (sell / overbought) |

```json
{ "type": "rsi", "threshold": 30.0, "direction": "below" }
{ "type": "rsi", "threshold": 70.0, "direction": "above" }
```

### MACD

Triggers on bullish or bearish crossover.

| Field | Default | Description |
|---|---|---|
| `fast` | `12` | Fast EMA period |
| `slow` | `26` | Slow EMA period |
| `signal` | `9` | Signal line period |

### Bollinger Bands

Triggers when price touches or breaks the lower band.

| Field | Default | Description |
|---|---|---|
| `period` | `20` | SMA period |
| `stddev` | `2.0` | Standard deviation multiplier |

### MA Crossover

Triggers on Golden Cross (fast SMA crosses above slow SMA) or Death Cross (fast SMA crosses below slow SMA).

| Field | Description |
|---|---|
| `fast_period` | Fast SMA period (must be < `slow_period`) |
| `slow_period` | Slow SMA period |

## Architecture

```
main.rs              — CLI entry point
  ↓
lib.rs               — run loop, signal handling, market evaluation
  ├── config.rs      — JSON parsing, validation, env overrides
  ├── yahoo.rs       — Yahoo Finance API types and URL builder
  ├── data.rs        — OHLCV extraction, price helpers
  ├── db.rs          — SQLite: schema, CRUD, dedup, purge
  ├── alert.rs       — ntfy push alerts
  └── indicators/    — trait-based indicator dispatch
      ├── rsi.rs
      ├── macd.rs
      ├── bb.rs
      └── crossover.rs
```

## Development

The recommended development environment uses Nix and `direnv`. To activate the developer shell automatically upon entering the directory:

```
direnv allow
```

Alternatively, you can manually enter the developer shell:

```
nix develop
```

Then the usual Rust commands are available:

```
cargo test
cargo clippy
cargo fmt
cargo llvm-cov
```

To run all formatting, linting, tests, and coverage checks hermetically, run:

```
nix flake check
```

Or to format the entire project:

```
nix fmt
```

## NixOS module

```nix
services.indicator-alert-daemon = {
  enable = true;
  ntfyUrl = "https://ntfy.sh/my-topic";
  intervalType = "1wk";
  pollFrequency = 3600;
  tickers = [
    {
      symbol = "KOID";
      indicators = [
        { type = "rsi"; threshold = 35.0; }
      ];
    }
    {
      symbol = "ETH-USD";
      intervalType = "1d";
      indicators = [
        { type = "rsi"; threshold = 30.0; }
      ];
    }
  ];
};
```

Import the flake module as `inputs.indicator-alert-daemon.nixosModules.default` (there is no root `module.nix`).

The systemd service runs with `DynamicUser`, `ProtectSystem=strict`, `ProtectHome=true`, `PrivateTmp`, and `NoNewPrivileges`.

## License

MIT
