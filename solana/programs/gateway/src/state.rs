//! Module for the Gateway program account structs.

mod approved_command;
pub mod config;
pub mod execute_data;
pub mod verifier_set_tracker;

pub use approved_command::{
    ApprovedMessageStatus, GatewayApprovedCommand, GatewayCommandStatus, RotateSignersStatus,
};
pub use config::GatewayConfig;
pub use execute_data::GatewayExecuteData;
