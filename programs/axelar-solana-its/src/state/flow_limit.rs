//! Module with data structure definition for handling flow limits on interchain
//! tokens.

use core::time::Duration;

use anchor_discriminators::Discriminator;
use anchor_discriminators_macros::account;
use program_utils::pda::BorshPda;
use solana_program::clock::Clock;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::sysvar::Sysvar;

const EPOCH_TIME: Duration = Duration::from_secs(6 * 60 * 60);

#[account]
#[derive(Debug, Eq, PartialEq, Clone)]
/// Struct containing flow information for a specific epoch.
pub struct FlowState {
    pub flow_limit: Option<u64>,
    pub flow_in: u64,
    pub flow_out: u64,
    pub epoch: u64,
}

/// Module for handling flow limits on interchain tokens.
impl FlowState {
    pub(crate) const fn new(flow_limit: Option<u64>, epoch: u64) -> Self {
        Self {
            flow_in: 0,
            flow_out: 0,
            epoch,
            flow_limit,
        }
    }

    pub(crate) fn add_flow(&mut self, amount: u64, direction: FlowDirection) -> ProgramResult {
        let Some(flow_limit) = self.flow_limit else {
            return Ok(());
        };

        let (to_add, to_compare) = match direction {
            FlowDirection::In => (&mut self.flow_in, self.flow_out),
            FlowDirection::Out => (&mut self.flow_out, self.flow_in),
        };

        Self::update_flow(flow_limit, to_add, to_compare, amount)
    }

    fn update_flow(
        flow_limit: u64,
        to_add: &mut u64,
        to_compare: u64,
        amount: u64,
    ) -> ProgramResult {
        // Individual transfer amount cannot exceed the flow limit
        if amount > flow_limit {
            msg!("Flow limit exceeded");
            return Err(ProgramError::InvalidArgument);
        }

        // Calculate new flow amount after adding the transfer
        let new_flow = to_add
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Calculate net flow: |new_flow - to_compare|
        // The flow limit is interpreted as a limit over the net amount of tokens
        // transferred from one chain to another within a six hours time window.
        let net_flow = if new_flow >= to_compare {
            new_flow - to_compare
        } else {
            to_compare - new_flow
        };

        // Check if net flow exceeds the limit
        if net_flow > flow_limit {
            msg!("Flow limit exceeded");
            return Err(ProgramError::InvalidArgument);
        }

        *to_add = new_flow;

        Ok(())
    }
}

impl BorshPda for FlowState {}

#[derive(Debug, Clone, Copy)]
pub(crate) enum FlowDirection {
    In,
    Out,
}

pub fn current_flow_epoch() -> Result<u64, ProgramError> {
    flow_epoch_with_timestamp(Clock::get()?.unix_timestamp)
}

/// Returns the current flow epoch based on the provided clock.
///
/// # Errors
///
/// Returns an error if conversion from clock to internal flow epoch fails.
pub fn flow_epoch_with_timestamp(timestamp: i64) -> Result<u64, ProgramError> {
    let unix_timestamp: u64 = timestamp
        .try_into()
        .map_err(|_err| ProgramError::ArithmeticOverflow)?;

    unix_timestamp
        .checked_div(EPOCH_TIME.as_secs())
        .ok_or(ProgramError::ArithmeticOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_slot_new_valid() {
        // Test valid creation of FlowSlot
        let expected_flow_in = 0;
        let expected_flow_out = 0;

        let slot = FlowState::new(Some(0), 0);
        assert_eq!(slot.flow_in, expected_flow_in);
        assert_eq!(slot.flow_out, expected_flow_out);
    }

    #[test]
    fn test_add_flow_in_valid() {
        // Test adding flow_in within limits
        let flow_limit = 100;
        let mut slot = FlowState::new(Some(flow_limit), 0);
        slot.add_flow(20, FlowDirection::In).unwrap();
        slot.add_flow(30, FlowDirection::Out).unwrap();
        let amount = 40;

        let result = slot.add_flow(amount, FlowDirection::In);
        assert!(result.is_ok());
        assert_eq!(slot.flow_in, 60);
    }

    #[test]
    fn test_add_flow_in_exceeds_limit() {
        // Test adding flow_in that exceeds limit should fail
        let flow_limit = 100;
        let mut slot = FlowState::new(Some(flow_limit), 0);
        slot.add_flow(80, FlowDirection::In).unwrap();
        let amount = 30; // This would make flow_in 110, exceeding the limit

        let result = slot.add_flow(amount, FlowDirection::In);
        assert_eq!(result, Err(ProgramError::InvalidArgument));
        assert_eq!(slot.flow_in, 80); // Ensure flow_in hasn't changed
    }

    #[test]
    fn test_add_flow_out_valid() {
        // Test adding flow_out within limits
        let flow_limit = 100;
        let mut slot = FlowState::new(Some(flow_limit), 0);
        slot.add_flow(30, FlowDirection::In).unwrap();
        slot.add_flow(20, FlowDirection::Out).unwrap();
        let amount = 50;

        let result = slot.add_flow(amount, FlowDirection::Out);
        assert!(result.is_ok());
        assert_eq!(slot.flow_out, 70);
    }

    #[test]
    fn test_add_flow_out_exceeds_limit() {
        // Test adding flow_out that exceeds limit should fail
        let flow_limit = 100;
        let mut slot = FlowState::new(Some(flow_limit), 0);
        slot.add_flow(90, FlowDirection::Out).unwrap();
        let amount = 20; // This would make flow_out 110, exceeding the limit

        let result = slot.add_flow(amount, FlowDirection::Out);
        assert_eq!(result, Err(ProgramError::InvalidArgument));
        assert_eq!(slot.flow_out, 90); // Ensure flow_out hasn't changed
    }

    #[test]
    fn test_add_flow_in_overflow() {
        // Test arithmetic overflow in add_flow_in
        let flow_limit = u64::MAX;
        let mut slot = FlowState::new(Some(flow_limit), 0);
        slot.add_flow(u64::MAX - 10, FlowDirection::In).unwrap();
        let amount = 20;

        let result = slot.add_flow(amount, FlowDirection::In);
        assert_eq!(result, Err(ProgramError::ArithmeticOverflow));
        assert_eq!(slot.flow_in, u64::MAX - 10); // Ensure flow_in hasn't
                                                 // changed
    }

    #[test]
    fn test_add_flow_out_overflow() {
        // Test arithmetic overflow in add_flow_out
        let flow_limit = u64::MAX;
        let mut slot = FlowState::new(Some(flow_limit), 0);
        slot.add_flow(u64::MAX - 10, FlowDirection::Out).unwrap();
        let amount = 20;

        let result = slot.add_flow(amount, FlowDirection::Out);
        assert_eq!(result, Err(ProgramError::ArithmeticOverflow));
        assert_eq!(slot.flow_out, u64::MAX - 10); // Ensure flow_out hasn't
                                                  // changed
    }

    #[test]
    fn test_add_flow_zero_flow_limit() {
        // Test behavior when flow_limit is zero in add_flow methods
        let flow_limit = 0;
        let mut slot = FlowState::new(Some(flow_limit), 0);

        let result_in = slot.add_flow(1, FlowDirection::In);
        let result_out = slot.add_flow(1, FlowDirection::Out);

        assert!(result_in.is_err());
        assert!(result_out.is_err());
    }

    #[test]
    fn test_add_flow_amount_exceeds_flow_limit() {
        // Test when amount exceeds flow_limit in add_flow methods
        let flow_limit = 50;
        let mut slot = FlowState::new(Some(flow_limit), 0);
        let amount = 60; // Exceeds flow_limit

        let result_in = slot.add_flow(amount, FlowDirection::In);
        let result_out = slot.add_flow(amount, FlowDirection::Out);

        assert_eq!(result_in, Err(ProgramError::InvalidArgument));
        assert_eq!(result_out, Err(ProgramError::InvalidArgument));
        assert_eq!(slot.flow_in, 0);
        assert_eq!(slot.flow_out, 0);
    }

    #[test]
    fn test_add_flow_new_total_exceeds_max_allowed_flow() {
        // Test when new_total exceeds max_allowed_flow in add_flow methods
        let flow_limit = 100;
        let mut slot = FlowState::new(Some(flow_limit), 0);
        slot.add_flow(80, FlowDirection::In).unwrap();
        slot.add_flow(50, FlowDirection::Out).unwrap();
        let amount = 80; // This would make flow_in 160, which exceeds max_allowed_flow (flow_out +
                         // flow_limit = 150)

        let result = slot.add_flow(amount, FlowDirection::In);

        assert_eq!(result, Err(ProgramError::InvalidArgument));
        assert_eq!(slot.flow_in, 80); // Ensure flow_in hasn't changed
    }

    #[test]
    fn test_add_flow_new_total_exceeds_max_allowed_flow_over_multiple_updates() {
        // Test when new_total exceeds max_allowed_flow in add_flow methods
        let flow_limit = 100;
        let mut slot = FlowState::new(Some(flow_limit), 0);
        slot.add_flow(80, FlowDirection::In).unwrap();
        slot.add_flow(50, FlowDirection::Out).unwrap();
        let amount = 20; // This would make flow_in 100, which does not exceed max_allowed_flow (flow_out
                         // + flow_limit = 150)

        let result = slot.add_flow(amount, FlowDirection::In);

        assert!(result.is_ok());

        let amount = 60; // This would make flow_in 160, which exceeds max_allowed_flow (flow_out +
                         // flow_limit = 150)

        let result = slot.add_flow(amount, FlowDirection::In);

        assert_eq!(result, Err(ProgramError::InvalidArgument));
        assert_eq!(slot.flow_in, 100); // Ensure flow_in hasn't changed
    }

    #[test]
    fn test_flow_slot_initialization_with_direction() {
        // Test that FlowSlot initializes correctly based on transfer direction
        let flow_limit = 100;
        let amount = 50;

        // Test incoming transfer initialization
        let mut slot_in = FlowState::new(Some(flow_limit), 0);
        slot_in.add_flow(amount, FlowDirection::In).unwrap();
        assert_eq!(slot_in.flow_in, amount);
        assert_eq!(slot_in.flow_out, 0);

        // Test outgoing transfer initialization
        let mut slot_out = FlowState::new(Some(flow_limit), 0);
        slot_out.add_flow(amount, FlowDirection::Out).unwrap();
        assert_eq!(slot_out.flow_in, 0);
        assert_eq!(slot_out.flow_out, amount);
    }
}
