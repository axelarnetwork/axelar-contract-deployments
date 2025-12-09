use std::collections::BTreeMap;
use std::str::FromStr;

use anchor_lang::InstructionData;
use base64::Engine as _;
use borsh::BorshDeserialize;
use clap::{ArgGroup, Args, Parser, Subcommand};
use cosmrs::proto::cosmwasm::wasm::v1::query_client;
use eyre::eyre;
use k256::ecdsa::SigningKey;
use k256::elliptic_curve::sec1::ToEncodedPoint;
use serde_json::json;
use solana_axelar_gateway::state::config::RotationDelaySecs;
use solana_axelar_gateway::state::config::{InitialVerifierSet, InitializeConfigParams};
use solana_axelar_std::U256;
use solana_axelar_std::execute_data::{
    ExecuteData, MerklizedPayload, Payload, encode, hash_payload,
};
use solana_axelar_std::hasher::Hasher;
use solana_axelar_std::message::{CrossChainId, MerklizedMessage, Message, MessageLeaf, Messages};
use solana_axelar_std::pubkey::{PublicKey, Signature};
use solana_axelar_std::verifier_set::{VerifierSet, verifier_set_hash};
use solana_client::rpc_client::RpcClient;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::message::Message as SolanaMessage;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature as SolanaSignature;
use solana_sdk::transaction::Transaction as SolanaTransaction;
use solana_transaction_status::{UiInstruction, UiTransactionEncoding};

use crate::config::Config;
use crate::multisig_prover_types::Uint128Extensions;
use crate::multisig_prover_types::msg::ProofStatus;
use crate::types::{
    LocalSigner, SerializableSolanaTransaction, SerializeableVerifierSet, SigningVerifierSet,
    SolanaTransactionParams,
};
use crate::utils::{
    self, ADDRESS_KEY, AXELAR_KEY, CHAINS_KEY, CONNECTION_TYPE_KEY, CONTRACTS_KEY,
    DOMAIN_SEPARATOR_KEY, GATEWAY_KEY, GRPC_KEY, MINIMUM_ROTATION_DELAY_KEY, MULTISIG_PROVER_KEY,
    OPERATOR_KEY, PREVIOUS_SIGNERS_RETENTION_KEY, UPGRADE_AUTHORITY_KEY, domain_separator,
    fetch_latest_blockhash, read_json_file_from_path, write_json_to_file_path,
};

const SOLANA_GATEWAY_CONNECTION_TYPE: &str = "amplifier";

fn command_id(source_chain: &str, message_id: &str) -> [u8; 32] {
    solana_sdk::keccak::hashv(&[source_chain.as_bytes(), b"-", message_id.as_bytes()]).0
}

#[derive(Debug)]
#[allow(dead_code)]
enum GatewayEvent {
    CallContract(solana_axelar_gateway::CallContractEvent),
    VerifierSetRotated(solana_axelar_gateway::VerifierSetRotatedEvent),
    OperatorshipTransferred(solana_axelar_gateway::OperatorshipTransferredEvent),
    MessageApproved(solana_axelar_gateway::MessageApprovedEvent),
    MessageExecuted(solana_axelar_gateway::MessageExecutedEvent),
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Initialize the AxelarGateway program on Solana
    Init(InitArgs),

    /// Call a contract on another chain
    CallContract(CallContractArgs),

    /// Transfer the AxelarGateway program's operatorship to another address
    TransferOperatorship(TransferOperatorshipArgs),

    /// Approve a message using a local SignerSet (required to be the current VerifierSet registered
    /// with the AxelarGateway)
    Approve(ApproveArgs),

    /// Rotate the VerifierSet on the AxelarGateway program. Omit `new_signer` and `new_signer_set`
    /// to query the MultisigProver for the current VerifierSet
    Rotate(RotateArgs),

    /// Submit a proof to the AxelarGateway program, triggering VerifierSet rotation or message
    /// approvals
    SubmitProof(SubmitProofArgs),

    /// Execute a cross-chain message on Solana
    Execute(ExecuteArgs),
}

/// Commands for querying gateway related data
#[derive(Subcommand, Debug)]
pub(crate) enum QueryCommands {
    /// Get GatewayEvents from a transaction
    Events(EventsArgs),

    /// Query message status on Gateway
    MessageStatus(MessageStatusArgs),

    /// Query gateway config (domain separator, operator, etc.)
    Config,

    /// Query verifier set tracker by merkle root hash
    VerifierSetTracker(VerifierSetTrackerArgs),

    /// Compute verifier set merkle root from MultisigProver's current verifier set
    ComputeMerkleRoot,
}

#[derive(Args, Debug)]
pub(crate) struct VerifierSetTrackerArgs {
    /// The verifier set merkle root hash (hex, with or without 0x prefix)
    merkle_root: String,
}

#[derive(Args, Debug)]
pub(crate) struct EventsArgs {
    /// The transaction signature to get events from
    signature: String,

    /// Print all event data
    #[clap(long)]
    full: bool,
}

#[derive(Args, Debug)]
pub(crate) struct MessageStatusArgs {
    /// The name of the chain from which the message was sent as it is registered with Axelar
    source_chain: String,

    /// Message ID
    message_id: String,
}

#[derive(Parser, Debug)]
#[clap(
    group(
        ArgGroup::new("signers_source")
        .args(&["signer", "signer-set"])
        .multiple(false)
        .requires("nonce")
        .required(false)
        )
    )
]
pub(crate) struct InitArgs {
    /// Previous SignerSet retention
    #[clap(long)]
    previous_signers_retention: u128,

    /// Minimum delay between SignerSet rotations
    #[clap(long)]
    minimum_rotation_delay: RotationDelaySecs,

    /// Optional hex string with secp256k1 compressed public key used to create the initial SignerSet
    #[clap(long)]
    signer: Option<String>,

    /// Nonce to be used for the SignerSet, required if `signer` or `signers` is set
    #[clap(long)]
    nonce: Option<u64>,

    /// An optional JSON containing a SignerSet
    #[clap(long)]
    signer_set: Option<String>,

    /// Address of the AxelarGateway program operator
    #[clap(long)]
    operator: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct CallContractArgs {
    /// The chain where the message has to be sent to
    #[clap(long)]
    destination_chain: String,

    /// The destination contract address on the destination chain that should receive the message
    #[clap(long)]
    destination_address: String,

    /// The payload as expected by the destination contract as a hex encoded string
    #[clap(long)]
    payload: String,
}

#[derive(Parser, Debug)]
pub(crate) struct TransferOperatorshipArgs {
    /// Current operator OR upgrade authority
    #[clap(long)]
    authority: Pubkey,

    /// Address of the new operator
    #[clap(long)]
    new_operator: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct ApproveArgs {
    /// Hex string with secp256k1 private key of the signer used to generate the proof
    #[clap(long, value_parser=utils::parse_secret_key, value_hint=clap::ValueHint::AnyPath)]
    signer: k256::SecretKey,

    /// Nonce associated with the SignerSet the signer is part of
    #[clap(long)]
    nonce: u64,

    /// The chain where the message originated from
    #[clap(long)]
    source_chain: String,

    /// The Axelar message identifier
    #[clap(long)]
    message_id: String,

    /// The address of the contract where the message originated from
    #[clap(long)]
    source_address: String,

    /// The destination contract address on Solana that should receive the message
    #[clap(long)]
    destination_address: String,

    /// The payload as expected by the destination contract as a hex encoded string
    #[clap(long)]
    payload: String,
}

#[derive(Parser, Debug)]
#[clap(
    group(
        ArgGroup::new("signers_source")
        .args(&["new-signer", "new-signer-set"])
        .multiple(false)
        .requires("nonce")
        .required(false))
    )
]
pub(crate) struct RotateArgs {
    /// Hex string with secp256k1 private key of the signer used to generate the proof
    #[clap(long, value_parser=utils::parse_secret_key, value_hint=clap::ValueHint::AnyPath)]
    signer: k256::SecretKey,

    /// Nonce to be used for the SignerSet, required if `signer` or `signers` is set.
    #[clap(long)]
    nonce: u64,

    /// Hex string with secp256k1 compressed public key used to create the new SignerSet.
    #[clap(long)]
    new_signer: Option<String>,

    /// A JSON containing a SignerSet
    #[clap(long)]
    new_signer_set: Option<String>,

    /// The new nonce to be used for the new SignerSet, required if `new_signer` or `new_signers` is set
    #[clap(long)]
    new_nonce: Option<u64>,
}

#[derive(Parser, Debug)]
pub(crate) struct SubmitProofArgs {
    /// The session id associated with the proof, used o query the MultisigProver
    #[clap(long)]
    multisig_session_id: u64,
}

#[derive(Parser, Debug)]
pub(crate) struct ExecuteArgs {
    /// Chain where the message originated from
    #[clap(long)]
    source_chain: String,

    /// The Axelar message identifier
    #[clap(long)]
    message_id: String,

    /// Source address of the message in the source chain
    #[clap(long)]
    source_address: String,

    /// The destination contract address on Solana that should receive the message
    #[clap(long)]
    destination_address: String,

    /// The payload as expected by the destination contract as a hex encoded string
    #[clap(long)]
    payload: String,
}

pub(crate) async fn build_transaction(
    fee_payer: &Pubkey,
    command: Commands,
    config: &Config,
) -> eyre::Result<Vec<SerializableSolanaTransaction>> {
    let instructions = match command {
        Commands::Init(init_args) => init(fee_payer, init_args, config).await?,
        Commands::CallContract(call_contract_args) => call_contract(fee_payer, call_contract_args)?,
        Commands::TransferOperatorship(transfer_operatorship_args) => {
            transfer_operatorship(transfer_operatorship_args)?
        }
        Commands::Approve(approve_args) => approve(fee_payer, approve_args, config)?,
        Commands::Rotate(rotate_args) => rotate(fee_payer, rotate_args, config).await?,
        Commands::SubmitProof(submit_proof_args) => {
            submit_proof(fee_payer, submit_proof_args, config).await?
        }
        Commands::Execute(execute_args) => execute(fee_payer, execute_args, config)?,
    };

    let blockhash = fetch_latest_blockhash(&config.url)?;
    let mut serializable_transactions = Vec::with_capacity(instructions.len());

    for instruction in instructions {
        let message =
            SolanaMessage::new_with_blockhash(&[instruction], Some(fee_payer), &blockhash);
        let transaction = SolanaTransaction::new_unsigned(message);
        let params = SolanaTransactionParams {
            fee_payer: fee_payer.to_string(),
            recent_blockhash: Some(blockhash.to_string()),
            nonce_account: None,
            nonce_authority: None,
            blockhash_for_message: blockhash.to_string(),
        };

        let serializable_tx = SerializableSolanaTransaction::new(transaction, params);
        serializable_transactions.push(serializable_tx);
    }

    Ok(serializable_transactions)
}

async fn query_axelar<T: serde::de::DeserializeOwned>(
    mut endpoint: String,
    address: cosmrs::AccountId,
    query_data: Vec<u8>,
) -> eyre::Result<T> {
    if !endpoint.starts_with("https://") {
        endpoint = format!("https://{endpoint}");
    }

    let res = query_client::QueryClient::connect(endpoint)
        .await?
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
    signer: Option<&String>,
    signer_set: Option<&String>,
    nonce: Option<u64>,
    config: &Config,
    chains_info: &serde_json::Value,
) -> eyre::Result<VerifierSet> {
    if let Some(signer_key) = signer {
        let key_bytes: [u8; 33] = hex::decode(signer_key.strip_prefix("0x").unwrap_or(signer_key))?
            .try_into()
            .map_err(|_| eyre!("Invalid key length"))?;

        let pk = k256::PublicKey::from_sec1_bytes(&key_bytes)?;
        let pubkey = PublicKey(
            pk.to_encoded_point(true)
                .as_bytes()
                .try_into()
                .map_err(|_| eyre!("Invalid encoded point conversion"))?,
        );
        let signers = BTreeMap::from([(pubkey, 1u128)]);
        let nonce = nonce.ok_or_else(|| eyre!("Nonce is required"))?;

        Ok(VerifierSet {
            nonce,
            signers,
            quorum: 1u128,
        })
    } else if let Some(signer_set) = signer_set {
        let signer_set: SerializeableVerifierSet = serde_json::from_str(signer_set)?;

        Ok(signer_set.into())
    } else {
        let multisig_prover_address = {
            let address = <String as serde::Deserialize>::deserialize(
                &chains_info[AXELAR_KEY][CONTRACTS_KEY][MULTISIG_PROVER_KEY][&config.chain]
                    [ADDRESS_KEY],
            )?;

            cosmrs::AccountId::from_str(&address).unwrap()
        };
        let axelar_grpc_endpoint =
            <String as serde::Deserialize>::deserialize(&chains_info[AXELAR_KEY][GRPC_KEY])?;
        let multisig_prover_response =
            query_axelar::<crate::multisig_prover_types::VerifierSetResponse>(
                axelar_grpc_endpoint,
                multisig_prover_address,
                serde_json::to_vec(&crate::multisig_prover_types::QueryMsg::CurrentVerifierSet)?,
            )
            .await?;
        let mut signers = BTreeMap::new();

        for signer in multisig_prover_response.verifier_set.signers.values() {
            let pubkey: PublicKey = signer.pub_key.clone().try_into()?;
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

fn construct_execute_data(
    signer_set: &SigningVerifierSet,
    payload: Payload,
    domain_separator: [u8; 32],
) -> eyre::Result<ExecuteData> {
    let message_hash = hash_payload::<Hasher>(&domain_separator, payload.clone())?;
    let signatures = signer_set
        .signers
        .iter()
        .map(|signer| {
            let signing_key = SigningKey::from(&signer.secret);
            let (signature, recovery_id) = signing_key.sign_prehash_recoverable(&message_hash)?;
            let mut signature_bytes = signature.normalize_s().unwrap_or(signature).to_vec();
            signature_bytes.push(recovery_id.to_byte());

            Ok((
                PublicKey(
                    signer
                        .secret
                        .public_key()
                        .to_encoded_point(true)
                        .as_bytes()
                        .try_into()
                        .map_err(|_| eyre!("Invalid signature"))?,
                ),
                Signature(
                    signature_bytes
                        .try_into()
                        .map_err(|_e| eyre!("Invalid signature"))?,
                ),
            ))
        })
        .collect::<eyre::Result<BTreeMap<_, _>>>()?;
    let execute_data_bytes = encode(
        &signer_set.verifier_set(),
        &signatures,
        domain_separator,
        payload,
    )?;
    let execute_data: ExecuteData = ExecuteData::try_from_slice(&execute_data_bytes)?;

    Ok(execute_data)
}

fn build_signing_verifier_set(secret: k256::SecretKey, nonce: u64) -> SigningVerifierSet {
    let signer = LocalSigner {
        secret,
        weight: 1u128,
    };

    SigningVerifierSet::new(vec![signer], nonce)
}

fn append_verification_flow_instructions(
    fee_payer: &Pubkey,
    instructions: &mut Vec<Instruction>,
    execute_data: &ExecuteData,
    gateway_config_pda: &Pubkey,
) -> eyre::Result<Pubkey> {
    let (verifier_set_tracker_pda, _bump) = solana_axelar_gateway::VerifierSetTracker::find_pda(
        &execute_data.signing_verifier_set_merkle_root,
    );

    let (verification_session_pda, _bump) =
        solana_axelar_gateway::SignatureVerificationSessionData::find_pda(
            &execute_data.payload_merkle_root,
            &execute_data.signing_verifier_set_merkle_root,
        );

    let init_session_ix_data =
        solana_axelar_gateway::instruction::InitializePayloadVerificationSession {
            merkle_root: execute_data.payload_merkle_root,
        }
        .data();

    instructions.push(Instruction {
        program_id: solana_axelar_gateway::id(),
        accounts: vec![
            AccountMeta::new(*fee_payer, true),
            AccountMeta::new_readonly(*gateway_config_pda, false),
            AccountMeta::new(verification_session_pda, false),
            AccountMeta::new_readonly(verifier_set_tracker_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data: init_session_ix_data,
    });

    for signature_leaf in &execute_data.signing_verifier_set_leaves {
        let verifier_info = signature_leaf.clone();

        let verify_sig_ix_data = solana_axelar_gateway::instruction::VerifySignature {
            payload_merkle_root: execute_data.payload_merkle_root,
            verifier_info,
        }
        .data();

        instructions.push(Instruction {
            program_id: solana_axelar_gateway::id(),
            accounts: vec![
                AccountMeta::new_readonly(*gateway_config_pda, false),
                AccountMeta::new_readonly(verifier_set_tracker_pda, false),
                AccountMeta::new(verification_session_pda, false),
            ],
            data: verify_sig_ix_data,
        });
    }

    Ok(verification_session_pda)
}

async fn init(
    fee_payer: &Pubkey,
    init_args: InitArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let mut chains_info: serde_json::Value =
        read_json_file_from_path(&config.chains_info_file).unwrap_or_default();
    let (gateway_config_pda, _bump) = solana_axelar_gateway::GatewayConfig::find_pda();
    let verifier_set = get_verifier_set(
        init_args.signer.as_ref(),
        init_args.signer_set.as_ref(),
        init_args.nonce,
        config,
        &chains_info,
    )
    .await?;
    let domain_separator = domain_separator(&chains_info, config.network_type, &config.chain)?;
    let verifier_set_hash = verifier_set_hash::<Hasher>(&verifier_set, &domain_separator)?;
    let (verifier_set_tracker_pda, _bump) =
        solana_axelar_gateway::VerifierSetTracker::find_pda(&verifier_set_hash);
    let payer = *fee_payer;
    let upgrade_authority = payer;

    chains_info[CHAINS_KEY][&config.chain][CONTRACTS_KEY][GATEWAY_KEY] = json!({
        ADDRESS_KEY: solana_axelar_gateway::id().to_string(),
        CONNECTION_TYPE_KEY: SOLANA_GATEWAY_CONNECTION_TYPE.to_owned(),
        DOMAIN_SEPARATOR_KEY: format!("0x{}", hex::encode(domain_separator)),
        MINIMUM_ROTATION_DELAY_KEY: init_args.minimum_rotation_delay,
        OPERATOR_KEY: init_args.operator.to_string(),
        PREVIOUS_SIGNERS_RETENTION_KEY: init_args.previous_signers_retention,
        UPGRADE_AUTHORITY_KEY: fee_payer.to_string(),
    });

    write_json_to_file_path(&chains_info, &config.chains_info_file)?;

    let gateway_program_data =
        solana_sdk::bpf_loader_upgradeable::get_program_data_address(&solana_axelar_gateway::id());

    let params = InitializeConfigParams {
        domain_separator,
        initial_verifier_set: InitialVerifierSet {
            hash: verifier_set_hash,
            pda: verifier_set_tracker_pda,
        },
        minimum_rotation_delay: init_args.minimum_rotation_delay,
        operator: init_args.operator,
        previous_verifier_retention: U256::from(
            u64::try_from(init_args.previous_signers_retention)
                .map_err(|_| eyre!("previous_signers_retention value too large for u64"))?,
        ),
    };

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(upgrade_authority, true),
        AccountMeta::new_readonly(gateway_program_data, false),
        AccountMeta::new(gateway_config_pda, false),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new(verifier_set_tracker_pda, false),
    ];

    let ix_data = solana_axelar_gateway::instruction::InitializeConfig { params }.data();

    Ok(vec![Instruction {
        program_id: solana_axelar_gateway::id(),
        accounts,
        data: ix_data,
    }])
}

fn call_contract(
    fee_payer: &Pubkey,
    call_contract_args: CallContractArgs,
) -> eyre::Result<Vec<Instruction>> {
    let payload = hex::decode(
        call_contract_args
            .payload
            .strip_prefix("0x")
            .unwrap_or(&call_contract_args.payload),
    )?;

    let gateway_config_pda = solana_axelar_gateway::GatewayConfig::find_pda().0;
    let (event_authority_pda, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_gateway::id());

    let ix_data = solana_axelar_gateway::instruction::CallContract {
        destination_chain: call_contract_args.destination_chain,
        destination_contract_address: call_contract_args.destination_address,
        payload,
        signing_pda_bump: 0,
    }
    .data();

    let accounts = vec![
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(gateway_config_pda, false),
        AccountMeta::new_readonly(event_authority_pda, false),
        AccountMeta::new_readonly(solana_axelar_gateway::id(), false),
    ];

    Ok(vec![Instruction {
        program_id: solana_axelar_gateway::id(),
        accounts,
        data: ix_data,
    }])
}

fn transfer_operatorship(
    transfer_operatorship_args: TransferOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    let gateway_config_pda = solana_axelar_gateway::GatewayConfig::find_pda().0;
    let gateway_program_data =
        solana_sdk::bpf_loader_upgradeable::get_program_data_address(&solana_axelar_gateway::id());
    let (event_authority_pda, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_gateway::id());

    let ix_data = solana_axelar_gateway::instruction::TransferOperatorship {}.data();

    Ok(vec![Instruction {
        program_id: solana_axelar_gateway::id(),
        accounts: vec![
            AccountMeta::new(gateway_config_pda, false),
            AccountMeta::new_readonly(transfer_operatorship_args.authority, true),
            AccountMeta::new_readonly(gateway_program_data, false),
            AccountMeta::new_readonly(transfer_operatorship_args.new_operator, false),
            AccountMeta::new_readonly(event_authority_pda, false),
            AccountMeta::new_readonly(solana_axelar_gateway::id(), false),
        ],
        data: ix_data,
    }])
}

fn approve(
    fee_payer: &Pubkey,
    approve_args: ApproveArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let mut instructions = vec![];
    let chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    let signer_set = build_signing_verifier_set(approve_args.signer.clone(), approve_args.nonce);
    let domain_separator = domain_separator(&chains_info, config.network_type, &config.chain)?;
    let payload_bytes = hex::decode(
        approve_args
            .payload
            .strip_prefix("0x")
            .unwrap_or(&approve_args.payload),
    )?;
    let payload_hash = solana_sdk::keccak::hashv(&[&payload_bytes]).to_bytes();
    let message = Message {
        cc_id: CrossChainId {
            chain: approve_args.source_chain,
            id: approve_args.message_id,
        },
        source_address: approve_args.source_address,
        destination_chain: config.chain.clone(),
        destination_address: approve_args.destination_address,
        payload_hash,
    };
    let gateway_config_pda = solana_axelar_gateway::GatewayConfig::find_pda().0;
    let gmp_payload = Payload::Messages(Messages(vec![message]));
    let execute_data = construct_execute_data(&signer_set, gmp_payload, domain_separator)?;
    let verification_session_pda = append_verification_flow_instructions(
        fee_payer,
        &mut instructions,
        &execute_data,
        &gateway_config_pda,
    )?;
    let MerklizedPayload::NewMessages { mut messages } = execute_data.payload_items else {
        eyre::bail!("Expected Messages payload");
    };
    let Some(merklized_message) = messages.pop() else {
        eyre::bail!("No messages in the batch");
    };
    let command_id = command_id(
        &merklized_message.leaf.message.cc_id.chain,
        &merklized_message.leaf.message.cc_id.id,
    );
    let (incoming_message_pda, _bump) =
        solana_axelar_gateway::IncomingMessage::find_pda(&command_id);

    println!(
        "Building instruction to approve message from {} with id: {}",
        merklized_message.leaf.message.cc_id.chain, merklized_message.leaf.message.cc_id.id
    );

    let (event_authority_pda, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_gateway::id());

    let v2_merklized_message = MerklizedMessage {
        leaf: MessageLeaf {
            message: Message {
                cc_id: CrossChainId {
                    chain: merklized_message.leaf.message.cc_id.chain.clone(),
                    id: merklized_message.leaf.message.cc_id.id.clone(),
                },
                source_address: merklized_message.leaf.message.source_address.clone(),
                destination_chain: merklized_message.leaf.message.destination_chain.clone(),
                destination_address: merklized_message.leaf.message.destination_address.clone(),
                payload_hash: merklized_message.leaf.message.payload_hash,
            },
            position: merklized_message.leaf.position,
            set_size: merklized_message.leaf.set_size,
            domain_separator: merklized_message.leaf.domain_separator,
        },
        proof: merklized_message.proof.clone(),
    };

    let approve_ix_data = solana_axelar_gateway::instruction::ApproveMessage {
        merklized_message: v2_merklized_message,
        payload_merkle_root: execute_data.payload_merkle_root,
    }
    .data();

    instructions.push(Instruction {
        program_id: solana_axelar_gateway::id(),
        accounts: vec![
            AccountMeta::new_readonly(gateway_config_pda, false),
            AccountMeta::new(*fee_payer, true),
            AccountMeta::new_readonly(verification_session_pda, false),
            AccountMeta::new(incoming_message_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(event_authority_pda, false),
            AccountMeta::new_readonly(solana_axelar_gateway::id(), false),
        ],
        data: approve_ix_data,
    });

    Ok(instructions)
}

async fn rotate(
    fee_payer: &Pubkey,
    rotate_args: RotateArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let mut instructions = vec![];
    let chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    let signer_set = build_signing_verifier_set(rotate_args.signer, rotate_args.nonce);
    let new_verifier_set = get_verifier_set(
        rotate_args.new_signer.as_ref(),
        rotate_args.new_signer_set.as_ref(),
        rotate_args.new_nonce,
        config,
        &chains_info,
    )
    .await?;
    let domain_separator = domain_separator(&chains_info, config.network_type, &config.chain)?;
    let current_verifier_set_hash =
        verifier_set_hash::<Hasher>(&signer_set.verifier_set(), &domain_separator)?;
    let new_verifier_set_hash = verifier_set_hash::<Hasher>(&new_verifier_set, &domain_separator)?;
    let (verifier_set_tracker_pda, _bump) =
        solana_axelar_gateway::VerifierSetTracker::find_pda(&current_verifier_set_hash);
    let (new_verifier_set_tracker_pda, _bump) =
        solana_axelar_gateway::VerifierSetTracker::find_pda(&new_verifier_set_hash);
    let gateway_config_pda = solana_axelar_gateway::GatewayConfig::find_pda().0;
    let payload = Payload::NewVerifierSet(new_verifier_set.clone());
    let execute_data = construct_execute_data(&signer_set, payload, domain_separator)?;
    let verification_session_pda = append_verification_flow_instructions(
        fee_payer,
        &mut instructions,
        &execute_data,
        &gateway_config_pda,
    )?;

    let (event_authority_pda, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_gateway::id());

    let rotate_ix_data = solana_axelar_gateway::instruction::RotateSigners {
        new_verifier_set_merkle_root: new_verifier_set_hash,
    }
    .data();

    instructions.push(Instruction {
        program_id: solana_axelar_gateway::id(),
        accounts: vec![
            AccountMeta::new(gateway_config_pda, false),
            AccountMeta::new_readonly(verification_session_pda, false),
            AccountMeta::new_readonly(verifier_set_tracker_pda, false),
            AccountMeta::new(new_verifier_set_tracker_pda, false),
            AccountMeta::new(*fee_payer, true),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(event_authority_pda, false),
            AccountMeta::new_readonly(solana_axelar_gateway::id(), false),
        ],
        data: rotate_ix_data,
    });

    Ok(instructions)
}

#[allow(clippy::too_many_lines)]
async fn submit_proof(
    fee_payer: &Pubkey,
    submit_proof_args: SubmitProofArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    let multisig_prover_address = {
        let address = <String as serde::Deserialize>::deserialize(
            &chains_info[AXELAR_KEY][CONTRACTS_KEY][MULTISIG_PROVER_KEY][&config.chain]
                [ADDRESS_KEY],
        )?;

        cosmrs::AccountId::from_str(&address).unwrap()
    };
    let axelar_grpc_endpoint =
        <String as serde::Deserialize>::deserialize(&chains_info[AXELAR_KEY][GRPC_KEY])?;
    let multisig_prover_response = query_axelar::<crate::multisig_prover_types::ProofResponse>(
        axelar_grpc_endpoint,
        multisig_prover_address,
        serde_json::to_vec(&crate::multisig_prover_types::QueryMsg::Proof {
            multisig_session_id: submit_proof_args.multisig_session_id,
        })?,
    )
    .await?;

    let gateway_config_pda = solana_axelar_gateway::GatewayConfig::find_pda().0;
    let execute_data: ExecuteData = match multisig_prover_response.status {
        ProofStatus::Pending => eyre::bail!("Proof is not completed yet"),
        ProofStatus::Completed { execute_data } => {
            ExecuteData::try_from_slice(execute_data.as_slice())?
        }
    };

    let mut instructions = Vec::new();
    let verification_session_pda = append_verification_flow_instructions(
        fee_payer,
        &mut instructions,
        &execute_data,
        &gateway_config_pda,
    )?;

    match execute_data.payload_items {
        MerklizedPayload::VerifierSetRotation {
            new_verifier_set_merkle_root,
        } => {
            println!("Building instruction to rotate signers");
            let (verifier_set_tracker_pda, _bump) =
                solana_axelar_gateway::VerifierSetTracker::find_pda(
                    &execute_data.signing_verifier_set_merkle_root,
                );
            let (new_verifier_set_tracker_pda, _bump) =
                solana_axelar_gateway::VerifierSetTracker::find_pda(&new_verifier_set_merkle_root);
            let (event_authority_pda, _) =
                Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_gateway::id());

            let rotate_ix_data = solana_axelar_gateway::instruction::RotateSigners {
                new_verifier_set_merkle_root,
            }
            .data();

            instructions.push(Instruction {
                program_id: solana_axelar_gateway::id(),
                accounts: vec![
                    AccountMeta::new(gateway_config_pda, false),
                    AccountMeta::new_readonly(verification_session_pda, false),
                    AccountMeta::new_readonly(verifier_set_tracker_pda, false),
                    AccountMeta::new(new_verifier_set_tracker_pda, false),
                    AccountMeta::new(*fee_payer, true),
                    AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
                    AccountMeta::new_readonly(event_authority_pda, false),
                    AccountMeta::new_readonly(solana_axelar_gateway::id(), false),
                ],
                data: rotate_ix_data,
            });
        }
        MerklizedPayload::NewMessages { messages } => {
            for message in messages {
                println!(
                    "Building instruction to approve message from {} with id: {}",
                    message.leaf.message.cc_id.chain, message.leaf.message.cc_id.id
                );
                let msg_command_id = command_id(
                    message.leaf.message.cc_id.chain.as_str(),
                    message.leaf.message.cc_id.id.as_str(),
                );
                let (incoming_message_pda, _bump) =
                    solana_axelar_gateway::IncomingMessage::find_pda(&msg_command_id);

                let (event_authority_pda, _) = Pubkey::find_program_address(
                    &[b"__event_authority"],
                    &solana_axelar_gateway::id(),
                );

                let v2_merklized_message = MerklizedMessage {
                    leaf: MessageLeaf {
                        message: Message {
                            cc_id: CrossChainId {
                                chain: message.leaf.message.cc_id.chain.clone(),
                                id: message.leaf.message.cc_id.id.clone(),
                            },
                            source_address: message.leaf.message.source_address.clone(),
                            destination_chain: message.leaf.message.destination_chain.clone(),
                            destination_address: message.leaf.message.destination_address.clone(),
                            payload_hash: message.leaf.message.payload_hash,
                        },
                        position: message.leaf.position,
                        set_size: message.leaf.set_size,
                        domain_separator: message.leaf.domain_separator,
                    },
                    proof: message.proof.clone(),
                };

                let approve_ix_data = solana_axelar_gateway::instruction::ApproveMessage {
                    merklized_message: v2_merklized_message,
                    payload_merkle_root: execute_data.payload_merkle_root,
                }
                .data();

                instructions.push(Instruction {
                    program_id: solana_axelar_gateway::id(),
                    accounts: vec![
                        AccountMeta::new_readonly(gateway_config_pda, false),
                        AccountMeta::new(*fee_payer, true),
                        AccountMeta::new_readonly(verification_session_pda, false),
                        AccountMeta::new(incoming_message_pda, false),
                        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
                        AccountMeta::new_readonly(event_authority_pda, false),
                        AccountMeta::new_readonly(solana_axelar_gateway::id(), false),
                    ],
                    data: approve_ix_data,
                });
            }
        }
    }

    Ok(instructions)
}

fn execute(
    _fee_payer: &Pubkey,
    execute_args: ExecuteArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let payload = hex::decode(
        execute_args
            .payload
            .strip_prefix("0x")
            .unwrap_or(&execute_args.payload),
    )?;

    let message = Message {
        cc_id: CrossChainId {
            chain: execute_args.source_chain,
            id: execute_args.message_id,
        },
        source_address: execute_args.source_address,
        destination_chain: config.chain.clone(),
        destination_address: execute_args.destination_address,
        payload_hash: solana_sdk::keccak::hashv(&[&payload]).to_bytes(),
    };

    let command_id = command_id(&message.cc_id.chain, &message.cc_id.id);
    let (_incoming_message_pda, _) = solana_axelar_gateway::IncomingMessage::find_pda(&command_id);

    let destination_address = Pubkey::from_str(&message.destination_address).map_err(|e| {
        eyre::eyre!(
            "Invalid destination address '{}': {}",
            message.destination_address,
            e
        )
    })?;

    if destination_address == solana_axelar_its::id() {
        eyre::bail!("ITS GMP execution not yet implemented.");
    } else if destination_address == solana_axelar_governance::id() {
        eyre::bail!(
            "Governance GMP execution not yet implemented for new Anchor program. Use governance-specific commands instead."
        );
    } else {
        eyre::bail!(
            "Generic executable instruction building not yet implemented for v2. Use ITS or Governance specific commands."
        );
    }
}

pub(crate) async fn query(command: QueryCommands, config: &Config) -> eyre::Result<()> {
    match command {
        QueryCommands::Events(args) => events(args, config),
        QueryCommands::MessageStatus(args) => message_status(args, config),
        QueryCommands::Config => gateway_config(config),
        QueryCommands::VerifierSetTracker(args) => verifier_set_tracker(args, config),
        QueryCommands::ComputeMerkleRoot => compute_merkle_root(config).await,
    }
}

fn gateway_config(config: &Config) -> eyre::Result<()> {
    use anchor_lang::AccountDeserialize;
    use solana_axelar_gateway::state::config::GatewayConfig;

    let rpc_client = RpcClient::new(config.url.clone());
    let (gateway_config_pda, _bump) = solana_axelar_gateway::GatewayConfig::find_pda();

    let account_data = rpc_client.get_account_data(&gateway_config_pda)?;
    let gateway_config = GatewayConfig::try_deserialize(&mut account_data.as_slice())?;

    println!("Gateway Config PDA: {gateway_config_pda}");
    println!(
        "Domain Separator: 0x{}",
        hex::encode(gateway_config.domain_separator)
    );
    println!("Operator: {}", gateway_config.operator);
    println!("Current Epoch: {}", gateway_config.current_epoch);
    println!(
        "Minimum Rotation Delay: {} seconds",
        gateway_config.minimum_rotation_delay
    );
    println!(
        "Previous Verifier Set Retention: {}",
        gateway_config.previous_verifier_set_retention
    );
    println!(
        "Last Rotation Timestamp: {}",
        gateway_config.last_rotation_timestamp
    );

    Ok(())
}

fn verifier_set_tracker(args: VerifierSetTrackerArgs, config: &Config) -> eyre::Result<()> {
    use anchor_lang::AccountDeserialize;
    use solana_axelar_gateway::state::verifier_set_tracker::VerifierSetTracker;

    let merkle_root_hex = args.merkle_root.trim_start_matches("0x");
    let merkle_root: [u8; 32] = hex::decode(merkle_root_hex)?
        .try_into()
        .map_err(|_| eyre!("Invalid merkle root length, expected 32 bytes"))?;

    let (tracker_pda, bump) = solana_axelar_gateway::VerifierSetTracker::find_pda(&merkle_root);

    println!("Verifier Set Merkle Root: 0x{}", hex::encode(merkle_root));
    println!("Tracker PDA: {tracker_pda}");
    println!("PDA Bump: {bump}");

    let rpc_client = RpcClient::new(config.url.clone());
    match rpc_client.get_account_data(&tracker_pda) {
        Ok(account_data) => {
            let tracker = VerifierSetTracker::try_deserialize(&mut account_data.as_slice())?;
            println!("\nTracker exists on-chain:");
            println!("  Epoch: {}", tracker.epoch);
            println!(
                "  Verifier Set Hash: 0x{}",
                hex::encode(tracker.verifier_set_hash)
            );
        }
        Err(e) => {
            println!("\nTracker does NOT exist on-chain: {e}");
            println!("This verifier set has not been registered with the gateway.");
        }
    }

    Ok(())
}

async fn compute_merkle_root(config: &Config) -> eyre::Result<()> {
    use std::collections::BTreeMap;

    let chains_info: serde_json::Value =
        crate::utils::read_json_file_from_path(&config.chains_info_file)?;

    let multisig_prover_address = {
        let address = <String as serde::Deserialize>::deserialize(
            &chains_info[crate::utils::AXELAR_KEY][crate::utils::CONTRACTS_KEY]
                [crate::utils::MULTISIG_PROVER_KEY][&config.chain][crate::utils::ADDRESS_KEY],
        )?;
        cosmrs::AccountId::from_str(&address)?
    };

    let axelar_grpc_endpoint = <String as serde::Deserialize>::deserialize(
        &chains_info[crate::utils::AXELAR_KEY][crate::utils::GRPC_KEY],
    )?;

    println!("Querying MultisigProver: {multisig_prover_address}");
    println!("GRPC Endpoint: {axelar_grpc_endpoint}");

    let multisig_prover_response =
        query_axelar::<crate::multisig_prover_types::VerifierSetResponse>(
            axelar_grpc_endpoint,
            multisig_prover_address,
            serde_json::to_vec(&crate::multisig_prover_types::QueryMsg::CurrentVerifierSet)?,
        )
        .await?;

    let mut signers = BTreeMap::new();
    for signer in multisig_prover_response.verifier_set.signers.values() {
        let pubkey: PublicKey = signer.pub_key.clone().try_into()?;
        let weight = signer.weight.u128();
        signers.insert(pubkey, weight);
    }

    let verifier_set = VerifierSet {
        nonce: multisig_prover_response.verifier_set.created_at,
        signers,
        quorum: multisig_prover_response.verifier_set.threshold.u128(),
    };

    println!("\nVerifier Set from Prover:");
    println!("  Nonce (created_at): {}", verifier_set.nonce);
    println!("  Quorum (threshold): {}", verifier_set.quorum);
    println!("  Signers: {} total", verifier_set.signers.len());
    for (pubkey, weight) in &verifier_set.signers {
        println!(
            "    - pubkey: 0x{}, weight: {}",
            hex::encode(pubkey.0),
            weight
        );
    }

    let domain_sep =
        crate::utils::domain_separator(&chains_info, config.network_type, &config.chain)?;
    println!("\nDomain Separator: 0x{}", hex::encode(domain_sep));

    let merkle_root = verifier_set_hash::<Hasher>(&verifier_set, &domain_sep)?;
    println!("\nComputed Merkle Root: 0x{}", hex::encode(merkle_root));

    let (tracker_pda, _) = solana_axelar_gateway::VerifierSetTracker::find_pda(&merkle_root);
    println!("Corresponding Tracker PDA: {tracker_pda}");

    Ok(())
}

fn events(args: EventsArgs, config: &Config) -> eyre::Result<()> {
    let rpc_client = RpcClient::new(config.url.clone());
    let signature = SolanaSignature::from_str(&args.signature)?;
    let transaction = rpc_client.get_transaction(&signature, UiTransactionEncoding::Base64)?;

    let meta = transaction
        .transaction
        .meta
        .ok_or_else(|| eyre!("Transaction missing metadata"))?;

    let inner_instructions = meta.inner_instructions.unwrap_or_else(std::vec::Vec::new);

    let mut event_count = 0;

    for (invocation_index, inner_ix_set) in inner_instructions.iter().enumerate() {
        let invocation_events = inner_ix_set
            .instructions
            .iter()
            .filter_map(|inner_ix| {
                let data = match inner_ix {
                    UiInstruction::Compiled(compiled_ix) => {
                        match base64::engine::general_purpose::STANDARD.decode(&compiled_ix.data) {
                            Ok(d) => d,
                            Err(_) => return None,
                        }
                    }
                    UiInstruction::Parsed(_) => return None,
                };

                match parse_gateway_event(&data) {
                    Ok(Some(event)) => Some(event),
                    Ok(None) => None,
                    Err(e) => {
                        eprintln!("\u{26A0}\u{FE0F}  Warning: {e}");
                        None
                    }
                }
            })
            .collect::<Vec<_>>();

        if !invocation_events.is_empty() {
            println!("\u{2728} Invocation index [{invocation_index}]: ");
            for (event_index, event) in invocation_events.iter().enumerate() {
                print!("\t\u{1F4EC} Event index [{event_index}]: ");
                let raw_output = format!("{event:#?}");

                let output = if args.full {
                    raw_output.as_str()
                } else {
                    raw_output
                        .split_once('(')
                        .map_or(raw_output.as_str(), |(name, _)| name)
                };

                println!("{output}");
                event_count += 1;
            }
        }
    }

    if event_count == 0 {
        println!("\u{1F4EA} No gateway events found");
    }

    Ok(())
}

#[allow(clippy::missing_asserts_for_indexing)]
fn parse_gateway_event(data: &[u8]) -> eyre::Result<Option<GatewayEvent>> {
    use anchor_lang::AnchorDeserialize;
    use anchor_lang::Discriminator;
    use solana_axelar_gateway::{
        CallContractEvent, MessageApprovedEvent, MessageExecutedEvent,
        OperatorshipTransferredEvent, VerifierSetRotatedEvent,
    };

    if data.len() < 16 {
        return Ok(None);
    }

    let ev_disc = &data[0..8];
    if ev_disc != anchor_lang::event::EVENT_IX_TAG_LE {
        return Ok(None);
    }

    let disc = &data[8..16];
    let event_data = &data[16..];

    match disc {
        x if x == CallContractEvent::DISCRIMINATOR => {
            let event = CallContractEvent::deserialize(&mut &*event_data)
                .map_err(|e| eyre!("Failed to deserialize CallContractEvent: {}", e))?;
            Ok(Some(GatewayEvent::CallContract(event)))
        }
        x if x == VerifierSetRotatedEvent::DISCRIMINATOR => {
            let event = VerifierSetRotatedEvent::deserialize(&mut &*event_data)
                .map_err(|e| eyre!("Failed to deserialize VerifierSetRotatedEvent: {}", e))?;
            Ok(Some(GatewayEvent::VerifierSetRotated(event)))
        }
        x if x == OperatorshipTransferredEvent::DISCRIMINATOR => {
            let event = OperatorshipTransferredEvent::deserialize(&mut &*event_data)
                .map_err(|e| eyre!("Failed to deserialize OperatorshipTransferredEvent: {}", e))?;
            Ok(Some(GatewayEvent::OperatorshipTransferred(event)))
        }
        x if x == MessageApprovedEvent::DISCRIMINATOR => {
            let event = MessageApprovedEvent::deserialize(&mut &*event_data)
                .map_err(|e| eyre!("Failed to deserialize MessageApprovedEvent: {}", e))?;
            Ok(Some(GatewayEvent::MessageApproved(event)))
        }
        x if x == MessageExecutedEvent::DISCRIMINATOR => {
            let event = MessageExecutedEvent::deserialize(&mut &*event_data)
                .map_err(|e| eyre!("Failed to deserialize MessageExecutedEvent: {}", e))?;
            Ok(Some(GatewayEvent::MessageExecuted(event)))
        }
        _ => Ok(None),
    }
}

fn message_status(args: MessageStatusArgs, config: &Config) -> eyre::Result<()> {
    use anchor_lang::AccountDeserialize;

    let rpc_client = RpcClient::new(config.url.clone());
    let command_id = solana_sdk::keccak::hashv(&[
        args.source_chain.as_bytes(),
        b"-",
        args.message_id.as_bytes(),
    ])
    .0;
    let (incoming_message_pda, _) = solana_axelar_gateway::IncomingMessage::find_pda(&command_id);
    let raw_incoming_message =
        rpc_client
            .get_account_data(&incoming_message_pda)
            .map_err(|_| {
                eyre!("Couldn't fetch information about given message. Are the details correct?")
            })?;
    let incoming_message =
        solana_axelar_gateway::IncomingMessage::try_deserialize(&mut &raw_incoming_message[8..])
            .map_err(|_| eyre!("Failed to deserialize message data"))?;

    let status = if incoming_message.status.is_approved() {
        String::from("Approved")
    } else {
        String::from("Executed")
    };
    println!("Message status: {status}");

    Ok(())
}
