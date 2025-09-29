//! Parse Solana events from transaction data

use axelar_solana_gas_service_events::events::GasServiceEvent;
use base64::{engine::general_purpose, Engine};

/// Represents the state of a program invocation along with associated events.
#[derive(Debug, PartialEq, Eq)]
pub enum ProgramInvocationState<T> {
    /// The program invocation is currently in progress, holding a list of events and their indexes.
    InProgress(Vec<(usize, T)>),
    /// The program invocation has succeeded, holding a list of events and their indexes.
    Succeeded(Vec<(usize, T)>),
    /// The program invocation has failed, holding a list of events and their indexes.
    Failed(Vec<(usize, T)>),
}

#[allow(clippy::struct_field_names)]
/// Context for matching specific log prefixes in program invocations.
pub struct MatchContext {
    /// The log prefix indicating the start of the target program.
    expected_start: String,
    /// The log prefix indicating the successful completion of the target program.
    expected_success: String,
    /// The log prefix indicating the failure of the target program.
    expected_failure: String,
}

impl MatchContext {
    /// Creates a new `MatchContext` for the given program ID.
    ///
    /// # Arguments
    ///
    /// * `program_id` - The ID of the program to match in the logs.
    #[must_use]
    pub fn new(program_id: &str) -> Self {
        Self {
            expected_start: format!("Program {program_id} invoke"),
            expected_success: format!("Program {program_id} success"),
            expected_failure: format!("Program {program_id} failed"),
        }
    }
}

/// Builds a stack of program invocation states from logs by parsing events.
///
/// # Arguments
///
/// * `ctx` - The `MatchContext` containing expected log prefixes.
/// * `logs` - A slice of logs to parse.
/// * `transformer` - A function that transforms a log entry into an event and updates the program stack.
///
/// # Returns
///
/// A vector of `ProgramInvocationState` representing parsed program invocations and their events.
///
/// # Type Parameters
///
/// * `T` - The type of log entries, convertible to a string slice.
/// * `K` - The type of events.
/// * `Err` - The error type returned by the transformer function.
/// * `F` - The transformer function that will transform logs into even structures
pub fn build_program_event_stack<T, K, Err, F>(
    ctx: &MatchContext,
    logs: &[T],
    transformer: F,
) -> Vec<ProgramInvocationState<K>>
where
    T: AsRef<str>,
    F: Fn(&T) -> Result<K, Err>,
{
    let logs = logs.iter().enumerate();
    let mut program_stack: Vec<ProgramInvocationState<K>> = Vec::new();

    for (idx, log) in logs {
        tracing::trace!(log = ?log.as_ref(), "incoming log from Solana");
        if log.as_ref().starts_with(ctx.expected_start.as_str()) {
            // Start a new program invocation
            program_stack.push(ProgramInvocationState::InProgress(Vec::new()));
        } else if log.as_ref().starts_with(ctx.expected_success.as_str()) {
            handle_success_log(&mut program_stack);
        } else if log.as_ref().starts_with(ctx.expected_failure.as_str()) {
            handle_failure_log(&mut program_stack);
        } else {
            // Process logs if inside a program invocation
            let Some(&mut ProgramInvocationState::InProgress(ref mut events)) =
                program_stack.last_mut()
            else {
                continue;
            };
            #[allow(clippy::let_underscore_must_use, clippy::let_underscore_untyped)]
            let Ok(event) = transformer(log) else {
                continue;
            };
            events.push((idx, event));
        }
    }
    program_stack
}

#[inline]
/// Decodes a Base64-encoded string into bytes.
#[must_use]
pub fn decode_base64(input: &str) -> Option<Vec<u8>> {
    general_purpose::STANDARD.decode(input).ok()
}

/// Parses gas service logs and extracts events.
///
/// # Arguments
///
/// * `log` - The log entry to parse.
///
/// # Errors
///
/// - if the discriminant for the event is not present
/// - if the event was detected via the discriminant but the data does not match the discriminant type
pub fn parse_gas_service_log<T>(log: &T) -> Result<GasServiceEvent, event_utils::EventParseError>
where
    T: AsRef<str>,
{
    use axelar_solana_gas_service_events::event_prefixes::*;
    use axelar_solana_gas_service_events::events::{
        NativeGasAddedEvent, NativeGasPaidForContractCallEvent, NativeGasRefundedEvent,
        SplGasAddedEvent, SplGasPaidForContractCallEvent, SplGasRefundedEvent,
    };
    use event_utils::EventParseError;

    let mut logs = log
        .as_ref()
        .trim()
        .trim_start_matches("Program data: ")
        .split(' ')
        .filter_map(decode_base64);
    let disc = logs
        .next()
        .ok_or(EventParseError::MissingData("discriminant"))?;
    let disc = disc.as_slice();
    let gas_service_event = match disc {
        NATIVE_GAS_PAID_FOR_CONTRACT_CALL => {
            let event = NativeGasPaidForContractCallEvent::new(logs)?;
            GasServiceEvent::NativeGasPaidForContractCall(event)
        }
        NATIVE_GAS_ADDED => {
            let event = NativeGasAddedEvent::new(logs)?;
            GasServiceEvent::NativeGasAdded(event)
        }
        NATIVE_GAS_REFUNDED => {
            let event = NativeGasRefundedEvent::new(logs)?;
            GasServiceEvent::NativeGasRefunded(event)
        }
        SPL_PAID_FOR_CONTRACT_CALL => {
            let event = SplGasPaidForContractCallEvent::new(logs)?;
            GasServiceEvent::SplGasPaidForContractCall(event)
        }
        SPL_GAS_ADDED => {
            let event = SplGasAddedEvent::new(logs)?;
            GasServiceEvent::SplGasAdded(event)
        }
        SPL_GAS_REFUNDED => {
            let event = SplGasRefundedEvent::new(logs)?;
            GasServiceEvent::SplGasRefunded(event)
        }
        _ => {
            return Err(EventParseError::Other("unsupported discriminant"));
        }
    };

    Ok(gas_service_event)
}

/// Handles a failure log by marking the current program invocation as failed.
fn handle_failure_log<K>(program_stack: &mut Vec<ProgramInvocationState<K>>) {
    let Some(state) = program_stack.pop() else {
        tracing::warn!("Program failure without matching invocation");
        return;
    };

    match state {
        ProgramInvocationState::InProgress(events) => {
            program_stack.push(ProgramInvocationState::Failed(events));
        }
        ProgramInvocationState::Succeeded(_) | ProgramInvocationState::Failed(_) => {
            tracing::warn!("Unexpected state when marking program failure");
        }
    }
}

/// Handles a success log by marking the current program invocation as succeeded.
fn handle_success_log<K>(program_stack: &mut Vec<ProgramInvocationState<K>>) {
    let Some(state) = program_stack.pop() else {
        tracing::warn!("Program success without matching invocation");
        return;
    };
    match state {
        ProgramInvocationState::InProgress(events) => {
            program_stack.push(ProgramInvocationState::Succeeded(events));
        }
        ProgramInvocationState::Succeeded(_) | ProgramInvocationState::Failed(_) => {
            tracing::warn!("Unexpected state when marking program success");
        }
    }
}

#[cfg(test)]
mod tests {
    use core::str::FromStr;

    use axelar_solana_gas_service_events::events::NativeGasPaidForContractCallEvent;
    use pretty_assertions::assert_eq;
    use solana_sdk::pubkey::Pubkey;
    use test_log::test;

    use super::*;

    #[test]
    fn test_gas_service_fixture() {
        let logs = [
            "Program gasHQkvaC4jTD2MQpAuEN3RdNwde2Ym5E5QNDoh6m6G invoke [1]",
            "Program 11111111111111111111111111111111 invoke [2]",
            "Program 11111111111111111111111111111111 success",
            "Program data: bmF0aXZlIGdhcyBwYWlkIGZvciBjb250cmFjdCBjYWxs uHuSGR4VBBCRNPjze8Y91JXLTJnrh8qv2IxFZAjnrfI= ZXZt MHhkZWFkYmVlZg== /Qd2xw7aQmd/4PP+LMP3Kwouwb8mAfoKYiWkSoTQv5E= AAAAAAAAAAMAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA= iBMAAAAAAAA=",
            "Program gasHQkvaC4jTD2MQpAuEN3RdNwde2Ym5E5QNDoh6m6G consumed 7199 of 400000 compute units",
            "Program gasHQkvaC4jTD2MQpAuEN3RdNwde2Ym5E5QNDoh6m6G success",
            "Program mem7LhKWbKydCPk1TwNzeCvVSpoVx2mqxNuvjGgWAbG invoke [1]",
            "Program log: Instruction: Native",
            "Program log: Instruction: SendToGateway",
            "Program gtwLjHAsfKAR6GWB4hzTUAA1w4SDdFMKamtGA5ttMEe invoke [2]",
            "Program log: Instruction: Call Contract",
            "Program data: Y2FsbCBjb250cmFjdF9fXw== 7JQPdUfAeRg1X1Nr6GECnQ3fp0Mj2A6smBFZZwEbwhI= /Qd2xw7aQmd/4PP+LMP3Kwouwb8mAfoKYiWkSoTQv5E= ZXZt MHhkZWFkYmVlZg== bXNnIG1lbW8gYW5kIGdhcw==",
            "Program gtwLjHAsfKAR6GWB4hzTUAA1w4SDdFMKamtGA5ttMEe consumed 4799 of 386578 compute units",
            "Program gtwLjHAsfKAR6GWB4hzTUAA1w4SDdFMKamtGA5ttMEe success",
            "Program mem7LhKWbKydCPk1TwNzeCvVSpoVx2mqxNuvjGgWAbG consumed 11145 of 392801 compute units",
            "Program mem7LhKWbKydCPk1TwNzeCvVSpoVx2mqxNuvjGgWAbG success",
        ];
        let match_context = MatchContext::new("gasHQkvaC4jTD2MQpAuEN3RdNwde2Ym5E5QNDoh6m6G");
        let result = build_program_event_stack(&match_context, &logs, parse_gas_service_log);

        let event = NativeGasPaidForContractCallEvent {
            config_pda: "DR9Ja5ojPLPDWmWFRmpc2SEUvK94dKX4uM6AofgwAAJm"
                .parse()
                .unwrap(),
            destination_chain: "evm".to_owned(),
            destination_address: "0xdeadbeef".to_owned(),
            payload_hash: [
                253, 7, 118, 199, 14, 218, 66, 103, 127, 224, 243, 254, 44, 195, 247, 43, 10, 46,
                193, 191, 38, 1, 250, 10, 98, 37, 164, 74, 132, 208, 191, 145,
            ],
            refund_address: "11111112D1oxKts8YPdTJRG5FzxTNpMtWmq8hkVx3".parse().unwrap(),
            gas_fee_amount: 5000,
        };
        let expected = vec![ProgramInvocationState::Succeeded(vec![(
            3,
            GasServiceEvent::NativeGasPaidForContractCall(event),
        )])];

        assert_eq!(result, expected);
    }
}
