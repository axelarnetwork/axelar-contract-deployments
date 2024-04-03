//! Module for the Gateway program account structs.

mod approved_command;
pub mod config;
pub mod execute_data;

pub use approved_command::{
    GatewayApprovedCommand, GatewayCommandStatus, TransferOperatorship, ValidateContractCall,
};
pub use config::GatewayConfig;
pub use execute_data::GatewayExecuteData;
