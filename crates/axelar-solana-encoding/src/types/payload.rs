//! # Payload Module
//!
//! This module defines the `Payload` enum, which encapsulates the different
//! types of payloads that can be processed within the system. The `Payload`
//! enum supports two variants: handling collections of messages and managing
//! verifier set updates. This abstraction allows for flexible and secure
//! processing of diverse data types within the system's workflow.

use super::messages::Messages;
use super::verifier_set::VerifierSet;

/// Represents the different types of payloads that can be processed within the
/// system.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Payload {
    /// Encapsulates a collection of messages to be processed.
    Messages(Messages),

    /// Represents an updated verifier set for system consensus.
    NewVerifierSet(VerifierSet),
}
