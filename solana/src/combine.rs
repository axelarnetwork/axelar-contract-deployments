use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;

use solana_sdk::{pubkey::Pubkey, signature::Signature as SolanaSignature};

use crate::config::Config;
use crate::types::{NetworkType, PartialSignature, SignedSolanaTransaction};
use crate::utils;

#[derive(Debug, Clone)]
pub struct CombineArgs {
    pub unsigned_tx_path: PathBuf,
    pub signature_paths: Vec<PathBuf>,
    pub output_signed_tx_path: PathBuf,
}

fn get_required_signers_from_instructions(
    instructions: &[crate::types::SerializableInstruction],
    fee_payer: &Pubkey,
    nonce_authority: Option<&Pubkey>,
) -> eyre::Result<HashSet<Pubkey>> {
    let mut signers = HashSet::new();
    signers.insert(*fee_payer);
    if let Some(na) = nonce_authority {
        signers.insert(*na);
    }
    for ix in instructions {
        for acc_meta in &ix.accounts {
            if acc_meta.is_signer {
                signers.insert(Pubkey::from_str(&acc_meta.pubkey)?);
            }
        }
    }
    Ok(signers)
}

pub fn combine_solana_signatures(args: &CombineArgs, config: &Config) -> eyre::Result<()> {
    println!("Starting Solana signature combination...");

    let unsigned_tx = utils::load_unsigned_solana_transaction(&args.unsigned_tx_path)?;
    println!(
        "Loaded unsigned transaction data from: {}",
        args.unsigned_tx_path.display()
    );

    let fee_payer = Pubkey::from_str(&unsigned_tx.params.fee_payer)?;
    let nonce_authority_pubkey: Option<Pubkey> = unsigned_tx
        .params
        .nonce_authority
        .as_ref()
        .map(|s| Pubkey::from_str(s))
        .transpose()?;
    let required_signers = get_required_signers_from_instructions(
        &unsigned_tx.instructions,
        &fee_payer,
        nonce_authority_pubkey.as_ref(),
    )?;
    println!(
        "Required signers determined from unsigned data: {:?}",
        required_signers
            .iter()
            .map(|pk| pk.to_string())
            .collect::<Vec<_>>()
    );

    let mut signatures_map: HashMap<Pubkey, SolanaSignature> = HashMap::new();
    let mut loaded_paths = HashSet::new();

    for sig_path in &args.signature_paths {
        if !loaded_paths.insert(sig_path.clone()) {
            println!(
                "Skipping duplicate signature file path: {}",
                sig_path.display()
            );
            continue;
        }
        let partial_sig = utils::load_partial_signature(sig_path)?;
        println!("Loaded signature from path: {}", sig_path.display());
        let signer_pubkey = Pubkey::from_str(&partial_sig.signer_pubkey)?;
        let signature = SolanaSignature::from_str(&partial_sig.signature)?;
        println!(" -> Signer: {}, Signature: {}", signer_pubkey, signature);

        if !required_signers.contains(&signer_pubkey) {
            println!(
                "Warning: Signature provided by {} who is not listed as a required signer. Including it anyway.",
                signer_pubkey
            );
        }
        if let Some(existing_sig) = signatures_map.insert(signer_pubkey, signature) {
            if existing_sig != signature {
                eyre::bail!(
                    "Conflicting signatures provided for the same signer: {}.",
                    signer_pubkey
                );
            }
        }
    }

    if signatures_map.is_empty() {
        eyre::bail!("No valid signatures were loaded from the provided paths.");
    }
    println!("Loaded {} unique signatures.", signatures_map.len());

    let mut missing_signers = Vec::new();
    for required_signer in &required_signers {
        if !signatures_map.contains_key(required_signer) {
            missing_signers.push(required_signer.to_string());
        }
    }

    if !missing_signers.is_empty() {
        eyre::bail!("Missing required signatures for: {:?}", missing_signers);
    }
    println!("Validation OK: All required signers have provided signatures.");

    let message_bytes = hex::decode(&unsigned_tx.signable_message_hex)?;
    for (signer_pubkey, signature) in &signatures_map {
        if !signature.verify(signer_pubkey.as_ref(), &message_bytes) {
            eyre::bail!(
                "Signature verification failed for signer: {}",
                signer_pubkey
            );
        }
    }

    let partial_signatures_vec: Vec<PartialSignature> = signatures_map
        .into_iter()
        .map(|(pubkey, sig)| PartialSignature {
            signer_pubkey: pubkey.to_string(),
            signature: sig.to_string(),
        })
        .collect();

    let signed_tx = SignedSolanaTransaction {
        unsigned_tx_data: unsigned_tx,
        signatures: partial_signatures_vec,
    };

    utils::save_signed_solana_transaction(&signed_tx, &args.output_signed_tx_path)?;
    println!(
        "Combined signed Solana transaction data saved to: {}",
        args.output_signed_tx_path.display()
    );

    if config.network_type == NetworkType::Mainnet {
        println!(
            "-> Combined transaction file should be transferred to an online machine for broadcasting."
        );
    }

    Ok(())
}
