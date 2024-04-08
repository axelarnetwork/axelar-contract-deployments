mod execute;
mod initialize_command;
mod initialize_config;
mod initialize_execute_data;

use axelar_message_primitives::{DataPayload, EncodingScheme};
use cosmwasm_std::Uint256;
use gmp_gateway::state::{GatewayApprovedCommand, GatewayExecuteData};
use multisig::worker_set::WorkerSet;
use solana_program_test::{processor, ProgramTest};
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use test_fixtures::axelar_message::new_worker_set;
use test_fixtures::execute_data::create_signer_with_weight;
use test_fixtures::test_setup::TestFixture;

pub fn program_test() -> ProgramTest {
    ProgramTest::new(
        "gmp_gateway",
        gmp_gateway::id(),
        processor!(gmp_gateway::processor::Processor::process_instruction),
    )
}

pub fn example_payload() -> DataPayload<'static> {
    let payload = DataPayload::new(
        b"payload-from-other-chain",
        &[
            AccountMeta::new_readonly(Pubkey::new_unique(), false),
            AccountMeta::new_readonly(Pubkey::new_unique(), false),
            AccountMeta::new_readonly(Pubkey::new_unique(), false),
        ],
        EncodingScheme::Borsh,
    );
    payload
}

pub fn example_worker_set(new_weight: u128, created_at_block: u64) -> WorkerSet {
    let new_operators = vec![create_signer_with_weight(new_weight).unwrap()];
    new_worker_set(
        &new_operators,
        created_at_block,
        Uint256::from_u128(new_weight),
    )
}

pub fn gateway_approved_command_ixs(
    execute_data: GatewayExecuteData,
    gateway_root_pda: Pubkey,
    fixture: &TestFixture,
) -> Vec<(Pubkey, solana_sdk::instruction::Instruction)> {
    let ixs = execute_data
        .command_batch
        .commands
        .iter()
        .map(|command| {
            let (gateway_approved_message_pda, _bump, _seeds) =
                GatewayApprovedCommand::pda(&gateway_root_pda, command);
            let ix = gmp_gateway::instructions::initialize_pending_command(
                &gateway_root_pda,
                &fixture.payer.pubkey(),
                command.clone(),
            )
            .unwrap();
            (gateway_approved_message_pda, ix)
        })
        .collect::<Vec<_>>();
    ixs
}
