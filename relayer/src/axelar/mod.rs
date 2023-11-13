use crate::common::types::Message;
use log::{error, info};
use serde_json::{json, Value};
use std::process::Command;

pub mod gateway_verify_messages;
pub mod verifier_is_verified;
