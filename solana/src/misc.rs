use std::str::FromStr;

use axelar_executable::AxelarMessagePayload;
use axelar_executable::EncodingScheme;
use clap::{Args, Subcommand};
use eyre::Result;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;

/// Commands for miscellaneous utilities
#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Build an axelar-executable message
    BuildAxelarMessage(BuildAxelarMessageArgs),
}

#[derive(Args, Debug)]
pub(crate) struct BuildAxelarMessageArgs {
    /// Accounts in the format of "pubkey:is_signer:is_writable" (e.g., "HQ57JcVZEMkpfEYJRJqnoH6wQdrNqNP6TDvzqnpPYBXQ:true:false"). The order should be the same as expected by the destination program.
    #[clap(long, multiple_values = true)]
    accounts: Vec<String>,

    /// Raw payload as a hex string
    #[clap(long)]
    payload: String,

    /// Use ABI encoding instead of Borsh
    #[clap(long)]
    abi: bool,
}

/// Build a message for miscellaneous utilities
pub(crate) fn do_misc(args: Commands) -> Result<()> {
    match args {
        Commands::BuildAxelarMessage(args) => build_axelar_message(args),
    }
}

fn build_axelar_message(args: BuildAxelarMessageArgs) -> Result<()> {
    // Parse accounts
    let mut account_metas = Vec::with_capacity(args.accounts.len());
    for account_str in args.accounts {
        let parts: Vec<&str> = account_str.split(':').collect();
        if parts.len() != 3 {
            return Err(eyre::eyre!(
                "Invalid account format. Expected 'pubkey:is_signer:is_writable'"
            ));
        }

        let pubkey = Pubkey::from_str(parts[0])?;
        let is_signer = parts[1].parse::<bool>()?;
        let is_writable = parts[2].parse::<bool>()?;

        account_metas.push(AccountMeta {
            pubkey,
            is_signer,
            is_writable,
        });
    }

    // Decode payload from hex
    let payload_bytes = hex::decode(&args.payload)?;

    // Set encoding scheme
    let encoding_scheme = if args.abi {
        EncodingScheme::AbiEncoding
    } else {
        EncodingScheme::Borsh
    };

    // Build AxelarMessagePayload
    let axelar_message = AxelarMessagePayload::new(&payload_bytes, &account_metas, encoding_scheme);

    // Encode the payload
    let encoded = axelar_message
        .encode()
        .map_err(|e| eyre::eyre!("Failed to encode message: {e}"))?;

    // Print the encoded payload as a hex string
    println!("{}", hex::encode(encoded));

    Ok(())
}
