# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- RSI `direction` config field: `"below"` (default, buy/oversold) or `"above"` (sell/overbought)
- CLI `--help` / `-h` usage text; missing config path now exits with usage instead of panicking

### Changed

- CLI requires `--config <path>` instead of a positional config argument
- NixOS module `ExecStart` passes `--config` to match the new CLI
- Example config includes an RSI overbought (`direction: "above"`) alert on TSLA
