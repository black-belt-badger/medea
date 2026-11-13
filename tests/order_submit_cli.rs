use std::process::{Command, Output};

use serde_json::{Value, json};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_medea"))
        .args(args)
        // Tests must never inherit a real key and accidentally submit an order.
        .env_remove("APCA_API_KEY_ID")
        .env_remove("APCA_API_SECRET_KEY")
        .output()
        .expect("Medea CLI should launch")
}

const MARKET: &[&str] = &[
    "order",
    "submit",
    "--symbol",
    "BTC/USD",
    "--side",
    "buy",
    "--type",
    "market",
    "--qty",
    "0.000100001",
    "--client-order-id",
    "experiment-01",
];

#[test]
fn submit_help_exposes_important_safety_flags() {
    let out = cli(&["order", "submit", "--help"]);
    assert!(out.status.success());
    let help = String::from_utf8_lossy(&out.stdout);
    for flag in [
        "--symbol",
        "--side",
        "--type",
        "--qty",
        "--notional",
        "--limit-price",
        "--time-in-force",
        "--client-order-id",
        "--confirm",
        "--out",
    ] {
        assert!(help.contains(flag), "Missing {flag}");
    }
    assert!(help.contains("dry run"));
    assert!(!help.contains("--live"));
}

#[test]
fn dry_run_succeeds_without_credentials_and_prints_exact_request() {
    let out = cli(MARKET);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("DRY RUN"));
    assert!(stdout.contains("no credentials loaded"));
    assert!(stdout.contains("no order submitted"));
    assert!(stdout.contains("use --confirm to submit"));
    let json: Value = serde_json::from_str(stdout.lines().last().unwrap()).unwrap();
    assert_eq!(
        json,
        json!({
            "symbol": "BTC/USD",
            "side": "buy",
            "type": "market",
            "time_in_force": "gtc",
            "qty": "0.000100001",
            "client_order_id": "experiment-01"
        })
    );
}

#[test]
fn confirmed_order_fails_at_credentials_without_sending_any_network_request() {
    let mut args = MARKET.to_vec();
    args.push("--confirm");
    let out = cli(&args);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .contains("Missing environment variable: APCA_API_KEY_ID")
    );
}

#[test]
fn dry_run_output_file_is_rejected_before_credentials() {
    let mut args = MARKET.to_vec();
    args.extend(["--out", "/tmp/this-file-must-not-exist.json"]);
    let out = cli(&args);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("--out requires --confirm"), "{err}");
    assert!(!err.contains("Missing environment variable"));
}

#[test]
fn semantic_errors_fail_before_credentials_without_sending_a_post() {
    for (additional, expected) in [
        (
            vec!["--notional", "25"],
            "--qty and --notional cannot be combined",
        ),
        (
            vec!["--limit-price", "70000"],
            "--limit-price is only allowed for limit orders",
        ),
    ] {
        let mut args = MARKET.to_vec();
        args.extend(additional);
        let out = cli(&args);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains(expected), "{stderr}");
        assert!(!stderr.contains("Missing environment variable"));
    }

    let limit_no_price = [
        "order",
        "submit",
        "--symbol",
        "BTC/USD",
        "--side",
        "buy",
        "--type",
        "limit",
        "--qty",
        "1",
        "--client-order-id",
        "experiment-01",
    ];
    let out = cli(&limit_no_price);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("Limit orders require --limit-price"));

    let too_precise = [
        "order",
        "submit",
        "--symbol",
        "BTC/USD",
        "--side",
        "buy",
        "--type",
        "market",
        "--notional",
        "25.123",
        "--client-order-id",
        "experiment-01",
    ];
    let out = cli(&too_precise);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .contains("USD-quoted crypto notional supports at most 2 decimal places")
    );
}

#[test]
fn invalid_lexical_flags_are_rejected_by_clap_before_credentials() {
    let invalid_cases = [
        vec!["--symbol", "../orders"],
        vec!["--side", "short"],
        vec!["--type", "stop"],
        vec!["--time-in-force", "day"],
        vec!["--qty", "-1"],
        vec!["--notional", "NaN"],
        vec!["--limit-price", "bad"],
        vec!["--client-order-id", "client/order"],
        vec!["--live"],
    ];
    for invalid in invalid_cases {
        let mut args = MARKET.to_vec();
        args.extend(invalid);
        let out = cli(&args);
        assert!(!out.status.success(), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        assert!(
            !String::from_utf8_lossy(&out.stderr).contains("Missing environment variable"),
            "{args:?}"
        );
    }
}

#[test]
fn preview_limit_and_notional_variants_without_credentials() {
    for options in [
        vec![
            "order",
            "submit",
            "--symbol",
            "ETH/USD",
            "--side",
            "sell",
            "--type",
            "market",
            "--time-in-force",
            "ioc",
            "--notional",
            "25.12",
            "--client-order-id",
            "experiment-eth-1",
        ],
        vec![
            "order",
            "submit",
            "--symbol",
            "SOL/USD",
            "--side",
            "buy",
            "--type",
            "limit",
            "--time-in-force",
            "gtc",
            "--qty",
            "2.000000001",
            "--limit-price",
            "100.123456789",
            "--client-order-id",
            "experiment-sol-1",
        ],
    ] {
        let out = cli(&options);
        assert!(
            out.status.success(),
            "{options:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.stderr.is_empty());
        let stdout = String::from_utf8(out.stdout).unwrap();
        let value: Value = serde_json::from_str(stdout.lines().last().unwrap()).unwrap();
        assert!(value["symbol"].is_string());
        assert!(value["client_order_id"].is_string());
        assert!(value.get("notional").is_some() || value.get("limit_price").is_some());
        assert!(!stdout.contains("APCA-API-SECRET-KEY"));
    }
}
