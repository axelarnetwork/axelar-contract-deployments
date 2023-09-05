use super::*;

#[event]
pub struct OperatorshipTransferredEvent {
    pub new_operators_data: Vec<u8>,
}

fn transfer(_params: Vec<u8>) -> Result<()> {
    Ok(())
}
