//! Fully offline cancellation request/status tests. No network calls here.

use super::{
    ClientError, account_request, authenticated_client, cancel_order_request,
    interpret_cancel_status,
};
use crate::cli::OrderCancelArgs;
use crate::credentials::Credentials;
use reqwest::{Method, StatusCode};

fn sample_args(confirmed: bool) -> OrderCancelArgs {
    OrderCancelArgs {
        order_id: "9275f66a-0a1b-4a22-8c55-167416c7cf14".parse().unwrap(),
        confirmed,
    }
}

fn credentials() -> Credentials {
    Credentials {
        key_id: "example-key".into(),
        secret_key: "example-secret".into(),
    }
}

#[test]
fn unconfirmed_delete_request_cannot_be_built_even_directly() {
    let client = authenticated_client().unwrap();
    let error = cancel_order_request(&client, &credentials(), &sample_args(false)).unwrap_err();
    assert_eq!(error, ClientError::ConfirmationRequired);
    assert!(error.to_string().contains("--confirm"));
}

#[test]
fn confirmed_delete_uses_single_uuid_path_and_paper_only_authenticated_tls() {
    let client = authenticated_client().unwrap();
    let creds = credentials();
    let request = cancel_order_request(&client, &creds, &sample_args(true)).unwrap();
    let account = account_request(&client, &creds).unwrap();

    assert_eq!(request.method(), Method::DELETE);
    assert_eq!(request.url().scheme(), "https");
    assert_eq!(request.url().host_str(), Some("paper-api.alpaca.markets"));
    assert_eq!(request.url().port_or_known_default(), Some(443));
    assert_eq!(
        request.url().path(),
        "/v2/orders/9275f66a-0a1b-4a22-8c55-167416c7cf14"
    );
    assert_eq!(request.url().query(), None);
    assert_eq!(request.headers(), account.headers());
    assert!(request.body().is_none());
}

#[test]
fn uppercase_valid_uuid_is_preserved_as_an_encoded_single_path_segment() {
    let client = authenticated_client().unwrap();
    let creds = credentials();
    let args = OrderCancelArgs {
        order_id: "ABCDEF12-0A1B-4A22-8C55-167416C7CF14".parse().unwrap(),
        confirmed: true,
    };
    let request = cancel_order_request(&client, &creds, &args).unwrap();
    assert_eq!(
        request.url().path(),
        "/v2/orders/ABCDEF12-0A1B-4A22-8C55-167416C7CF14"
    );
    assert!(request.url().query().is_none());
}

#[test]
fn only_http_204_accepts_a_cancellation_request() {
    assert_eq!(interpret_cancel_status(StatusCode::NO_CONTENT), Ok(()));
    for code in [
        200, 201, 202, 206, 301, 302, 400, 401, 403, 409, 429, 500, 503,
    ] {
        assert_eq!(
            interpret_cancel_status(StatusCode::from_u16(code).unwrap()),
            Err(ClientError::UnexpectedStatus(code)),
            "HTTP {code} cannot prove cancellation was accepted"
        );
    }
}

#[test]
fn not_found_and_not_cancelable_are_distinct_errors() {
    assert_eq!(
        interpret_cancel_status(StatusCode::NOT_FOUND),
        Err(ClientError::OrderNotFound)
    );
    assert_eq!(
        interpret_cancel_status(StatusCode::UNPROCESSABLE_ENTITY),
        Err(ClientError::OrderNotCancelable)
    );
    assert_eq!(
        ClientError::OrderNotFound.to_string(),
        "Alpaca paper order not found (HTTP 404)."
    );
    assert_eq!(
        ClientError::OrderNotCancelable.to_string(),
        "Alpaca rejected cancellation: order is not cancelable (HTTP 422)."
    );
}

#[test]
fn errors_never_include_response_bodies_order_ids_or_header_values() {
    for error in [
        ClientError::ConfirmationRequired,
        ClientError::OrderNotFound,
        ClientError::OrderNotCancelable,
        ClientError::InvalidCredentials,
        ClientError::ConnectionFailed,
        ClientError::UnexpectedStatus(401),
        ClientError::UnexpectedStatus(403),
        ClientError::UnexpectedStatus(429),
        ClientError::UnexpectedStatus(503),
    ] {
        let text = error.to_string();
        for secret in ["example-secret", "example-key", "private-body", "9275f66a"] {
            assert!(!text.contains(secret), "{error:?}");
        }
    }
}
