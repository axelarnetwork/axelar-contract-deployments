//! Utilities for dealing with tokio tasks

use tokio::task::JoinError;
use tracing::{error, info};

pub(crate) fn handle_join_error(join_error: JoinError) {
    if join_error.is_cancelled() {
        info!("task was cancelled");
    } else if join_error.is_panic() {
        error!("task panicked")
    }
    info!("task stopped");
}
