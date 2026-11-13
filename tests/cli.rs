use std::process::Command;

fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_medea"))
        .args(args)
        // Never read a developer's real paper credentials in offline tests.
        .env_remove("APCA_API_KEY_ID")
        .env_remove("APCA_API_SECRET_KEY")
        .output()
        .expect("Medea binary should be runnable")
}

#[test]
fn help_lists_account_command() {
    let result = cli(&["--help"]);
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("account"));
}

#[test]
fn help_lists_positions_command() {
    let result = cli(&["--help"]);
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("positions"));
}

#[test]
fn positions_help_lists_output_option() {
    let result = cli(&["positions", "--help"]);
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("--out"));
}

#[test]
fn missing_positions_credentials_produce_clear_error() {
    let result = cli(&["positions"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("Missing environment variable: APCA_API_KEY_ID"));
    assert!(result.stdout.is_empty());
}

#[test]
fn missing_positions_output_path_is_invalid_cli() {
    let result = cli(&["positions", "--out"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("--out"));
}

#[test]
fn positions_reject_unsupported_filter_flags_before_credentials() {
    for args in [
        vec!["positions", "--symbol", "BTCUSD"],
        vec!["positions", "--asset-class", "crypto"],
        vec!["positions", "--limit", "5"],
    ] {
        let result = cli(&args);
        assert!(!result.status.success(), "{args:?}");
        assert!(result.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(!stderr.contains("Missing environment variable"));
    }
}

#[test]
fn help_lists_assets_command() {
    let result = cli(&["--help"]);
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("assets"));
}

#[test]
fn assets_help_lists_filters_and_output() {
    let result = cli(&["assets", "--help"]);
    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("--asset-class"));
    assert!(stdout.contains("--status"));
    assert!(stdout.contains("--exchange"));
    assert!(stdout.contains("--attributes"));
    assert!(stdout.contains("--out"));
    assert!(stdout.contains("us_equity"));
}

#[test]
fn missing_assets_credentials_are_reported_without_any_response() {
    let result = cli(&["assets"]);
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("Missing environment variable: APCA_API_KEY_ID")
    );
    assert!(result.stdout.is_empty());
}

#[test]
fn invalid_assets_filters_and_missing_option_values_fail_cli() {
    for args in [
        vec!["assets", "--asset-class", "other"],
        vec!["assets", "--status", "other"],
        vec!["assets", "--out"],
        vec!["assets", "--status"],
        vec!["assets", "--asset-class"],
    ] {
        let result = cli(&args);
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
    }
}

#[test]
fn asset_exchange_mismatch_fails_before_loading_credentials() {
    for args in [
        vec!["assets", "--exchange", "NASDAQ"],
        vec![
            "assets",
            "--asset-class",
            "us_equity",
            "--exchange",
            "INDEX",
        ],
        vec!["assets", "--asset-class", "us_index", "--exchange", "NYSE"],
        vec![
            "assets",
            "--asset-class",
            "crypto_perp",
            "--exchange",
            "NASDAQ",
        ],
    ] {
        let output = cli(&args);
        assert!(!output.status.success(), "{args:?}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("exchange") || stderr.contains("Exchange") || stderr.contains("INDEX"),
            "{stderr}"
        );
        assert!(!stderr.contains("Missing environment variable"));
    }
}

#[test]
fn invalid_assets_csv_is_rejected_by_clap_without_credentials_or_stdout() {
    for args in [
        vec!["assets", "--exchange", "NASDAQ,"],
        vec!["assets", "--exchange", "CRYPTO"],
        vec!["assets", "--attributes", "has_options,has_options"],
        vec!["assets", "--attributes", "NOT_AN_ATTRIBUTE"],
    ] {
        let output = cli(&args);
        assert!(!output.status.success(), "{args:?}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("Missing environment variable"));
    }
}

#[test]
fn help_lists_orders_command() {
    let result = cli(&["--help"]);
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("orders"));
}

#[test]
fn orders_help_lists_filters_and_output() {
    let result = cli(&["orders", "--help"]);
    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
    for flag in [
        "--status",
        "--limit",
        "--symbols",
        "--direction",
        "--out",
        "--after",
        "--until",
        "--nested",
        "--side",
        "--asset-class",
        "--before-order-id",
        "--after-order-id",
    ] {
        assert!(stdout.contains(flag), "Missing {flag}");
    }
}

#[test]
fn missing_orders_credentials_are_reported_without_any_response() {
    let result = cli(&["orders"]);
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("Missing environment variable: APCA_API_KEY_ID")
    );
    assert!(result.stdout.is_empty());
}

#[test]
fn invalid_orders_filters_and_missing_values_fail_cli() {
    for args in [
        vec!["orders", "--status", "filled"],
        vec!["orders", "--direction", "up"],
        vec!["orders", "--limit", "0"],
        vec!["orders", "--limit", "501"],
        vec!["orders", "--symbols", "BTC/USD"],
        vec!["orders", "--symbols", "BTCUSD,,AAPL"],
        vec!["orders", "--out"],
        vec!["orders", "--limit"],
        vec!["orders", "--status"],
        vec!["orders", "--symbols"],
        vec!["orders", "--direction"],
    ] {
        let result = cli(&args);
        assert!(!result.status.success(), "{args:?}");
        assert!(result.stdout.is_empty(), "{args:?}");
    }
}

#[test]
fn conflicting_order_cursors_and_dates_fail_before_credentials() {
    let uuid = "9275f66a-0a1b-4a22-8c55-167416c7cf14";
    for args in [
        vec![
            "orders",
            "--before-order-id",
            uuid,
            "--after-order-id",
            uuid,
        ],
        vec![
            "orders",
            "--before-order-id",
            uuid,
            "--after",
            "2026-11-12T01:00:00Z",
        ],
        vec![
            "orders",
            "--before-order-id",
            uuid,
            "--until",
            "2026-11-12T01:00:00Z",
        ],
        vec![
            "orders",
            "--after-order-id",
            uuid,
            "--after",
            "2026-11-12T01:00:00Z",
        ],
        vec![
            "orders",
            "--after-order-id",
            uuid,
            "--until",
            "2026-11-12T01:00:00Z",
        ],
        vec![
            "orders",
            "--after",
            "2026-11-12T02:00:00Z",
            "--until",
            "2026-11-12T01:00:00Z",
        ],
    ] {
        let output = cli(&args);
        assert!(!output.status.success(), "{args:?}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("Missing environment variable"), "{stderr}");
        assert!(
            stderr.contains("combined")
                || stderr.contains("mutually exclusive")
                || stderr.contains("later"),
            "{stderr}"
        );
    }
}

#[test]
fn malformed_new_orders_parameters_fail_in_cli_before_credentials() {
    for args in [
        vec!["orders", "--asset-class", "all,crypto"],
        vec!["orders", "--asset-class", "crypto,crypto"],
        vec!["orders", "--asset-class", "CRYPTO"],
        vec!["orders", "--side", "BUY"],
        vec!["orders", "--after", "2026-11-12"],
        vec!["orders", "--until", "2026-11-12T01:00:00"],
        vec!["orders", "--before-order-id", "not-a-uuid"],
        vec!["orders", "--after-order-id", "../positions"],
        vec!["orders", "--asset-class"],
        vec!["orders", "--before-order-id"],
    ] {
        let output = cli(&args);
        assert!(!output.status.success(), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("Missing environment variable"),
            "{args:?}"
        );
    }
}

#[test]
fn help_lists_order_get_command() {
    let top = cli(&["--help"]);
    assert!(top.status.success());
    assert!(String::from_utf8_lossy(&top.stdout).contains("order"));
    let sub = cli(&["order", "--help"]);
    assert!(sub.status.success());
    assert!(String::from_utf8_lossy(&sub.stdout).contains("get"));
}

#[test]
fn order_get_help_lists_uuid_nested_and_json_output() {
    let result = cli(&["order", "get", "--help"]);
    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
    for item in ["ORDER_ID", "--nested", "--out"] {
        assert!(stdout.contains(item), "Missing {item}");
    }
}

#[test]
fn missing_order_get_credentials_are_reported_without_leaking_data() {
    let result = cli(&["order", "get", "9275f66a-0a1b-4a22-8c55-167416c7cf14"]);
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("Missing environment variable: APCA_API_KEY_ID")
    );
    assert!(result.stdout.is_empty());
}

#[test]
fn invalid_order_uuid_is_rejected_before_trying_credentials() {
    for args in [
        vec!["order", "get"],
        vec!["order", "get", "not-a-uuid"],
        vec![
            "order",
            "get",
            "9275f66a-0a1b-4a22-8c55-167416c7cf14?nested=true",
        ],
        vec![
            "order",
            "get",
            "9275f66a-0a1b-4a22-8c55-167416c7cf14",
            "--out",
        ],
        vec![
            "order",
            "get",
            "9275f66a-0a1b-4a22-8c55-167416c7cf14",
            "--unknown",
        ],
    ] {
        let result = cli(&args);
        assert!(!result.status.success(), "{args:?}");
        assert!(result.stdout.is_empty(), "{args:?}");
        assert!(!String::from_utf8_lossy(&result.stderr).contains("APCA_API_KEY_ID"));
    }
}

#[test]
fn crypto_help_lists_historical_bars() {
    let top = cli(&["--help"]);
    assert!(top.status.success());
    assert!(String::from_utf8_lossy(&top.stdout).contains("crypto"));
    let sub = cli(&["crypto", "--help"]);
    assert!(sub.status.success());
    assert!(String::from_utf8_lossy(&sub.stdout).contains("bars"));
}

#[test]
fn crypto_bars_help_lists_filters_paging_and_formats() {
    let result = cli(&["crypto", "bars", "--help"]);
    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
    for flag in [
        "--symbols",
        "--location",
        "--timeframe",
        "--start",
        "--end",
        "--limit",
        "--sort",
        "--max-pages",
        "--page-token",
        "--format",
        "--out",
    ] {
        assert!(stdout.contains(flag), "Missing {flag}");
    }
}

#[test]
fn missing_crypto_credentials_reported_without_secret_or_response() {
    let result = cli(&[
        "crypto",
        "bars",
        "--symbols",
        "BTC/USD",
        "--timeframe",
        "1Min",
    ]);
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("Missing environment variable: APCA_API_KEY_ID")
    );
    assert!(result.stdout.is_empty());
}

#[test]
fn invalid_crypto_arguments_fail_before_loading_credentials() {
    for args in [
        vec!["crypto", "bars"],
        vec!["crypto", "bars", "--symbols", "BTC/USD"],
        vec!["crypto", "bars", "--timeframe", "1Min"],
        vec![
            "crypto",
            "bars",
            "--symbols",
            "BTCUSD",
            "--timeframe",
            "1Min",
        ],
        vec![
            "crypto",
            "bars",
            "--symbols",
            "BTC/USD",
            "--timeframe",
            "0Min",
        ],
        vec![
            "crypto",
            "bars",
            "--symbols",
            "BTC/USD",
            "--timeframe",
            "1Min",
            "--limit",
            "0",
        ],
        vec![
            "crypto",
            "bars",
            "--symbols",
            "BTC/USD",
            "--timeframe",
            "1Min",
            "--limit",
            "10001",
        ],
        vec![
            "crypto",
            "bars",
            "--symbols",
            "BTC/USD",
            "--timeframe",
            "1Min",
            "--max-pages",
            "21",
        ],
        vec![
            "crypto",
            "bars",
            "--symbols",
            "BTC/USD",
            "--timeframe",
            "1Min",
            "--location",
            "../us",
        ],
        vec![
            "crypto",
            "bars",
            "--symbols",
            "BTC/USD",
            "--timeframe",
            "1Min",
            "--start",
            "yesterday",
        ],
        vec![
            "crypto",
            "bars",
            "--symbols",
            "BTC/USD",
            "--timeframe",
            "1Min",
            "--format",
            "csv",
        ],
        vec![
            "crypto",
            "bars",
            "--symbols",
            "BTC/USD",
            "--timeframe",
            "1Min",
            "--sort",
            "random",
        ],
        vec![
            "crypto",
            "bars",
            "--symbols",
            "BTC/USD",
            "--timeframe",
            "1Min",
            "--page-token",
            "",
        ],
        vec![
            "crypto",
            "bars",
            "--symbols",
            "BTC/USD",
            "--timeframe",
            "1Min",
            "--out",
        ],
        vec![
            "crypto",
            "bars",
            "--symbols",
            "BTC/USD",
            "--timeframe",
            "1Min",
            "--unknown",
        ],
    ] {
        let result = cli(&args);
        assert!(!result.status.success(), "{args:?}");
        assert!(result.stdout.is_empty(), "{args:?}");
        assert!(
            !String::from_utf8_lossy(&result.stderr).contains("APCA_API_KEY_ID"),
            "{args:?}"
        );
    }
}

#[test]
fn reverse_crypto_time_range_fails_before_loading_credentials() {
    let result = cli(&[
        "crypto",
        "bars",
        "--symbols",
        "BTC/USD",
        "--timeframe",
        "1Min",
        "--start",
        "2026-10-04",
        "--end",
        "2026-10-02",
    ]);
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("--start must not be later than --end")
    );
    assert!(!String::from_utf8_lossy(&result.stderr).contains("APCA_API_KEY_ID"));
    assert!(result.stdout.is_empty());
}

#[test]
fn crypto_trades_help_shows_all_supported_flags_without_timeframe() {
    let parent = cli(&["crypto", "--help"]);
    assert!(parent.status.success());
    assert!(String::from_utf8_lossy(&parent.stdout).contains("trades"));
    let result = cli(&["crypto", "trades", "--help"]);
    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
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
        assert!(stdout.contains(flag), "missing {flag}");
    }
    assert!(!stdout.contains("--timeframe"));
}

#[test]
fn crypto_trades_missing_credentials_reported_without_leaking() {
    let result = cli(&["crypto", "trades", "--symbols", "BTC/USD"]);
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("Missing environment variable: APCA_API_KEY_ID")
    );
    assert!(result.stdout.is_empty());
}

#[test]
fn crypto_trades_invalid_arguments_fail_before_loading_credentials() {
    for args in [
        vec!["crypto", "trades"],
        vec!["crypto", "trades", "--symbols", "BTCUSD"],
        vec![
            "crypto",
            "trades",
            "--symbols",
            "BTC/USD",
            "--timeframe",
            "1Min",
        ],
        vec!["crypto", "trades", "--symbols", "BTC/USD", "--limit", "0"],
        vec![
            "crypto",
            "trades",
            "--symbols",
            "BTC/USD",
            "--limit",
            "10001",
        ],
        vec![
            "crypto",
            "trades",
            "--symbols",
            "BTC/USD",
            "--max-pages",
            "21",
        ],
        vec![
            "crypto",
            "trades",
            "--symbols",
            "BTC/USD",
            "--location",
            "hostile",
        ],
        vec!["crypto", "trades", "--symbols", "BTC/USD,,ETH/USD"],
        vec![
            "crypto",
            "trades",
            "--symbols",
            "BTC/USD",
            "--start",
            "yesterday",
        ],
        vec![
            "crypto",
            "trades",
            "--symbols",
            "BTC/USD",
            "--format",
            "csv",
        ],
        vec![
            "crypto",
            "trades",
            "--symbols",
            "BTC/USD",
            "--page-token",
            "",
        ],
        vec!["crypto", "trades", "--symbols", "BTC/USD", "--out"],
    ] {
        let result = cli(&args);
        assert!(!result.status.success(), "{args:?}");
        assert!(result.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&result.stderr).contains("APCA_API_KEY_ID"));
    }
}

#[test]
fn crypto_trades_reversed_range_is_rejected_before_loading_credentials() {
    let result = cli(&[
        "crypto",
        "trades",
        "--symbols",
        "BTC/USD",
        "--start",
        "2026-10-04",
        "--end",
        "2026-10-02",
    ]);
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("--start must not be later than --end")
    );
    assert!(!String::from_utf8_lossy(&result.stderr).contains("APCA_API_KEY_ID"));
    assert!(result.stdout.is_empty());
}

#[test]
fn account_help_lists_output_option() {
    let result = cli(&["account", "--help"]);
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("--out"));
}

#[test]
fn missing_credentials_produce_clear_error() {
    let result = cli(&["account"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("Missing environment variable: APCA_API_KEY_ID"));
    assert!(result.stdout.is_empty());
}

#[test]
fn missing_output_path_is_invalid_cli() {
    let result = cli(&["account", "--out"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("--out"));
}
