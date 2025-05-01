use std::collections::BTreeMap;
use std::str::FromStr;

use axelar_solana_encoding::hasher::NativeHasher;
use axelar_solana_encoding::types::pubkey::PublicKey;
use axelar_solana_encoding::types::verifier_set::VerifierSet;
use axelar_solana_gateway::state::config::RotationDelaySecs;
use clap::{Parser, Subcommand};
use cosmrs::proto::cosmwasm::wasm::v1::query_client;
use serde::Deserialize;
use serde_json::json;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;

use crate::config::Config;
use crate::types::ChainNameOnAxelar;
use crate::utils::{
    read_json_file_from_path, write_json_to_file_path, ADDRESS_KEY, AXELAR_KEY, CHAINS_KEY,
    CONTRACTS_KEY, DOMAIN_SEPARATOR_KEY, GATEWAY_KEY, GRPC_KEY, MULTISIG_PROVER_KEY,
};

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    #[clap(long_about = "Initialize the Gateway program")]
    Init(InitArgs),

    #[clap(long_about = "Call contract on an Axelar enabled destination chain")]
    CallContract(CallContractArgs),

    #[clap(long_about = "Transfer operatorship of the Gateway program")]
    TransferOperatorship(TransferOperatorshipArgs),
}

#[derive(Parser, Debug)]
pub(crate) struct InitArgs {
    #[clap(short = 'r', long)]
    previous_signers_retention: u128,

    #[clap(short, long)]
    minimum_rotation_delay: RotationDelaySecs,

    #[clap(short, long)]
    axelar_grpc_endpoint: Option<String>,

    #[clap(short = 'p', long)]
    multisig_prover_address: Option<String>,

    #[clap(long)]
    operator: Pubkey,

    #[clap(short, long)]
    domain_separator: Option<String>,
}

#[derive(Parser, Debug)]
pub(crate) struct CallContractArgs {
    #[clap(short = 'd', long)]
    destination_chain: String,

    #[clap(short = 'a', long)]
    destination_contract_address: String,

    #[clap(short, long)]
    payload: String,
}

#[derive(Parser, Debug)]
pub(crate) struct TransferOperatorshipArgs {
    /// Current operator OR upgrade authority
    #[clap(short, long)]
    authority: Pubkey,

    /// Address of the new operator
    #[clap(short, long)]
    new_operator: Pubkey,
}

pub(crate) async fn build_instruction(
    fee_payer: &Pubkey,
    command: Commands,
    config: &Config,
) -> eyre::Result<Instruction> {
    match command {
        Commands::Init(init_args) => init(fee_payer, init_args, config).await,
        Commands::CallContract(call_contract_args) => {
            call_contract(fee_payer, call_contract_args).await
        }
        Commands::TransferOperatorship(transfer_operatorship_args) => {
            transfer_operatorship(fee_payer, transfer_operatorship_args).await
        }
    }
}

async fn query<T: serde::de::DeserializeOwned>(
    mut endpoint: String,
    address: cosmrs::AccountId,
    query_data: Vec<u8>,
) -> eyre::Result<T> {
    if !endpoint.starts_with("https://") {
        endpoint = format!("https://{}", endpoint);
    }
    let mut c = query_client::QueryClient::connect(endpoint).await?;

    let res = c
        .smart_contract_state(
            cosmrs::proto::cosmwasm::wasm::v1::QuerySmartContractStateRequest {
                address: address.to_string(),
                query_data,
            },
        )
        .await?
        .into_inner()
        .data;

    let result = serde_json::from_slice::<T>(res.as_ref())?;

    Ok(result)
}

async fn init(
    fee_payer: &Pubkey,
    init_args: InitArgs,
    config: &Config,
) -> eyre::Result<Instruction> {
    let mut chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;

    let (gateway_config_pda, _bump) = axelar_solana_gateway::get_gateway_root_config_pda();
    let multisig_prover_address = {
        let address = match init_args.multisig_prover_address {
            Some(address) => address,
            None => String::deserialize(
                &chains_info[AXELAR_KEY][CONTRACTS_KEY][MULTISIG_PROVER_KEY]
                    [ChainNameOnAxelar::from(config.network_type).0][ADDRESS_KEY],
            )?,
        };

        cosmrs::AccountId::from_str(&address).unwrap()
    };

    let axelar_grpc_endpoint = {
        match init_args.axelar_grpc_endpoint {
            Some(endpoint) => endpoint,
            None => String::deserialize(&chains_info[AXELAR_KEY][GRPC_KEY])?,
        }
    };

    let multisig_prover_response = query::<multisig_prover::msg::VerifierSetResponse>(
        axelar_grpc_endpoint,
        multisig_prover_address,
        serde_json::to_vec(&multisig_prover::msg::QueryMsg::CurrentVerifierSet {})?,
    )
    .await?;

    let mut signers = BTreeMap::new();
    for signer in multisig_prover_response.verifier_set.signers.values() {
        let pubkey = PublicKey::Secp256k1(signer.pub_key.as_ref().try_into()?);
        let weight = signer.weight.u128();
        signers.insert(pubkey, weight);
    }
    let verifier_set = VerifierSet {
        nonce: multisig_prover_response.verifier_set.created_at,
        signers,
        quorum: multisig_prover_response.verifier_set.threshold.u128(),
    };

    let domain_separator = {
        match init_args.domain_separator {
            Some(domain_separator) => domain_separator,
            None => String::deserialize(
                &chains_info[AXELAR_KEY][CONTRACTS_KEY][MULTISIG_PROVER_KEY]
                    [ChainNameOnAxelar::from(config.network_type).0][DOMAIN_SEPARATOR_KEY],
            )?,
        }
    };

    let mut domain_separator_bytes = [0_u8; 32];
    hex::decode_to_slice(&domain_separator, &mut domain_separator_bytes)?;

    let verifier_set_hash = axelar_solana_encoding::types::verifier_set::verifier_set_hash::<
        NativeHasher,
    >(&verifier_set, &domain_separator_bytes)?;

    let (init_tracker_pda, _bump) =
        axelar_solana_gateway::get_verifier_set_tracker_pda(verifier_set_hash);
    let payer = *fee_payer;
    let upgrade_authority = payer;

    chains_info[CHAINS_KEY][ChainNameOnAxelar::from(config.network_type).0][CONTRACTS_KEY]
        [GATEWAY_KEY] = json!({
        "address": bs58::encode(axelar_solana_gateway::id()).into_string(),
        "deployer": fee_payer,
        "operator": init_args.operator,
        "minimumRotationDelay": init_args.minimum_rotation_delay,
        "previousSignersRetention": init_args.previous_signers_retention,
        "domainSeparator": domain_separator,
    });

    write_json_to_file_path(&chains_info, &config.chains_info_file)?;

    Ok(axelar_solana_gateway::instructions::initialize_config(
        payer,
        upgrade_authority,
        domain_separator_bytes,
        vec![(verifier_set_hash, init_tracker_pda)],
        init_args.minimum_rotation_delay,
        init_args.operator,
        init_args.previous_signers_retention.into(),
        gateway_config_pda,
    )?)
}

async fn call_contract(
    fee_payer: &Pubkey,
    call_contract_args: CallContractArgs,
) -> eyre::Result<Instruction> {
    let (signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(*fee_payer);
    let payload = hex::decode(call_contract_args.payload)?;

    Ok(axelar_solana_gateway::instructions::call_contract(
        axelar_solana_gateway::id(),
        axelar_solana_gateway::get_gateway_root_config_pda().0,
        *fee_payer,
        signing_pda,
        signing_pda_bump,
        call_contract_args.destination_chain,
        call_contract_args.destination_contract_address,
        payload,
    )?)
}

async fn transfer_operatorship(
    fee_payer: &Pubkey,
    transfer_operatorship_args: TransferOperatorshipArgs,
) -> eyre::Result<Instruction> {
    Ok(axelar_solana_gateway::instructions::transfer_operatorship(
        *fee_payer,
        transfer_operatorship_args.authority,
        transfer_operatorship_args.new_operator,
    )?)
}
