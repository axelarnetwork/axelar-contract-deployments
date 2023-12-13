use std::process::Command;

use crate::common::types::Message;
use log::{error, info};
use serde_json::{json, Value};

pub mod gateway_verify_messages;
pub mod verifier_is_verified;
