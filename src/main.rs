fn print_help() {
    println!(
        "\
indicator-alert-daemon — monitor indicators and push ntfy alerts

Usage:
  indicator-alert-daemon --config <config.json>
  indicator-alert-daemon -h | --help

Options:
  --config <config.json>   Path to JSON configuration file
                           (see example-config.json in the repo)
  -h, --help               Show this help message and exit

Examples:
  indicator-alert-daemon --config example-config.json
  indicator-alert-daemon --config /etc/indicator-alert-daemon/config.json
"
    );
}

fn print_usage() {
    eprintln!("Usage: indicator-alert-daemon --config <config.json>");
    eprintln!("Try 'indicator-alert-daemon --help' for more information.");
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<String, String> {
    let Some(arg) = args.next() else {
        return Err("missing --config <config.json>".to_string());
    };

    if arg == "-h" || arg == "--help" {
        print_help();
        std::process::exit(0);
    }

    if arg != "--config" {
        if arg.starts_with('-') {
            return Err(format!("unknown option '{arg}'"));
        }
        return Err(format!("unexpected argument '{arg}'; use --config <path>"));
    }

    let Some(path) = args.next() else {
        return Err("--config requires a path argument".to_string());
    };

    if path.starts_with('-') {
        return Err(format!("invalid config path '{path}'"));
    }

    if args.next().is_some() {
        return Err("unexpected extra arguments".to_string());
    }

    Ok(path)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config_path = match parse_args(std::env::args().skip(1)) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("error: {err}\n");
            print_usage();
            std::process::exit(1);
        }
    };

    indicator_alert_daemon::run_loop(&config_path).await;
}
