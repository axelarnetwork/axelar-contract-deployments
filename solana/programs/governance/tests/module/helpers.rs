use alloy_primitives::Uint;
use axelar_rkyv_encoding::types::{CrossChainId, GmpMetadata};
use ethers::abi::{Function, ParamType, Token};
use governance::state::GovernanceConfig;
use governance_gmp::{GovernanceCommand, GovernanceCommandPayload};
use solana_program_test::{processor, ProgramTest};
use solana_sdk::program_error::ProgramError;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use test_fixtures::test_setup::TestFixture;

use crate::fixtures::{
    SOURCE_CHAIN_ADDRESS, SOURCE_CHAIN_ADDRESS_KECCAK_HASH, SOURCE_CHAIN_NAME,
    SOURCE_CHAIN_NAME_KECCAK_HASH,
};

pub(crate) fn program_test() -> ProgramTest {
    ProgramTest::new(
        "governance",
        governance::id(),
        processor!(governance::processor::Processor::process_instruction),
    )
}

pub(crate) async fn init_contract(fixture: &mut TestFixture) -> Result<(Pubkey, u8), ProgramError> {
    let (config_pda, bump) = GovernanceConfig::pda();

    let config = governance::state::GovernanceConfig::new(
        bump,
        SOURCE_CHAIN_NAME_KECCAK_HASH,
        SOURCE_CHAIN_ADDRESS_KECCAK_HASH,
    );
    let ix =
        governance::instructions::initialize_config(&fixture.payer.pubkey(), &config, &config_pda)
            .unwrap();
    fixture.send_tx(&[ix]).await;
    Ok((config_pda, bump))
}

#[allow(deprecated)]
pub(crate) fn sample_call_data() -> Vec<u8> {
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
        Token::Uint(100_u64.into()), // Value
    ];

    // Encode function call
    function.encode_input(&args).unwrap()
}

pub(crate) fn evm_governance_payload_command(command: GovernanceCommand) -> GovernanceCommandPayload {
    GovernanceCommandPayload {
        command,
        // 5GjBHaKUWnF87NFWLGK5jNzyosMA43PDE6drq3btfqSs
        target: [
            142, 58, 218, 11, 201, 166, 92, 115, 55, 67, 99, 101, 88, 152, 241, 122, 209, 4, 234,
            152, 34, 211, 123, 232, 217, 84, 231, 43, 45, 203, 10, 54,
        ]
        .into(),
        call_data: sample_call_data().into(),
        native_value: Uint::from(1_u32),
        eta: Uint::from(1_726_755_731),
    }
}

pub(crate) fn default_gmp_metadata() -> GmpMetadata {
    GmpMetadata {
        cross_chain_id: CrossChainId::new(SOURCE_CHAIN_NAME.to_string(), "09af".to_string()),
        source_address: SOURCE_CHAIN_ADDRESS.to_string(),
        destination_address: "B3gam8xC15TDne4XtAVAvDDfqJFeSH6mv6sn6TanVJju".to_string(),
        destination_chain: "solana".to_string(),
    }
}
