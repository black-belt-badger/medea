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
    for flag in ["--status", "--limit", "--symbols", "--direction", "--out"] {
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
