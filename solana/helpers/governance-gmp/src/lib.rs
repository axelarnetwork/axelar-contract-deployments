pub use alloy_primitives;
use alloy_sol_types::sol;

sol! {

    /// The available governance commands See https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/AxelarServiceGovernance.sol#L14-L19
    #[derive(Debug, PartialEq)]
    enum GovernanceCommand {
        ScheduleTimeLockProposal,
        CancelTimeLockProposal,
        ApproveOperatorProposal,
        CancelOperatorApproval
    }

    /// This is a representation of the proposal types. Currently, all commands
    /// have the same structure. See https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/AxelarServiceGovernance.sol#L112-L117
    #[derive(Debug, PartialEq)]
    #[repr(C)]
    struct GovernanceCommandPayload {
        /// The type of the command
        GovernanceCommand command;
        /// The target address the proposal will call.
        bytes target;
        /// The data the encodes the function and arguments to call on the target contract.
        bytes call_data;
        /// The value of native token to be sent to the target contract.
        uint256 native_value;
        /// The time after which the proposal can be executed.
        uint256 eta;
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::Uint;
    use alloy_sol_types::SolValue;
    use ethers::abi::{Function, ParamType, Token};

    use super::*;

    // 5GjBHaKUWnF87NFWLGK5jNzyosMA43PDE6drq3btfqSs
    const TARGET_ADDR: [u8; 32] = [
        142, 58, 218, 11, 201, 166, 92, 115, 55, 67, 99, 101, 88, 152, 241, 122, 209, 4, 234, 152,
        34, 211, 123, 232, 217, 84, 231, 43, 45, 203, 10, 54,
    ];
    const NATIVE_VALUE: u32 = 1;
    const ETA: u64 = 1726755731;

    #[test]
    fn encode_schedule_time_lock_proposal_command() {
        let command = GovernanceCommandPayload {
            command: GovernanceCommand::ScheduleTimeLockProposal,
            target: TARGET_ADDR.into(),
            call_data: sample_call_data().into(),
            native_value: Uint::from(NATIVE_VALUE),
            eta: Uint::from(ETA),
        };

        let expected = "0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000066ec339300000000000000000000000000000000000000000000000000000000000000208e3ada0bc9a65c73374363655898f17ad104ea9822d37be8d954e72b2dcb0a360000000000000000000000000000000000000000000000000000000000000044a9059cbb0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000006400000000000000000000000000000000000000000000000000000000";
        assert_eq!(expected, hex::encode(command.abi_encode()))
    }

    #[allow(deprecated)]
    fn sample_call_data() -> Vec<u8> {
        let function = Function {
            name: "transfer".into(),
            inputs: vec![
                ethers::abi::Param {
                    name: "to".into(),
                    kind: ParamType::Address,
                    internal_type: None,
                },
                ethers::abi::Param {
                    name: "value".into(),
                    kind: ParamType::Uint(256),
                    internal_type: None,
                },
            ],
            outputs: vec![],
            constant: None,
            state_mutability: ethers::abi::StateMutability::NonPayable,
        };

        // Function arguments
        let args = vec![
            Token::Address(
                "0x0000000000000000000000000000000000000000"
                    .parse()
                    .unwrap(),
            ), // Address
            Token::Uint(100u64.into()), // Value
        ];

        // Encode function call
        function.encode_input(&args).unwrap()
    }

    #[test]
    fn encode_cancel_time_lock_proposal_command() {
        let command = GovernanceCommandPayload {
            command: GovernanceCommand::CancelTimeLockProposal,
            target: TARGET_ADDR.into(),
            call_data: sample_call_data().into(),
            native_value: Uint::from(NATIVE_VALUE),
            eta: Uint::from(ETA),
        };

        let expected = "0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000066ec339300000000000000000000000000000000000000000000000000000000000000208e3ada0bc9a65c73374363655898f17ad104ea9822d37be8d954e72b2dcb0a360000000000000000000000000000000000000000000000000000000000000044a9059cbb0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000006400000000000000000000000000000000000000000000000000000000";
        assert_eq!(expected, hex::encode(command.abi_encode()))
    }

    #[test]
    fn encode_approve_operator_proposal_command() {
        let command: GovernanceCommandPayload = GovernanceCommandPayload {
            command: GovernanceCommand::ApproveOperatorProposal,
            target: TARGET_ADDR.into(),
            call_data: sample_call_data().into(),
            native_value: Uint::from(NATIVE_VALUE),
            eta: Uint::from(ETA),
        };

        let expected = "0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000066ec339300000000000000000000000000000000000000000000000000000000000000208e3ada0bc9a65c73374363655898f17ad104ea9822d37be8d954e72b2dcb0a360000000000000000000000000000000000000000000000000000000000000044a9059cbb0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000006400000000000000000000000000000000000000000000000000000000";
        assert_eq!(expected, hex::encode(command.abi_encode()))
    }

    #[test]
    fn encode_cancel_operator_approval_command() {
        let command: GovernanceCommandPayload = GovernanceCommandPayload {
            command: GovernanceCommand::CancelOperatorApproval,
            target: TARGET_ADDR.into(),
            call_data: sample_call_data().into(),
            native_value: Uint::from(NATIVE_VALUE),
            eta: Uint::from(ETA),
        };

        let expected = "0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000066ec339300000000000000000000000000000000000000000000000000000000000000208e3ada0bc9a65c73374363655898f17ad104ea9822d37be8d954e72b2dcb0a360000000000000000000000000000000000000000000000000000000000000044a9059cbb0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000006400000000000000000000000000000000000000000000000000000000";
        assert_eq!(expected, hex::encode(command.abi_encode()))
    }
}
