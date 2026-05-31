{
  pkgs,
  lib,
  config,
  inputs,
  ...
}: {
  languages.rust.enable = true;
  git-hooks.install.enable = true;
  git-hooks.hooks = {
    rustfmt.enable = true;
    clippy = {
      enable = true;
      settings.allFeatures = true;
    };
    "cargo-test" = {
      enable = true;
      name = "Cargo tests";
      entry = "cargo test";
      language = "system";
      pass_filenames = false;
    };
    "cargo-coverage" = {
      enable = true;
      name = "Coverage summary";
      entry = "cargo llvm-cov --workspace --all-features --summary-only --fail-under-lines 90";
      language = "system";
      pass_filenames = false;
    };
  };
}
