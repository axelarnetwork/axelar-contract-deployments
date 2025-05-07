use std::collections::BTreeMap;
use std::str::FromStr;

use axelar_solana_encoding::hash_payload;
use axelar_solana_encoding::hasher::NativeHasher;
use axelar_solana_encoding::types::execute_data::{ExecuteData, MerkleisedPayload};
use axelar_solana_encoding::types::messages::{CrossChainId, Message, Messages};
use axelar_solana_encoding::types::payload::Payload;
use axelar_solana_encoding::types::pubkey::{PublicKey, Signature};
use axelar_solana_encoding::types::verifier_set::VerifierSet;
use axelar_solana_gateway::state::config::RotationDelaySecs;
use axelar_solana_gateway::state::incoming_message::command_id;
use clap::{ArgGroup, Parser, Subcommand};
use cosmrs::proto::cosmwasm::wasm::v1::query_client;
use serde::Deserialize;
use serde_json::json;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;

use crate::config::Config;
use crate::types::{ChainNameOnAxelar, SerializeableVerifierSet};
use crate::utils::{
    read_json_file_from_path, write_json_to_file_path, SigningVerifierSet, TestSigner, ADDRESS_KEY,
    AXELAR_KEY, CHAINS_KEY, CONTRACTS_KEY, DOMAIN_SEPARATOR_KEY, GATEWAY_KEY, GRPC_KEY,
    MINIMUM_ROTATION_DELAY_KEY, MULTISIG_PROVER_KEY, OPERATOR_KEY, PREVIOUS_SIGNERS_RETENTION_KEY,
    UPGRADE_AUTHORITY_KEY,
};

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    #[clap(long_about = "Initialize the Gateway program")]
    Init(InitArgs),

    #[clap(long_about = "Call contract on an Axelar enabled destination chain")]
    CallContract(CallContractArgs),

    #[clap(long_about = "Transfer operatorship of the Gateway program")]
    TransferOperatorship(TransferOperatorshipArgs),

    #[clap(long_about = "Approve a message for test deployment")]
    Approve(ApproveArgs),
}

#[derive(Parser, Debug)]
#[clap(group(ArgGroup::new("signers_source").args(&["signer", "signer-set"]).multiple(false).requires("nonce").required(false)))]
pub(crate) struct InitArgs {
    #[clap(short = 'r', long)]
    previous_signers_retention: u128,

    #[clap(short, long)]
    minimum_rotation_delay: RotationDelaySecs,

    /// Hex string with secp256k1 compressed public key
    #[clap(long)]
    signer: Option<String>,

    /// Nonce to be used for the SignerSet, required if `signer` or `signers` is set.
    #[clap(short, long)]
    nonce: u64,

    /// A JSON containing a SignerSet
    #[clap(long)]
    signer_set: Option<String>,

    #[clap(long)]
    operator: Pubkey,

    /// Domain separator associated with the chain.
    /// In case no value is passed, tries to load from MultisigProver entry for the chain. If that fails, defaults to 0x0 if not set (which is fine for
    /// local tests).
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

#[derive(Parser, Debug)]
pub(crate) struct ApproveArgs {
    /// Hex string with secp256k1 private key of the signer used to generate the proof
    #[clap(long)]
    signer: String,

    #[clap(long)]
    source_chain: String,

    #[clap(short, long)]
    message_id: String,

    #[clap(short, long)]
    source_address: String,

    #[clap(short, long)]
    destination_address: String,

    #[clap(short, long)]
    payload: String,
}

pub(crate) async fn build_instruction(
    fee_payer: &Pubkey,
    command: Commands,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    match command {
        Commands::Init(init_args) => init(fee_payer, init_args, config).await,
        Commands::CallContract(call_contract_args) => {
            call_contract(fee_payer, call_contract_args).await
        }
        Commands::TransferOperatorship(transfer_operatorship_args) => {
            transfer_operatorship(fee_payer, transfer_operatorship_args).await
        }
        Commands::Approve(approve_args) => approve(fee_payer, approve_args, config).await,
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

async fn get_verifier_set(
    init_args: &InitArgs,
    config: &Config,
    chains_info: &serde_json::Value,
) -> eyre::Result<VerifierSet> {
    if let Some(signer_key) = &init_args.signer {
        let key_bytes: [u8; 33] = hex::decode(signer_key.strip_prefix("0x").unwrap_or(signer_key))
            .map_err(|_| eyre::eyre!("Failed to decode hex"))?
            .try_into()
            .map_err(|_| eyre::eyre!("Invalid signer pubkey"))?;
        let pk = libsecp256k1::PublicKey::parse_compressed(&key_bytes)?;
        let pubkey = PublicKey::Secp256k1(pk.serialize_compressed());
        let signers = BTreeMap::from([(pubkey, 1_u128)]);

        Ok(VerifierSet {
            nonce: init_args.nonce,
            signers,
            quorum: 1_u128,
        })
    } else if let Some(signer_set) = &init_args.signer_set {
        let signer_set: SerializeableVerifierSet = serde_json::from_str(signer_set)?;

        Ok(signer_set.into())
    } else {
        let multisig_prover_address = {
            let address = String::deserialize(
                &chains_info[AXELAR_KEY][CONTRACTS_KEY][MULTISIG_PROVER_KEY]
                    [ChainNameOnAxelar::from(config.network_type).0][ADDRESS_KEY],
            )?;

            cosmrs::AccountId::from_str(&address).unwrap()
        };

        let axelar_grpc_endpoint = String::deserialize(&chains_info[AXELAR_KEY][GRPC_KEY])?;
        let multisig_prover_response = query::<multisig_prover::msg::VerifierSetResponse>(
            axelar_grpc_endpoint,
            multisig_prover_address,
            serde_json::to_vec(&multisig_prover::msg::QueryMsg::CurrentVerifierSet)?,
        )
        .await?;

        let mut signers = BTreeMap::new();
        for signer in multisig_prover_response.verifier_set.signers.values() {
            let pubkey = PublicKey::Secp256k1(signer.pub_key.as_ref().try_into()?);
            let weight = signer.weight.u128();
            signers.insert(pubkey, weight);
        }

        Ok(VerifierSet {
            nonce: multisig_prover_response.verifier_set.created_at,
            signers,
            quorum: multisig_prover_response.verifier_set.threshold.u128(),
        })
    }
}

async fn init(
    fee_payer: &Pubkey,
    init_args: InitArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let mut chains_info: serde_json::Value =
        read_json_file_from_path(&config.chains_info_file).unwrap_or_default();

    let (gateway_config_pda, _bump) = axelar_solana_gateway::get_gateway_root_config_pda();
    let verifier_set = get_verifier_set(&init_args, config, &chains_info).await?;
    let domain_separator = {
        let maybe_domain_separator = if init_args.domain_separator.is_none() {
            String::deserialize(
                &chains_info[AXELAR_KEY][CONTRACTS_KEY][MULTISIG_PROVER_KEY]
                    [ChainNameOnAxelar::from(config.network_type).0][DOMAIN_SEPARATOR_KEY],
            )
            .ok()
        } else {
            None
        };

        let mut domain_separator_bytes = [0_u8; 32];
        if let Some(domain_separator) = maybe_domain_separator {
            hex::decode_to_slice(domain_separator, &mut domain_separator_bytes)?;
        }

        domain_separator_bytes
    };

    let verifier_set_hash = axelar_solana_encoding::types::verifier_set::verifier_set_hash::<
        NativeHasher,
    >(&verifier_set, &domain_separator)?;

    let (init_tracker_pda, _bump) =
        axelar_solana_gateway::get_verifier_set_tracker_pda(verifier_set_hash);
    let payer = *fee_payer;
    let upgrade_authority = payer;

    chains_info[CHAINS_KEY][ChainNameOnAxelar::from(config.network_type).0][CONTRACTS_KEY]
        [GATEWAY_KEY] = json!({
        ADDRESS_KEY: axelar_solana_gateway::id().to_string(),
        UPGRADE_AUTHORITY_KEY: fee_payer.to_string(),
        OPERATOR_KEY: init_args.operator.to_string(),
        MINIMUM_ROTATION_DELAY_KEY: init_args.minimum_rotation_delay,
        PREVIOUS_SIGNERS_RETENTION_KEY: init_args.previous_signers_retention,
        DOMAIN_SEPARATOR_KEY: domain_separator,
    });

    write_json_to_file_path(&chains_info, &config.chains_info_file)?;

    Ok(vec![
        axelar_solana_gateway::instructions::initialize_config(
            payer,
            upgrade_authority,
            domain_separator,
            vec![(verifier_set_hash, init_tracker_pda)],
            init_args.minimum_rotation_delay,
            init_args.operator,
            init_args.previous_signers_retention.into(),
            gateway_config_pda,
        )?,
    ])
}

async fn call_contract(
    fee_payer: &Pubkey,
    call_contract_args: CallContractArgs,
) -> eyre::Result<Vec<Instruction>> {
    let (signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(*fee_payer);
    let payload = hex::decode(call_contract_args.payload)?;

    Ok(vec![axelar_solana_gateway::instructions::call_contract(
        axelar_solana_gateway::id(),
        axelar_solana_gateway::get_gateway_root_config_pda().0,
        *fee_payer,
        signing_pda,
        signing_pda_bump,
        call_contract_args.destination_chain,
        call_contract_args.destination_contract_address,
        payload,
    )?])
}

async fn transfer_operatorship(
    fee_payer: &Pubkey,
    transfer_operatorship_args: TransferOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![
        axelar_solana_gateway::instructions::transfer_operatorship(
            *fee_payer,
            transfer_operatorship_args.authority,
            transfer_operatorship_args.new_operator,
        )?,
    ])
}

async fn approve(
    fee_payer: &Pubkey,
    approve_args: ApproveArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let mut instructions = vec![];
    let chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    let secret_bytes: [u8; 32] = hex::decode(
        approve_args
            .signer
            .strip_prefix("0x")
            .unwrap_or(&approve_args.signer),
    )?
    .try_into()
    .map_err(|_err| eyre::eyre!("Invalid signer key"))?;
    let sk = libsecp256k1::SecretKey::parse(&secret_bytes)?;
    let pk = libsecp256k1::PublicKey::from_secret_key(&sk);

    let signer = TestSigner {
        inner: sk,
        weight: 1_u128,
    };

    let domain_separator = {
        let maybe_domain_separator = String::deserialize(
            &chains_info[AXELAR_KEY][CONTRACTS_KEY][MULTISIG_PROVER_KEY]
                [ChainNameOnAxelar::from(config.network_type).0][DOMAIN_SEPARATOR_KEY],
        )
        .ok();

        let mut domain_separator_bytes = [0_u8; 32];
        if let Some(domain_separator) = maybe_domain_separator {
            hex::decode_to_slice(domain_separator, &mut domain_separator_bytes)?;
        }

        domain_separator_bytes
    };

    let payload_bytes = hex::decode(
        approve_args
            .payload
            .strip_prefix("0x")
            .unwrap_or(&approve_args.payload),
    )?;
    let payload_hash = solana_sdk::hash::hashv(&[&payload_bytes]).to_bytes();
    let signers_set = SigningVerifierSet::new(vec![signer], 0);
    let message = Message {
        cc_id: CrossChainId {
            chain: approve_args.source_chain,
            id: approve_args.message_id,
        },
        source_address: approve_args.source_address,
        destination_chain: ChainNameOnAxelar::from(config.network_type).0,
        destination_address: approve_args.destination_address,
        payload_hash,
    };

    let gmp_payload = Payload::Messages(Messages(vec![message]));
    let message_hash = hash_payload(
        &domain_separator,
        &signers_set.verifier_set(),
        gmp_payload.clone(),
    )?;

    let message = libsecp256k1::Message::parse(&message_hash);
    let (signature, recovery_id) = libsecp256k1::sign(&message, &sk);
    let mut signature_bytes = signature.serialize().to_vec();
    signature_bytes.push(recovery_id.serialize());
    let signatures = {
        BTreeMap::from([(
            PublicKey::Secp256k1(pk.serialize_compressed()),
            Signature::EcdsaRecoverable(
                signature_bytes
                    .try_into()
                    .map_err(|_e| eyre::eyre!("Invalid signature"))?,
            ),
        )])
    };
    let execute_data_bytes = axelar_solana_encoding::encode(
        &signers_set.verifier_set(),
        &signatures,
        domain_separator,
        gmp_payload,
    )?;

    let execute_data: ExecuteData = borsh::from_slice(&execute_data_bytes)?;
    let gateway_config_pda = axelar_solana_gateway::get_gateway_root_config_pda().0;

    instructions.push(
        axelar_solana_gateway::instructions::initialize_payload_verification_session(
            *fee_payer,
            gateway_config_pda,
            execute_data.payload_merkle_root,
        )?,
    );

    let (verifier_set_tracker_pda, _bump) = axelar_solana_gateway::get_verifier_set_tracker_pda(
        execute_data.signing_verifier_set_merkle_root,
    );

    for signature_leaf in &execute_data.signing_verifier_set_leaves {
        instructions.push(axelar_solana_gateway::instructions::verify_signature(
            gateway_config_pda,
            verifier_set_tracker_pda,
            execute_data.payload_merkle_root,
            signature_leaf.clone(),
        )?);
    }

    let (verification_session_pda, _bump) = axelar_solana_gateway::get_signature_verification_pda(
        &gateway_config_pda,
        &execute_data.payload_merkle_root,
    );

    let MerkleisedPayload::NewMessages { mut messages } = execute_data.payload_items else {
        eyre::bail!("Expected Messages payload");
    };

    let Some(merkleised_message) = messages.pop() else {
        eyre::bail!("No messages in the batch");
    };

    let command_id = command_id(
        &merkleised_message.leaf.message.cc_id.chain,
        &merkleised_message.leaf.message.cc_id.id,
    );

    let (incoming_message_pda, _bump) =
        axelar_solana_gateway::get_incoming_message_pda(&command_id);

    instructions.push(axelar_solana_gateway::instructions::approve_message(
        merkleised_message,
        execute_data.payload_merkle_root,
        gateway_config_pda,
        *fee_payer,
        verification_session_pda,
        incoming_message_pda,
    )?);

    Ok(instructions)
}

async fn rotate(fee_payer: &Pubkey, rotate_args: RotateArgs) -> eyre::Result<Vec<Instruction>> {
    todo!()
}
