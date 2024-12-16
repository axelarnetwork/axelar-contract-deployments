//! Parse Solana events from transaction data

use axelar_solana_gas_service::processor::GasServiceEvent;
use axelar_solana_gateway::processor::GatewayEvent;
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

/// Parses gateway logs and extracts events.
///
/// # Arguments
///
/// * `log` - The log entry to parse.
///
/// # Errors
///
/// - if the discrimintant for the event is not present
/// - if the event was detected via the discriminant but the data does not match the discriminant type
pub fn parse_gateway_logs<T>(
    log: &T,
) -> Result<GatewayEvent, axelar_solana_gateway::processor::EventParseError>
where
    T: AsRef<str>,
{
    use axelar_solana_gateway::event_prefixes::*;
    use axelar_solana_gateway::processor::EventParseError;

    let mut logs = log
        .as_ref()
        .trim()
        .trim_start_matches("Program data:")
        .split_whitespace()
        .filter_map(decode_base64);
    let disc = logs
        .next()
        .ok_or(EventParseError::MissingData("discriminant"))?
        .try_into()
        .map_err(|err: Vec<u8>| EventParseError::InvalidLength {
            field: "discriminant",
            expected: 32,
            actual: err.len(),
        })?;
    let gateway_event = match &disc {
        CALL_CONTRACT => {
            let event = axelar_solana_gateway::processor::CallContractEvent::new(logs)?;
            GatewayEvent::CallContract(event)
        }
        CALL_CONTRACT_OFFCHAIN_DATA => {
            let event = axelar_solana_gateway::processor::CallContractOffchainDataEvent::new(logs)?;
            GatewayEvent::CallContractOffchainData(event)
        }
        MESSAGE_APPROVED => {
            let event = axelar_solana_gateway::processor::MessageEvent::new(logs)?;
            GatewayEvent::MessageApproved(event)
        }
        MESSAGE_EXECUTED => {
            let event = axelar_solana_gateway::processor::MessageEvent::new(logs)?;
            GatewayEvent::MessageExecuted(event)
        }
        OPERATORSHIP_TRANSFERRED => {
            let event = axelar_solana_gateway::processor::OperatorshipTransferredEvent::new(logs)?;
            GatewayEvent::OperatorshipTransferred(event)
        }
        SIGNERS_ROTATED => {
            let event = axelar_solana_gateway::processor::VerifierSetRotated::new(logs)?;
            GatewayEvent::VerifierSetRotated(event)
        }
        _ => return Err(EventParseError::Other("unsupported discrimintant")),
    };

    Ok(gateway_event)
}

/// Parses gas service logs and extracts events.
///
/// # Arguments
///
/// * `log` - The log entry to parse.
///
/// # Errors
///
/// - if the discrimintant for the event is not present
/// - if the event was detected via the discriminant but the data does not match the discriminant type
pub fn parse_gas_service_log<T>(
    log: &T,
) -> Result<GasServiceEvent, axelar_solana_gas_service::event_utils::EventParseError>
where
    T: AsRef<str>,
{
    use axelar_solana_gas_service::event_prefixes::*;
    use axelar_solana_gas_service::event_utils::EventParseError;
    use axelar_solana_gas_service::processor::{
        NativeGasAddedEvent, NativeGasPaidForContractCallEvent, NativeGasRefundedEvent,
    };

    let mut logs = log
        .as_ref()
        .trim()
        .trim_start_matches("Program data:")
        .split_whitespace()
        .filter_map(decode_base64);
    let disc = logs
        .next()
        .ok_or(EventParseError::MissingData("discriminant"))?;
    let disc = disc.as_slice();
    let gas_service_event = match disc {
        NATIVE_GAS_PAID_FOR_CONTRACT_CALL => {
            let event = NativeGasPaidForContractCallEvent::new(logs)?;
            GasServiceEvent::NativeGasPaidForncontractCall(event)
        }
        NATIVE_GAS_ADDED => {
            let event = NativeGasAddedEvent::new(logs)?;
            GasServiceEvent::NativeGasAdded(event)
        }
        NATIVE_GAS_REFUNDED => {
            let event = NativeGasRefundedEvent::new(logs)?;
            GasServiceEvent::NativeGasRefunded(event)
        }
        _ => return Err(EventParseError::Other("unsupported discrimintant")),
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

    use axelar_solana_gateway::processor::CallContractEvent;
    use pretty_assertions::assert_eq;
    use solana_sdk::pubkey::Pubkey;
    use test_log::test;

    use super::*;

    static GATEWAY_EXAMPLE_ID: &str = "gtwEpzTprUX7TJLx1hFXNeqCXJMsoxYQhQaEbnuDcj1";

    // Include the test_call_data function
    fn fixture_call_data() -> (&'static str, GatewayEvent) {
        // this is a `CallContract` extract form other unittests
        let base64_data = "Y2FsbCBjb250cmFjdF9fXw== 6NGe5cm7PkXHz/g8V2VdRg0nU0l7R48x8lll4s0Clz0= xtlu5J3pLn7c4BhqnNSrP1wDZK/pQOJVCYbk6sroJhY= ZXRoZXJldW0= MHgwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDA2YzIwNjAzYzdiODc2NjgyYzEyMTczYmRlZjlhMWRjYTUyOGYxNGZk 8J+QqvCfkKrwn5Cq8J+Qqg==";
        // Simple `CallContract` fixture
        let event = GatewayEvent::CallContract(CallContractEvent {
            sender_key: Pubkey::from_str("GfpyaXoJrd9XHHRehAPCGETie3wpM8xDxscAUoC12Cxt").unwrap(),
            destination_chain: "ethereum".to_owned(),
            destination_contract_address:
                "0x0000000000000000000000006c20603c7b876682c12173bdef9a1dca528f14fd".to_owned(),
            payload: vec![
                240, 159, 144, 170, 240, 159, 144, 170, 240, 159, 144, 170, 240, 159, 144, 170,
            ],
            payload_hash: [
                198, 217, 110, 228, 157, 233, 46, 126, 220, 224, 24, 106, 156, 212, 171, 63, 92, 3,
                100, 175, 233, 64, 226, 85, 9, 134, 228, 234, 202, 232, 38, 22,
            ],
        });
        (base64_data, event)
    }

    fn fixture_match_context() -> MatchContext {
        MatchContext::new(GATEWAY_EXAMPLE_ID)
    }

    #[test]
    fn test_simple_event() {
        // Use the test_call_data fixture
        let (base64_data, event) = fixture_call_data();

        // Sample logs with multiple gateway calls, some succeed and some fail
        let logs = vec![
            format!("Program {GATEWAY_EXAMPLE_ID} invoke [1]"), // Invocation 1 starts
            "Program log: Instruction: Call Contract".to_owned(),
            format!("Program data: {}", base64_data),
            format!("Program {GATEWAY_EXAMPLE_ID} success"), // Invocation 1 succeeds
        ];

        let result = build_program_event_stack(&fixture_match_context(), &logs, parse_gateway_logs);

        // Expected result: two successful invocations with their events, one failed invocation
        let expected = vec![ProgramInvocationState::Succeeded(vec![(2, event)])];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_multiple_gateway_calls_some_succeed_some_fail() {
        // Use the test_call_data fixture
        let (base64_data, event) = fixture_call_data();

        // Sample logs with multiple gateway calls, some succeed and some fail
        let logs = vec![
            format!("Program {GATEWAY_EXAMPLE_ID} invoke [1]"), // Invocation 1 starts
            "Program log: Instruction: Call Contract".to_owned(),
            format!("Program data: {}", base64_data),
            format!("Program {GATEWAY_EXAMPLE_ID} success"), // Invocation 1 succeeds
            format!("Program {GATEWAY_EXAMPLE_ID} invoke [2]"), // Invocation 2 starts
            "Program log: Instruction: Call Contract".to_owned(),
            format!("Program data: {}", base64_data),
            format!("Program {GATEWAY_EXAMPLE_ID} failed"), // Invocation 2 fails
            format!("Program {GATEWAY_EXAMPLE_ID} invoke [3]"), // Invocation 3 starts
            "Program log: Instruction: Call Contract".to_owned(),
            format!("Program data: {}", base64_data),
            format!("Program {GATEWAY_EXAMPLE_ID} success"), // Invocation 3 succeeds
        ];

        let result = build_program_event_stack(&fixture_match_context(), &logs, parse_gateway_logs);

        // Expected result: two successful invocations with their events, one failed invocation
        let expected = vec![
            ProgramInvocationState::Succeeded(vec![(2, event.clone())]),
            ProgramInvocationState::Failed(vec![(6, event.clone())]),
            ProgramInvocationState::Succeeded(vec![(10, event)]),
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_no_gateway_calls() {
        // Logs with no gateway calls
        let logs = vec![
            "Program some_other_program invoke [1]".to_owned(),
            "Program log: Instruction: Do something".to_owned(),
            "Program some_other_program success".to_owned(),
        ];

        let result = build_program_event_stack(&fixture_match_context(), &logs, parse_gateway_logs);

        // Expected result: empty stack
        let expected = Vec::new();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_gateway_call_with_no_events() {
        // Gateway call that succeeds but has no events
        let logs = vec![
            format!("Program {GATEWAY_EXAMPLE_ID} invoke [1]"),
            "Program log: Instruction: Do something".to_owned(),
            format!("Program {GATEWAY_EXAMPLE_ID} success"),
        ];

        let result = build_program_event_stack(&fixture_match_context(), &logs, parse_gateway_logs);

        // Expected result: one successful invocation with no events
        let expected = vec![ProgramInvocationState::Succeeded(vec![])];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_gateway_call_failure_with_events() {
        // Use the test_call_data fixture
        let (base64_data, event) = fixture_call_data();

        // Gateway call that fails but has events (events should be discarded)
        let logs = vec![
            format!("Program {GATEWAY_EXAMPLE_ID} invoke [1]"),
            format!("Program data: {}", base64_data),
            format!("Program {GATEWAY_EXAMPLE_ID} failed"),
        ];

        let result = build_program_event_stack(&fixture_match_context(), &logs, parse_gateway_logs);

        // Expected result: one failed invocation
        let expected = vec![ProgramInvocationState::Failed(vec![(1, event)])];

        assert_eq!(result, expected);
    }
}
