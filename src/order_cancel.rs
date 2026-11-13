//! Dry-run-first cancellation of exactly one Alpaca PAPER order.
//!
//! No credentials are read and no HTTP request is constructed during a
//! default invocation. The opt-in path never retries an uncertain DELETE.

use crate::cli::OrderCancelArgs;
use crate::client::fetch_cancel_order;
use crate::credentials::load_credentials;

/// A success response means the cancellation *request* was accepted, not that
/// the order reached canceled state (it may still fill meanwhile).
pub fn render_cancel_accepted(options: &OrderCancelArgs) -> String {
    format!(
        "Alpaca accepted the paper order cancellation request (HTTP 204).\n\
         Cancellation is not yet confirmed. Check order status with:\n\
           medea order get {}\n",
        options.order_id.as_str()
    )
}

pub fn render_cancel_preview(options: &OrderCancelArgs) -> String {
    format!(
        "DRY RUN — no cancellation was sent.\n\
         Paper order: {}\n\
         To request cancellation, repeat with --confirm.\n",
        options.order_id.as_str()
    )
}

/// The callback is not invoked in dry run, and is invoked exactly once when
/// the user confirms. This makes the network boundary easy to test offline.
pub fn run_cancel_with<E>(
    options: &OrderCancelArgs,
    send: impl FnOnce(&OrderCancelArgs) -> Result<(), E>,
) -> Result<String, E> {
    if !options.confirmed {
        return Ok(render_cancel_preview(options));
    }
    send(options)?;
    Ok(render_cancel_accepted(options))
}

pub fn run_cancel(options: OrderCancelArgs) -> Result<(), String> {
    let message = run_cancel_with(&options, |confirmed| {
        let credentials = load_credentials().map_err(|err| err.to_string())?;
        fetch_cancel_order(&credentials, confirmed).map_err(|err| err.to_string())
    })?;
    print!("{message}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{render_cancel_accepted, render_cancel_preview, run_cancel_with};
    use crate::cli::OrderCancelArgs;
    use crate::client::ClientError;
    use std::cell::Cell;

    fn args(confirmed: bool) -> OrderCancelArgs {
        OrderCancelArgs {
            order_id: "9275f66a-0a1b-4a22-8c55-167416c7cf14".parse().unwrap(),
            confirmed,
        }
    }

    #[test]
    fn preview_does_not_invoke_callback_or_load_credentials() {
        let invoked = Cell::new(0);
        let result = run_cancel_with(&args(false), |_| {
            invoked.set(invoked.get() + 1);
            Err(ClientError::ConnectionFailed)
        })
        .unwrap();
        assert_eq!(invoked.get(), 0);
        assert_eq!(result, render_cancel_preview(&args(false)));
        assert!(result.starts_with("DRY RUN"));
        assert!(result.contains("no cancellation was sent"));
        assert!(result.contains("--confirm"));
    }

    #[test]
    fn confirmation_invokes_exactly_one_callback_and_never_claims_completion() {
        let invoked = Cell::new(0);
        let result = run_cancel_with(&args(true), |_| {
            invoked.set(invoked.get() + 1);
            Ok::<(), ClientError>(())
        })
        .unwrap();
        assert_eq!(invoked.get(), 1);
        assert_eq!(result, render_cancel_accepted(&args(true)));
        assert!(result.contains("HTTP 204"));
        assert!(result.contains("not yet confirmed"));
        assert!(result.contains("medea order get"));
        assert!(!result.contains("status=canceled"));
    }

    #[test]
    fn failed_or_uncertain_request_is_never_retried() {
        let invoked = Cell::new(0);
        let result = run_cancel_with(&args(true), |_| {
            invoked.set(invoked.get() + 1);
            Err(ClientError::ConnectionFailed)
        });
        assert_eq!(invoked.get(), 1);
        assert_eq!(result, Err(ClientError::ConnectionFailed));
    }
}
