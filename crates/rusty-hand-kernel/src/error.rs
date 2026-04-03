//! Kernel-specific error types.

use rusty_hand_types::error::RustyHandError;
use thiserror::Error;

/// Kernel error type wrapping RustyHandError with kernel-specific context.
#[derive(Error, Debug)]
pub enum KernelError {
    /// A wrapped RustyHandError.
    #[error(transparent)]
    RustyHand(#[from] RustyHandError),

    /// The kernel failed to boot.
    #[error("Boot failed: {0}")]
    BootFailed(String),
}

/// Alias for kernel results.
pub type KernelResult<T> = Result<T, KernelError>;
