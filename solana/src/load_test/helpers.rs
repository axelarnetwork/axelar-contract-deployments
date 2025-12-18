//! Helper functions for load testing (keypair derivation, PDA lookups, etc.).

use std::sync::Arc;

use eyre::eyre;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

use crate::config::Config;

pub(crate) fn load_default_keypair(fee_payer_path: Option<&str>) -> eyre::Result<Keypair> {
    let key_path = if let Some(path) = fee_payer_path {
        path.to_owned()
    } else {
        let config = solana_cli_config::CONFIG_FILE
            .as_ref()
            .and_then(|config_file| solana_cli_config::Config::load(config_file).ok())
            .ok_or_else(|| eyre!("No --fee-payer provided and no Solana CLI config found"))?;
        config.keypair_path
    };

    solana_sdk::signature::read_keypair_file(&key_path)
        .map_err(|e| eyre!("Failed to read keypair from {}: {}", key_path, e))
}

#[derive(borsh::BorshDeserialize, Debug)]
struct FlowSlot {
    _flow_limit: Option<u64>,
    _flow_in: u64,
    _flow_out: u64,
    _epoch: u64,
}

#[derive(borsh::BorshDeserialize, Debug)]
struct TokenManager {
    _ty: u8,
    _token_id: [u8; 32],
    token_address: Pubkey,
    _associated_token_account: Pubkey,
    _flow_slot: FlowSlot,
    _bump: u8,
}

pub(crate) fn get_mint_from_token_manager(
    token_id: &[u8; 32],
    config: &Config,
) -> eyre::Result<Pubkey> {
    use borsh::BorshDeserialize as _;

    let rpc_client = RpcClient::new(config.url.clone());
    let (its_root_pda, _) = find_its_root_pda();
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, token_id);
    let account = rpc_client.get_account(&token_manager_pda)?;
    if account.data.len() < 8 {
        return Err(eyre!(
            "TokenManager account data too short: expected at least 8 bytes (discriminator), got {}",
            account.data.len()
        ));
    }
    let mut data = &account.data[8..];
    let token_manager = TokenManager::deserialize(&mut data)?;
    Ok(token_manager.token_address)
}

pub(crate) fn get_token_program_from_mint(mint: &Pubkey, config: &Config) -> eyre::Result<Pubkey> {
    let rpc_client = RpcClient::new(config.url.clone());
    let mint_account = rpc_client.get_account(mint)?;
    Ok(mint_account.owner)
}

pub(crate) fn get_associated_token_address(
    wallet_address: &Pubkey,
    token_mint_address: &Pubkey,
    token_program_id: &Pubkey,
) -> Pubkey {
    let associated_token_program_id = spl_associated_token_account_program_id();
    Pubkey::find_program_address(
        &[
            wallet_address.as_ref(),
            token_program_id.as_ref(),
            token_mint_address.as_ref(),
        ],
        &associated_token_program_id,
    )
    .0
}

fn spl_associated_token_account_program_id() -> Pubkey {
    solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
}

fn find_its_root_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"interchain-token-service"], &solana_axelar_its::id())
}

fn find_token_manager_pda(its_root_pda: &Pubkey, token_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"token-manager", its_root_pda.as_ref(), token_id],
        &solana_axelar_its::id(),
    )
}

pub(crate) fn derive_keypairs_from_mnemonic(
    mnemonic: &str,
    count: usize,
) -> eyre::Result<Vec<Arc<dyn Signer + Send + Sync>>> {
    use solana_sdk::signature::keypair_from_seed;

    let seed = bip39::Mnemonic::parse(mnemonic)
        .map_err(|e| eyre!("Invalid mnemonic: {}", e))?
        .to_seed("");

    let mut keypairs: Vec<Arc<dyn Signer + Send + Sync>> = Vec::with_capacity(count);

    for i in 0..count {
        let derivation_path = format!("m/44'/501'/{i}'");
        let derived_key = derive_key_from_seed(&seed, &derivation_path)?;
        let keypair = keypair_from_seed(&derived_key[..32])
            .map_err(|e| eyre!("Failed to create keypair: {}", e))?;
        keypairs.push(Arc::new(keypair));
    }

    Ok(keypairs)
}

#[allow(clippy::big_endian_bytes, clippy::missing_asserts_for_indexing)]
fn derive_key_from_seed(seed: &[u8], path: &str) -> eyre::Result<[u8; 64]> {
    use hmac::Hmac;
    use hmac::Mac;
    use sha2::Sha512;

    let mut hmac = Hmac::<Sha512>::new_from_slice(b"ed25519 seed")
        .map_err(|e| eyre!("HMAC initialization failed: {}", e))?;
    hmac.update(seed);
    let result = hmac.finalize();
    let bytes = result.into_bytes();

    if bytes.len() <= 63 {
        return Err(eyre!("HMAC output too short"));
    }
    let mut key = [0u8; 64];
    key[..32].copy_from_slice(&bytes[..32]);
    key[32..].copy_from_slice(&bytes[32..64]);

    let parts: Vec<&str> = path.split('/').collect();
    for (i, part) in parts.iter().enumerate() {
        if i == 0 && *part == "m" {
            continue;
        }

        let hardened = part.ends_with('\'');
        let index_str = part.trim_end_matches('\'');
        let index: u32 = index_str
            .parse()
            .map_err(|_| eyre!("Invalid derivation path index: {}", part))?;

        let child_index = if hardened { 0x8000_0000 | index } else { index };

        let mut data = Vec::with_capacity(37);
        data.push(0);
        data.extend_from_slice(&key[..32]);
        data.extend_from_slice(&child_index.to_be_bytes());

        let mut hmac = Hmac::<Sha512>::new_from_slice(&key[32..64])
            .map_err(|e| eyre!("HMAC initialization failed: {}", e))?;
        hmac.update(&data);
        let result = hmac.finalize();
        let bytes = result.into_bytes();

        if bytes.len() <= 63 {
            return Err(eyre!("HMAC output too short"));
        }
        key[..32].copy_from_slice(&bytes[..32]);
        key[32..].copy_from_slice(&bytes[32..64]);
    }

    Ok(key)
}
