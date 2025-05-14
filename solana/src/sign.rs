use std::collections::HashSet;
use std::path::PathBuf;
use std::str::FromStr;

use eyre::eyre;
use solana_clap_v3_utils::keypair::signer_from_path;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;

use crate::types::PartialSignature;
use crate::utils;

#[derive(Debug, Clone)]
pub(crate) struct SignArgs {
    pub(crate) unsigned_tx_path: PathBuf,
    pub(crate) signer_key: String,
    pub(crate) output_dir: Option<PathBuf>,
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

pub(crate) fn sign_solana_transaction(args: &SignArgs) -> eyre::Result<()> {
    println!("Starting Solana transaction signing...");

    let unsigned_tx = utils::load_unsigned_solana_transaction(&args.unsigned_tx_path)?;
    println!(
        "Loaded unsigned Solana transaction from: {}",
        args.unsigned_tx_path.display()
    );

    let message_bytes = hex::decode(&unsigned_tx.signable_message_hex)
        .map_err(|e| eyre!("Failed to decode signable_message_hex from unsigned tx file: {e}"))?;
    println!(
        "Decoded message bytes ({} bytes) to sign.",
        message_bytes.len()
    );

    println!("Loading signer from: {}", args.signer_key);
    let signer_context = clap::ArgMatches::default();
    let signer = signer_from_path(&signer_context, &args.signer_key, "signer", &mut None)
        .map_err(|e| eyre!("Failed to load signer '{}': {}", args.signer_key, e))?;

    let signer_pubkey = signer.pubkey();
    println!("Signer loaded successfully. Pubkey: {signer_pubkey}");

    println!("Signing message with loaded signer...");
    let signature = signer
        .try_sign_message(&message_bytes)
        .map_err(|e| eyre!("Failed to sign message using '{}': {}", args.signer_key, e))?;
    println!("Generated signature: {signature}");

    let partial_signature = PartialSignature {
        signer_pubkey: signer_pubkey.to_string(),
        signature: signature.to_string(),
    };

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

    if required_signers.contains(&signer_pubkey) {
        println!("Validation OK: Signer {signer_pubkey} is required by the transaction.");
    } else {
        println!(
            "Warning: Signer {signer_pubkey} provided a signature, but is not found in the list of required signers (Fee Payer, Nonce Authority, or Instruction Signers)."
        );
    }

    println!("Signature generated successfully:");
    println!("  Signer Pubkey: {}", partial_signature.signer_pubkey);
    println!("  Signature: {}", partial_signature.signature);

    let output_dir = args.output_dir.clone().unwrap_or_else(|| {
        args.unsigned_tx_path.parent().map_or_else(
            || std::path::PathBuf::from("."),
            std::path::Path::to_path_buf,
        )
    });

    std::fs::create_dir_all(&output_dir)?;

    let unsigned_file_stem = args
        .unsigned_tx_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    let tx_name = unsigned_file_stem.replace(".unsigned", "");

    let pubkey_str = signer_pubkey.to_string();
    let sig_filename = format!("{tx_name}.{pubkey_str}.partial.sig");
    let sig_path = output_dir.join(sig_filename);

    utils::save_partial_signature(&partial_signature, &sig_path)?;

    println!("Partial signature saved to: {}", sig_path.display());

    Ok(())
}
