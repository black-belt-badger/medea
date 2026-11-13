use std::fmt;
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::blocking::{Client, Request};
use reqwest::header::HeaderValue;
use reqwest::redirect::Policy;

use crate::cli::{AssetClass, AssetStatus, OrderGetArgs, OrdersArgs};
use crate::credentials::Credentials;
use crate::crypto_bars::{BarsInstant, BarsSort, CryptoBarsArgs, CryptoLocation, CryptoSymbols};
use crate::crypto_trades::CryptoTradesArgs;

const PAPER_ACCOUNT_URL: &str = "https://paper-api.alpaca.markets/v2/account";
const PAPER_POSITIONS_URL: &str = "https://paper-api.alpaca.markets/v2/positions";
const PAPER_ASSETS_URL: &str = "https://paper-api.alpaca.markets/v2/assets";
const PAPER_ORDERS_URL: &str = "https://paper-api.alpaca.markets/v2/orders";
const CRYPTO_DATA_ROOT: &str = "https://data.alpaca.markets/v1beta3/crypto/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    InvalidCredentials,
    ConnectionFailed,
    UnexpectedStatus(u16),
    OrderNotFound,
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCredentials => write!(f, "Invalid Alpaca credentials in environment."),
            Self::ConnectionFailed => write!(
                f,
                "Could not connect to Alpaca's paper API. Check your network connection."
            ),
            Self::OrderNotFound => write!(f, "Alpaca paper order not found (HTTP 404)."),
            Self::UnexpectedStatus(401) => {
                write!(f, "Alpaca paper API rejected the credentials (HTTP 401).")
            }
            Self::UnexpectedStatus(status) => {
                write!(f, "Alpaca paper API returned HTTP {status}.")
            }
        }
    }
}

fn authenticated_client() -> Result<Client, ClientError> {
    Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(Policy::none()) // Never forward authentication headers to redirects.
        .build()
        .map_err(|_| ClientError::ConnectionFailed)
}

fn account_request(client: &Client, credentials: &Credentials) -> Result<Request, ClientError> {
    authenticated_request(client, credentials, PAPER_ACCOUNT_URL)
}

fn positions_request(client: &Client, credentials: &Credentials) -> Result<Request, ClientError> {
    authenticated_request(client, credentials, PAPER_POSITIONS_URL)
}

fn assets_request(
    client: &Client,
    credentials: &Credentials,
    class: AssetClass,
    status: Option<AssetStatus>,
) -> Result<Request, ClientError> {
    let mut request = authenticated_request(client, credentials, PAPER_ASSETS_URL)?;
    // The query values are typed allowlists; url::form_urlencoded handles
    // proper percent encoding. Never interpolate user input into the URL.
    let mut query = request.url_mut().query_pairs_mut();
    query.append_pair("asset_class", class.as_query());
    if let Some(status) = status {
        query.append_pair("status", status.as_query());
    }
    drop(query);
    Ok(request)
}

fn orders_request(
    client: &Client,
    credentials: &Credentials,
    options: &OrdersArgs,
) -> Result<Request, ClientError> {
    let mut request = authenticated_request(client, credentials, PAPER_ORDERS_URL)?;
    // Typed/validated filters: URL construction percent-encodes reserved characters.
    let mut query = request.url_mut().query_pairs_mut();
    query.append_pair("status", options.status.as_query());
    if let Some(limit) = options.limit {
        query.append_pair("limit", &limit.to_string());
    }
    if let Some(symbols) = &options.symbols {
        query.append_pair("symbols", &symbols.0.join(","));
    }
    query.append_pair("direction", options.direction.as_query());
    drop(query);
    Ok(request)
}

#[derive(Clone, Copy)]
enum CryptoDataKind {
    Bars,
    Trades,
}

impl CryptoDataKind {
    fn segment(self) -> &'static str {
        match self {
            Self::Bars => "bars",
            Self::Trades => "trades",
        }
    }
}

struct CryptoDataQuery<'a> {
    kind: CryptoDataKind,
    symbols: &'a CryptoSymbols,
    location: CryptoLocation,
    timeframe: Option<&'a str>,
    start: Option<&'a BarsInstant>,
    end: Option<&'a BarsInstant>,
    limit: u16,
    sort: BarsSort,
    page_token: Option<&'a str>,
}

fn crypto_bars_request(
    client: &Client,
    credentials: &Credentials,
    options: &CryptoBarsArgs,
    token: Option<&str>,
) -> Result<Request, ClientError> {
    crypto_data_request(
        client,
        credentials,
        CryptoDataQuery {
            kind: CryptoDataKind::Bars,
            symbols: &options.symbols,
            location: options.location,
            timeframe: Some(options.timeframe.as_str()),
            start: options.start.as_ref(),
            end: options.end.as_ref(),
            limit: options.limit,
            sort: options.sort,
            page_token: token,
        },
    )
}

fn crypto_trades_request(
    client: &Client,
    credentials: &Credentials,
    options: &CryptoTradesArgs,
    token: Option<&str>,
) -> Result<Request, ClientError> {
    crypto_data_request(
        client,
        credentials,
        CryptoDataQuery {
            kind: CryptoDataKind::Trades,
            symbols: &options.symbols,
            location: options.location,
            timeframe: None,
            start: options.start.as_ref(),
            end: options.end.as_ref(),
            limit: options.limit,
            sort: options.sort,
            page_token: token,
        },
    )
}

// Shared authenticated read-only market-data builder: endpoint is an enum,
// locations are typed, and URL query pairs are encoded by reqwest.
fn crypto_data_request(
    client: &Client,
    credentials: &Credentials,
    options: CryptoDataQuery<'_>,
) -> Result<Request, ClientError> {
    let endpoint = format!(
        "{CRYPTO_DATA_ROOT}{}/{}",
        options.location.as_path(),
        options.kind.segment()
    );
    let mut request = authenticated_request(client, credentials, &endpoint)?;
    let mut query = request.url_mut().query_pairs_mut();
    query.append_pair("symbols", &options.symbols.0.join(","));
    if let Some(timeframe) = options.timeframe {
        query.append_pair("timeframe", timeframe);
    }
    if let Some(start) = options.start {
        query.append_pair("start", start.as_str());
    }
    if let Some(end) = options.end {
        query.append_pair("end", end.as_str());
    }
    query.append_pair("limit", &options.limit.to_string());
    query.append_pair("sort", options.sort.as_query());
    if let Some(token) = options.page_token {
        query.append_pair("page_token", token);
    }
    drop(query);
    Ok(request)
}

fn order_request(
    client: &Client,
    credentials: &Credentials,
    options: &OrderGetArgs,
) -> Result<Request, ClientError> {
    let mut request = authenticated_request(client, credentials, PAPER_ORDERS_URL)?;
    // Append an already validated canonical UUID as an encoded path segment.
    request
        .url_mut()
        .path_segments_mut()
        .map_err(|_| ClientError::ConnectionFailed)?
        .push(options.order_id.as_str());
    if options.nested {
        request
            .url_mut()
            .query_pairs_mut()
            .append_pair("nested", "true");
    }
    Ok(request)
}

// One authenticated request builder, with identical safeguards for all paper endpoints.
fn authenticated_request(
    client: &Client,
    credentials: &Credentials,
    url: &str,
) -> Result<Request, ClientError> {
    fn header(value: &str) -> Result<HeaderValue, ClientError> {
        HeaderValue::from_str(value).map_err(|_| ClientError::InvalidCredentials)
    }
    client
        .get(url)
        .header("APCA-API-KEY-ID", header(&credentials.key_id)?)
        .header("APCA-API-SECRET-KEY", header(&credentials.secret_key)?)
        .build()
        .map_err(|_| ClientError::ConnectionFailed)
}

pub fn fetch_crypto_bars_page(
    credentials: &Credentials,
    options: &CryptoBarsArgs,
    token: Option<&str>,
) -> Result<Vec<u8>, ClientError> {
    fetch_authenticated(credentials, |client, credentials| {
        crypto_bars_request(client, credentials, options, token)
    })
}

pub fn fetch_crypto_trades_page(
    credentials: &Credentials,
    options: &CryptoTradesArgs,
    token: Option<&str>,
) -> Result<Vec<u8>, ClientError> {
    fetch_authenticated(credentials, |client, credentials| {
        crypto_trades_request(client, credentials, options, token)
    })
}

pub fn fetch_account(credentials: &Credentials) -> Result<Vec<u8>, ClientError> {
    fetch_authenticated(credentials, account_request)
}

pub fn fetch_positions(credentials: &Credentials) -> Result<Vec<u8>, ClientError> {
    fetch_authenticated(credentials, positions_request)
}

pub fn fetch_assets(
    credentials: &Credentials,
    class: AssetClass,
    status: Option<AssetStatus>,
) -> Result<Vec<u8>, ClientError> {
    fetch_authenticated(credentials, |client, credentials| {
        assets_request(client, credentials, class, status)
    })
}

pub fn fetch_orders(
    credentials: &Credentials,
    options: &OrdersArgs,
) -> Result<Vec<u8>, ClientError> {
    fetch_authenticated(credentials, |client, credentials| {
        orders_request(client, credentials, options)
    })
}

// Single-order HTTP 404 is a specific not-found error, not an auth failure.
fn order_error(err: ClientError) -> ClientError {
    match err {
        ClientError::UnexpectedStatus(404) => ClientError::OrderNotFound,
        other => other,
    }
}

pub fn fetch_order(
    credentials: &Credentials,
    options: &OrderGetArgs,
) -> Result<Vec<u8>, ClientError> {
    fetch_authenticated(credentials, |client, credentials| {
        order_request(client, credentials, options)
    })
    .map_err(order_error)
}

fn fetch_authenticated(
    credentials: &Credentials,
    build_request: impl FnOnce(&Client, &Credentials) -> Result<Request, ClientError>,
) -> Result<Vec<u8>, ClientError> {
    let client = authenticated_client()?;
    let request = build_request(&client, credentials)?;
    let response = client
        .execute(request)
        .map_err(|_| ClientError::ConnectionFailed)?;
    check_status(response.status())?;
    response
        .bytes()
        .map(|bytes| bytes.to_vec())
        .map_err(|_| ClientError::ConnectionFailed)
}

fn check_status(status: StatusCode) -> Result<(), ClientError> {
    if status.is_success() {
        Ok(())
    } else {
        Err(ClientError::UnexpectedStatus(status.as_u16()))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ClientError, account_request, assets_request, authenticated_client, check_status,
        crypto_bars_request, crypto_trades_request, order_error, order_request, orders_request,
        positions_request,
    };
    use crate::cli::{
        AssetClass, AssetStatus, OrderDirection, OrderGetArgs, OrderStatus, OrderSymbols,
        OrdersArgs,
    };
    use crate::credentials::Credentials;
    use crate::crypto_bars::{BarsFormat, BarsSort, CryptoBarsArgs, CryptoLocation};
    use crate::crypto_trades::CryptoTradesArgs;
    use reqwest::{Method, StatusCode};

    fn sample_bars_args() -> CryptoBarsArgs {
        CryptoBarsArgs {
            symbols: "BTC/USD,ETH/USD".parse().unwrap(),
            location: CryptoLocation::Us,
            timeframe: "1Hour".parse().unwrap(),
            start: Some("2026-10-02T00:00:00.123456789Z".parse().unwrap()),
            end: Some("2026-10-02T02:00:00Z".parse().unwrap()),
            limit: 25,
            sort: BarsSort::Asc,
            max_pages: 2,
            page_token: None,
            format: BarsFormat::Json,
            output: None,
        }
    }

    fn sample_trades_args() -> CryptoTradesArgs {
        CryptoTradesArgs {
            symbols: "BTC/USD,ETH/USD".parse().unwrap(),
            location: CryptoLocation::Us,
            start: Some("2026-10-02T00:00:00.123456789Z".parse().unwrap()),
            end: Some("2026-10-02T02:00:00Z".parse().unwrap()),
            limit: 25,
            sort: BarsSort::Asc,
            max_pages: 2,
            page_token: None,
            format: BarsFormat::Json,
            output: None,
        }
    }

    #[test]
    fn account_request_is_paper_only_get_without_api_parameters() {
        let client = authenticated_client().unwrap();
        let credentials = Credentials {
            key_id: "example-key".into(),
            secret_key: "example-secret".into(),
        };
        let request = account_request(&client, &credentials).unwrap();
        assert_eq!(request.method(), Method::GET);
        assert_eq!(request.url().scheme(), "https");
        assert_eq!(request.url().host_str(), Some("paper-api.alpaca.markets"));
        assert_eq!(request.url().path(), "/v2/account");
        assert_eq!(request.url().query(), None);
        assert!(request.body().is_none());
    }

    #[test]
    fn crypto_trade_request_uses_authenticated_market_data_endpoint_without_timeframe() {
        let client = authenticated_client().unwrap();
        let credentials = Credentials {
            key_id: "example-key".into(),
            secret_key: "example-secret".into(),
        };
        let trades =
            crypto_trades_request(&client, &credentials, &sample_trades_args(), None).unwrap();
        let account = account_request(&client, &credentials).unwrap();
        assert_eq!(trades.method(), Method::GET);
        assert_eq!(trades.url().scheme(), "https");
        assert_eq!(trades.url().host_str(), Some("data.alpaca.markets"));
        assert_eq!(trades.url().port_or_known_default(), Some(443));
        assert_eq!(trades.url().path(), "/v1beta3/crypto/us/trades");
        assert_eq!(trades.headers(), account.headers());
        assert_eq!(
            trades.url().query(),
            Some(
                "symbols=BTC%2FUSD%2CETH%2FUSD&start=2026-10-02T00%3A00%3A00.123456789Z&end=2026-10-02T02%3A00%3A00Z&limit=25&sort=asc"
            )
        );
        assert!(!trades.url().query().unwrap().contains("timeframe"));
    }

    #[test]
    fn crypto_trade_request_safely_encodes_token_and_typed_location() {
        let client = authenticated_client().unwrap();
        let credentials = Credentials {
            key_id: "example-key".into(),
            secret_key: "example-secret".into(),
        };
        let mut options = sample_trades_args();
        options.symbols = "SOL/USD".parse().unwrap();
        options.location = CryptoLocation::Eu1;
        options.start = None;
        options.end = None;
        options.sort = BarsSort::Desc;
        options.limit = 10000;
        let trades =
            crypto_trades_request(&client, &credentials, &options, Some("ABC+/==")).unwrap();
        assert_eq!(trades.url().path(), "/v1beta3/crypto/eu-1/trades");
        assert_eq!(
            trades.url().query(),
            Some("symbols=SOL%2FUSD&limit=10000&sort=desc&page_token=ABC%2B%2F%3D%3D")
        );
    }

    #[test]
    fn crypto_bar_request_uses_data_host_and_shared_authenticated_https() {
        let client = authenticated_client().unwrap();
        let credentials = Credentials {
            key_id: "example-key".into(),
            secret_key: "example-secret".into(),
        };
        let options = sample_bars_args();
        let bars = crypto_bars_request(&client, &credentials, &options, None).unwrap();
        let account = account_request(&client, &credentials).unwrap();
        assert_eq!(bars.method(), Method::GET);
        assert_eq!(bars.url().scheme(), "https");
        assert_eq!(bars.url().host_str(), Some("data.alpaca.markets"));
        assert_eq!(bars.url().port_or_known_default(), Some(443));
        assert_eq!(bars.url().path(), "/v1beta3/crypto/us/bars");
        assert_eq!(bars.headers(), account.headers());
        assert_eq!(
            bars.url().query(),
            Some(
                "symbols=BTC%2FUSD%2CETH%2FUSD&timeframe=1Hour&start=2026-10-02T00%3A00%3A00.123456789Z&end=2026-10-02T02%3A00%3A00Z&limit=25&sort=asc"
            )
        );
    }

    #[test]
    fn crypto_bar_request_encodes_tokens_and_allowlisted_location() {
        let client = authenticated_client().unwrap();
        let credentials = Credentials {
            key_id: "example-key".into(),
            secret_key: "example-secret".into(),
        };
        let mut options = sample_bars_args();
        options.symbols = "SOL/USD".parse().unwrap();
        options.timeframe = "1Min".parse().unwrap();
        options.location = CryptoLocation::Eu1;
        options.start = None;
        options.end = None;
        options.limit = 10000;
        options.sort = BarsSort::Desc;
        let bars = crypto_bars_request(&client, &credentials, &options, Some("ABC+/==")).unwrap();
        assert_eq!(bars.url().path(), "/v1beta3/crypto/eu-1/bars");
        assert_eq!(
            bars.url().query(),
            Some(
                "symbols=SOL%2FUSD&timeframe=1Min&limit=10000&sort=desc&page_token=ABC%2B%2F%3D%3D"
            )
        );
    }

    #[test]
    fn paper_account_request_is_https_get_with_headers() {
        let client = authenticated_client().unwrap();
        let creds = Credentials {
            key_id: "example-key".into(),
            secret_key: "example-secret".into(),
        };
        let request = account_request(&client, &creds).unwrap();
        assert_eq!(request.method(), Method::GET);
        assert_eq!(request.url().scheme(), "https");
        assert_eq!(request.url().host_str(), Some("paper-api.alpaca.markets"));
        assert_eq!(request.url().path(), "/v2/account");
        assert_eq!(request.url().port_or_known_default(), Some(443));
        assert_eq!(request.headers()["APCA-API-KEY-ID"], "example-key");
        assert_eq!(request.headers()["APCA-API-SECRET-KEY"], "example-secret");
    }

    #[test]
    fn paper_positions_request_reuses_https_get_and_authentication() {
        let client = authenticated_client().unwrap();
        let creds = Credentials {
            key_id: "example-key".into(),
            secret_key: "example-secret".into(),
        };
        let positions = positions_request(&client, &creds).unwrap();
        let account = account_request(&client, &creds).unwrap();
        assert_eq!(positions.method(), Method::GET);
        assert_eq!(positions.url().scheme(), "https");
        assert_eq!(positions.url().host_str(), Some("paper-api.alpaca.markets"));
        assert_eq!(positions.url().path(), "/v2/positions");
        assert_eq!(positions.url().port_or_known_default(), Some(443));
        assert_eq!(positions.headers(), account.headers());
    }

    #[test]
    fn assets_default_to_explicit_crypto_with_shared_paper_auth() {
        let client = authenticated_client().unwrap();
        let creds = Credentials {
            key_id: "example-key".into(),
            secret_key: "example-secret".into(),
        };
        let assets = assets_request(&client, &creds, AssetClass::Crypto, None).unwrap();
        let account = account_request(&client, &creds).unwrap();
        assert_eq!(assets.method(), Method::GET);
        assert_eq!(assets.url().scheme(), "https");
        assert_eq!(assets.url().host_str(), Some("paper-api.alpaca.markets"));
        assert_eq!(assets.url().path(), "/v2/assets");
        assert_eq!(assets.url().query(), Some("asset_class=crypto"));
        assert_eq!(assets.url().port_or_known_default(), Some(443));
        assert_eq!(assets.headers(), account.headers());
    }

    #[test]
    fn assets_support_class_and_status_query_filters() {
        let client = authenticated_client().unwrap();
        let creds = Credentials {
            key_id: "example-key".into(),
            secret_key: "example-secret".into(),
        };
        let assets = assets_request(
            &client,
            &creds,
            AssetClass::UsEquity,
            Some(AssetStatus::Active),
        )
        .unwrap();
        assert_eq!(
            assets.url().query(),
            Some("asset_class=us_equity&status=active")
        );

        let assets = assets_request(
            &client,
            &creds,
            AssetClass::Crypto,
            Some(AssetStatus::Inactive),
        )
        .unwrap();
        assert_eq!(
            assets.url().query(),
            Some("asset_class=crypto&status=inactive")
        );

        let assets =
            assets_request(&client, &creds, AssetClass::Crypto, Some(AssetStatus::All)).unwrap();
        assert_eq!(assets.url().query(), Some("asset_class=crypto&status=all"));
    }

    #[test]
    fn orders_use_shared_paper_auth_and_safe_defaults() {
        let client = authenticated_client().unwrap();
        let creds = Credentials {
            key_id: "example-key".into(),
            secret_key: "example-secret".into(),
        };
        let options = OrdersArgs {
            status: OrderStatus::Open,
            limit: None,
            symbols: None,
            direction: OrderDirection::Desc,
            output: None,
        };
        let orders = orders_request(&client, &creds, &options).unwrap();
        let account = account_request(&client, &creds).unwrap();
        assert_eq!(orders.method(), Method::GET);
        assert_eq!(orders.url().scheme(), "https");
        assert_eq!(orders.url().host_str(), Some("paper-api.alpaca.markets"));
        assert_eq!(orders.url().path(), "/v2/orders");
        assert_eq!(orders.url().port_or_known_default(), Some(443));
        assert_eq!(orders.url().query(), Some("status=open&direction=desc"));
        assert_eq!(orders.headers(), account.headers());
    }

    #[test]
    fn orders_encode_symbols_and_all_filters() {
        let client = authenticated_client().unwrap();
        let creds = Credentials {
            key_id: "example-key".into(),
            secret_key: "example-secret".into(),
        };
        let options = OrdersArgs {
            status: OrderStatus::All,
            limit: Some(500),
            symbols: Some(OrderSymbols(vec!["BTCUSD".into(), "AAPL".into()])),
            direction: OrderDirection::Asc,
            output: None,
        };
        let orders = orders_request(&client, &creds, &options).unwrap();
        assert_eq!(
            orders.url().query(),
            Some("status=all&limit=500&symbols=BTCUSD%2CAAPL&direction=asc")
        );
    }

    #[test]
    fn order_get_reuses_paper_auth_and_uses_validated_path_segment() {
        let client = authenticated_client().unwrap();
        let credentials = Credentials {
            key_id: "example-key".into(),
            secret_key: "example-secret".into(),
        };
        let uuid = "9275f66a-0a1b-4a22-8c55-167416c7cf14";
        let options = OrderGetArgs {
            order_id: uuid.parse().unwrap(),
            nested: false,
            output: None,
        };
        let request = order_request(&client, &credentials, &options).unwrap();
        let account = account_request(&client, &credentials).unwrap();
        assert_eq!(request.method(), Method::GET);
        assert_eq!(request.url().scheme(), "https");
        assert_eq!(request.url().host_str(), Some("paper-api.alpaca.markets"));
        assert_eq!(request.url().port_or_known_default(), Some(443));
        assert_eq!(
            request.url().path(),
            "/v2/orders/9275f66a-0a1b-4a22-8c55-167416c7cf14"
        );
        assert_eq!(request.url().query(), None);
        assert_eq!(request.headers(), account.headers());
    }

    #[test]
    fn order_get_only_sends_nested_when_explicitly_requested() {
        let client = authenticated_client().unwrap();
        let credentials = Credentials {
            key_id: "example-key".into(),
            secret_key: "example-secret".into(),
        };
        let options = OrderGetArgs {
            order_id: "9275f66a-0a1b-4a22-8c55-167416c7cf14".parse().unwrap(),
            nested: true,
            output: Some("order.json".into()),
        };
        let request = order_request(&client, &credentials, &options).unwrap();
        assert_eq!(request.url().query(), Some("nested=true"));
        assert_eq!(
            request.url().path_segments().unwrap().next_back(),
            Some(options.order_id.as_str())
        );
    }

    #[test]
    fn order_get_distinguishes_not_found_from_other_errors() {
        assert_eq!(
            order_error(ClientError::UnexpectedStatus(404)),
            ClientError::OrderNotFound
        );
        assert_eq!(
            ClientError::OrderNotFound.to_string(),
            "Alpaca paper order not found (HTTP 404)."
        );
        assert_eq!(
            order_error(ClientError::UnexpectedStatus(401)),
            ClientError::UnexpectedStatus(401)
        );
        assert_eq!(
            order_error(ClientError::ConnectionFailed),
            ClientError::ConnectionFailed
        );
        assert_eq!(
            check_status(StatusCode::NOT_FOUND),
            Err(ClientError::UnexpectedStatus(404))
        );
        assert!(!ClientError::OrderNotFound.to_string().contains("secret"));
    }

    #[test]
    fn response_error_does_not_echo_body_or_secret() {
        assert_eq!(
            ClientError::UnexpectedStatus(401).to_string(),
            "Alpaca paper API rejected the credentials (HTTP 401)."
        );
        assert_eq!(
            ClientError::UnexpectedStatus(503).to_string(),
            "Alpaca paper API returned HTTP 503."
        );
        assert!(!ClientError::ConnectionFailed.to_string().contains("secret"));
    }

    #[test]
    fn rejects_redirects_and_http_errors_without_response_bodies() {
        assert_eq!(check_status(StatusCode::OK), Ok(()));
        assert_eq!(check_status(StatusCode::CREATED), Ok(()));
        assert_eq!(
            check_status(StatusCode::FOUND),
            Err(ClientError::UnexpectedStatus(302))
        );
        assert_eq!(
            check_status(StatusCode::UNAUTHORIZED),
            Err(ClientError::UnexpectedStatus(401))
        );
        assert_eq!(
            check_status(StatusCode::SERVICE_UNAVAILABLE),
            Err(ClientError::UnexpectedStatus(503))
        );
    }

    #[test]
    fn rejects_invalid_header_value_without_echoing_it() {
        let client = authenticated_client().unwrap();
        let creds = Credentials {
            key_id: "example-key".into(),
            secret_key: "secret\nvalue".into(),
        };
        assert!(matches!(
            account_request(&client, &creds),
            Err(ClientError::InvalidCredentials)
        ));
        assert!(
            !ClientError::InvalidCredentials
                .to_string()
                .contains("secret")
        );
    }
}
