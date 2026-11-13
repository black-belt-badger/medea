use std::process::Command;

fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_medea"))
        .args(args)
        // Never use a developer's real credentials in offline tests.
        .env_remove("APCA_API_KEY_ID")
        .env_remove("APCA_API_SECRET_KEY")
        .output()
        .expect("Medea CLI should launch")
}

#[test]
fn quotes_are_listed_in_crypto_help() {
    let result = cli(&["crypto", "--help"]);
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("quotes"));
}

#[test]
fn quotes_help_exposes_all_documented_filters_without_timeframe() {
    let result = cli(&["crypto", "quotes", "--help"]);
    assert!(result.status.success());
    let help = String::from_utf8_lossy(&result.stdout);
    for flag in [
        "--symbols",
        "--location",
        "--start",
        "--end",
        "--limit",
        "--sort",
        "--max-pages",
        "--page-token",
        "--format",
        "--out",
    ] {
        assert!(help.contains(flag), "Missing {flag}: {help}");
    }
    assert!(!help.contains("--timeframe"));
    for location in ["us", "us-1", "us-2", "eu-1", "bs-1"] {
        assert!(help.contains(location), "Missing {location}: {help}");
    }
}

#[test]
fn valid_request_parameters_reach_credential_validation() {
    let result = cli(&[
        "crypto",
        "quotes",
        "--symbols",
        "BTC/USD,ETH/USD",
        "--location",
        "eu-1",
        "--start",
        "2026-10-02T00:00:00.123456789Z",
        "--end",
        "2026-10-02T03:00:00+02:00",
        "--limit",
        "10000",
        "--sort",
        "desc",
        "--max-pages",
        "20",
        "--page-token",
        "ABC+/==",
        "--format",
        "jsonl",
    ]);
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("Missing environment variable: APCA_API_KEY_ID")
    );
}

#[test]
fn missing_credentials_never_fall_through_to_network() {
    let result = cli(&["crypto", "quotes", "--symbols", "SOL/USD"]);
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("Missing environment variable: APCA_API_KEY_ID")
    );
}

#[test]
fn malformed_quote_arguments_fail_before_loading_credentials() {
    for args in [
        vec!["crypto", "quotes"],
        vec!["crypto", "quotes", "--symbols", "BTCUSD"],
        vec!["crypto", "quotes", "--symbols", "BTC/USD,"],
        vec!["crypto", "quotes", "--symbols", "BTC/USD", "--limit", "0"],
        vec![
            "crypto",
            "quotes",
            "--symbols",
            "BTC/USD",
            "--limit",
            "10001",
        ],
        vec![
            "crypto",
            "quotes",
            "--symbols",
            "BTC/USD",
            "--max-pages",
            "0",
        ],
        vec![
            "crypto",
            "quotes",
            "--symbols",
            "BTC/USD",
            "--max-pages",
            "21",
        ],
        vec![
            "crypto",
            "quotes",
            "--symbols",
            "BTC/USD",
            "--sort",
            "sideways",
        ],
        vec![
            "crypto",
            "quotes",
            "--symbols",
            "BTC/USD",
            "--format",
            "csv",
        ],
        vec![
            "crypto",
            "quotes",
            "--symbols",
            "BTC/USD",
            "--location",
            "moon",
        ],
        vec![
            "crypto",
            "quotes",
            "--symbols",
            "BTC/USD",
            "--timeframe",
            "1Min",
        ],
        vec![
            "crypto",
            "quotes",
            "--symbols",
            "BTC/USD",
            "--page-token",
            "has spaces",
        ],
        vec!["crypto", "quotes", "--symbols", "BTC/USD", "--out"],
        vec![
            "crypto",
            "quotes",
            "--symbols",
            "BTC/USD",
            "--start",
            "today",
        ],
        vec![
            "crypto",
            "quotes",
            "--symbols",
            "BTC/USD",
            "--end",
            "2026-12-01T12:00:00",
        ],
    ] {
        let result = cli(&args);
        assert!(!result.status.success(), "{args:?}");
        assert!(result.stdout.is_empty(), "{args:?}");
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            !stderr.contains("Missing environment variable"),
            "{args:?}: {stderr}"
        );
    }
}

#[test]
fn reversed_timestamp_bounds_are_rejected_before_credentials() {
    let result = cli(&[
        "crypto",
        "quotes",
        "--symbols",
        "BTC/USD",
        "--start",
        "2026-10-03T00:00:00Z",
        "--end",
        "2026-10-02T00:00:00Z",
    ]);
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("--start must not be later than --end"));
    assert!(!stderr.contains("Missing environment variable"));
}
