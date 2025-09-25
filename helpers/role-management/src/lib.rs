//! Role management crate for the Solana blockchain.
use solana_program::pubkey::Pubkey;

pub mod processor;
pub mod state;

/// Seed prefixes for different PDAs initialized by the program
pub mod seed_prefixes {
    /// The seed prefix for deriving the user roles PDA
    pub const USER_ROLES_SEED: &[u8] = b"user-roles";

    /// The seed prefix for deriving the role proposal PDA
    pub const ROLE_PROPOSAL_SEED: &[u8] = b"role-proposal";
}

/// Tries to create the PDA for `UserRoles` using the provided bump,
/// falling back to `find_program_address` if the bump is `None` or invalid.
#[must_use]
pub fn user_roles_pda(
    program_id: &Pubkey,
    resource: &Pubkey,
    user: &Pubkey,
    maybe_bump: Option<u8>,
) -> (Pubkey, u8) {
    maybe_bump
        .and_then(|bump| {
            Pubkey::create_program_address(
                &[
                    seed_prefixes::USER_ROLES_SEED,
                    resource.as_ref(),
                    user.as_ref(),
                    &[bump],
                ],
                program_id,
            )
            .map(|pubkey| (pubkey, bump))
            .ok()
        })
        .unwrap_or_else(|| {
            Pubkey::find_program_address(
                &[
                    seed_prefixes::USER_ROLES_SEED,
                    resource.as_ref(),
                    user.as_ref(),
                ],
                program_id,
            )
        })
}

/// Derives the PDA for a `UserRoles` account.
#[inline]
#[must_use]
pub fn find_user_roles_pda(program_id: &Pubkey, resource: &Pubkey, user: &Pubkey) -> (Pubkey, u8) {
    user_roles_pda(program_id, resource, user, None)
}

/// Tries to create the PDA for `RolesProposal` using the provided bump,
/// falling back to `find_program_address` if the bump is `None` or invalid.
#[must_use]
pub fn roles_proposal_pda<F: crate::state::RolesFlags>(
    program_id: &Pubkey,
    resource: &Pubkey,
    from: &Pubkey,
    to: &Pubkey,
    roles: F,
    maybe_bump: Option<u8>,
) -> (Pubkey, u8) {
    let roles_bytes = borsh::to_vec(&roles.bits())
        .expect("No obvious reason why serializing bits should fail. It's a bug.");
    maybe_bump
        .and_then(|bump| {
            Pubkey::create_program_address(
                &[
                    seed_prefixes::ROLE_PROPOSAL_SEED,
                    resource.as_ref(),
                    from.as_ref(),
                    to.as_ref(),
                    &roles_bytes,
                    &[bump],
                ],
                program_id,
            )
            .map(|pubkey| (pubkey, bump))
            .ok()
        })
        .unwrap_or_else(|| {
            Pubkey::find_program_address(
                &[
                    seed_prefixes::ROLE_PROPOSAL_SEED,
                    resource.as_ref(),
                    from.as_ref(),
                    to.as_ref(),
                    &roles_bytes,
                ],
                program_id,
            )
        })
}

/// Tries to create the PDA for `RolesProposal` using the provided bump,
/// falling back to `find_program_address` if the bump is invalid.
#[inline]
#[must_use]
pub fn create_roles_proposal_pda<F: crate::state::RolesFlags>(
    program_id: &Pubkey,
    resource: &Pubkey,
    from: &Pubkey,
    to: &Pubkey,
    roles: F,
    bump: u8,
) -> (Pubkey, u8) {
    roles_proposal_pda(program_id, resource, from, to, roles, Some(bump))
}

/// Derives the PDA for a `RolesProposal` account.
#[inline]
#[must_use]
pub fn find_roles_proposal_pda<F: crate::state::RolesFlags>(
    program_id: &Pubkey,
    resource: &Pubkey,
    from: &Pubkey,
    to: &Pubkey,
    roles: F,
) -> (Pubkey, u8) {
    roles_proposal_pda(program_id, resource, from, to, roles, None)
}
