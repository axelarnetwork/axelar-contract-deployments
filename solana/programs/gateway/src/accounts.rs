//! Module for the Gateway program account structs.

pub mod approved_message;
pub mod config;
pub mod discriminator;
pub mod execute_data;
pub mod transfer_operatorship;

pub use approved_message::GatewayApprovedMessage;
pub use config::GatewayConfig;
pub use execute_data::GatewayExecuteData;
