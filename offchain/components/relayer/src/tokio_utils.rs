//! Utilities for dealing with tokio tasks

use std::{borrow::Cow, error::Error, ops::Deref};

use tokio::task::{AbortHandle, JoinError};
use tracing::error;

/// Logs all information we can get out of a [`JoinError`].
pub(crate) fn log_join_error(join_error: JoinError) {
    let source: Cow<str> = join_error
        .source()
        .map(|error| Cow::Owned(error.to_string()))
        .unwrap_or(Cow::Borrowed("no error source"));
    error!(
        canceled = join_error.is_cancelled(),
        panicked = join_error.is_panic(),
        source = %source.as_ref(),
        "Failed to wait for asynchronous task to finish"
    );
}

/// Wrapper for aborting an [`AbortHandle`] on drop.
pub struct AbortHandleDropGuard(AbortHandle);

impl From<AbortHandle> for AbortHandleDropGuard {
    fn from(handle: AbortHandle) -> Self {
        Self(handle)
    }
}

impl Drop for AbortHandleDropGuard {
    fn drop(&mut self) {
        self.0.abort()
    }
}

impl Deref for AbortHandleDropGuard {
    type Target = AbortHandle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
