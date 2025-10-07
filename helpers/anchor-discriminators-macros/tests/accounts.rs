#![cfg(test)]
#![allow(clippy::indexing_slicing)]

pub(crate) mod v1_tests {
    use anchor_discriminators::Discriminator;
    use anchor_discriminators_macros::account;
    use borsh::{from_slice, to_vec};
    use bytemuck::{bytes_of, from_bytes, Pod, Zeroable};
    use program_utils::pda::BytemuckedPda;
    use solana_program::pubkey::Pubkey;

    solana_program::declare_id!("gtwi5T9x6rTWPtuuz6DA7ia1VmH8bdazm9QfDdi6DVp");

    /// Keep track of the gas collector for aggregating gas payments
    #[repr(C)]
    #[account]
    #[derive(Zeroable, Pod, Clone, Copy, PartialEq, Eq, Debug)]
    pub(crate) struct Config {
        /// Operator with permission to give refunds & withdraw funds
        pub operator: Pubkey,
        /// The bump seed used to derive the PDA, ensuring the address is valid.
        pub bump: u8,
    }

    impl BytemuckedPda for Config {}

    #[account]
    #[derive(Debug, Eq, PartialEq, Clone)]
    /// Struct containing flow information for a specific epoch.
    pub(crate) struct FlowState {
        pub flow_limit: Option<u64>,
        pub flow_in: u64,
        pub flow_out: u64,
        pub epoch: u64,
    }

    #[test]
    fn test_account_bytemuck_init() {
        let operator = Pubkey::new_unique();
        let bump = 1;

        // Create a buffer like an actual account would have
        let mut data = vec![0u8; Config::pda_size()];

        // Initialize with discriminator
        let config_ref = Config::init_mut(&mut data).unwrap();
        *config_ref = Config { operator, bump };

        // Verify discriminator was written
        assert_eq!(&data[..8], Config::DISCRIMINATOR);

        // Verify we can read it back
        let read_config = Config::read(&data).unwrap();
        assert_eq!(read_config.operator, operator);
        assert_eq!(read_config.bump, bump);

        // Verify we can mutate fields
        let config_mut = Config::read_mut(&mut data).unwrap();
        let new_operator = Pubkey::new_unique();
        config_mut.operator = new_operator;

        // Verify mutation worked
        let read_config = Config::read(&data).unwrap();
        assert_eq!(read_config.operator, new_operator);
        assert_eq!(read_config.bump, bump);
    }

    #[test]
    fn test_account_serde() {
        let flow = FlowState {
            flow_limit: Some(100),
            flow_in: 50,
            flow_out: 30,
            epoch: 1,
        };
        let bytes = to_vec(&flow).unwrap();
        assert_eq!(&bytes[..8], FlowState::DISCRIMINATOR);
        let deserialized: FlowState = from_slice(&bytes).unwrap();
        assert_eq!(flow, deserialized);
    }
}

// Defining it here since Anchor uses `crate::ID` inside expanded code
anchor_lang::declare_id!("gtwi5T9x6rTWPtuuz6DA7ia1VmH8bdazm9QfDdi6DVp");

use borsh as v1_borsh;
use solana_program::pubkey::Pubkey;

pub(crate) mod compat_tests {
    use super::{v1_borsh, Pubkey};
    use crate::v1_tests;
    use anchor_discriminators::Discriminator as V1Discriminator;
    use anchor_lang::{
        prelude::{
            account, borsh, zero_copy, AccountSerialize, AnchorDeserialize, AnchorSerialize,
        },
        Discriminator,
    };
    use bytemuck::bytes_of;

    #[account(zero_copy)]
    pub(crate) struct Config {
        /// Operator with permission to give refunds & withdraw funds
        pub operator: Pubkey,
        /// The bump seed used to derive the PDA, ensuring the address is valid.
        pub bump: u8,
    }

    #[account]
    /// Struct containing flow information for a specific epoch.
    pub(crate) struct FlowState {
        pub flow_limit: Option<u64>,
        pub flow_in: u64,
        pub flow_out: u64,
        pub epoch: u64,
    }

    #[test]
    fn test_matches_v1_bytemuck() {
        assert_eq!(v1_tests::Config::DISCRIMINATOR, Config::DISCRIMINATOR);

        let operator = Pubkey::new_unique();

        let bump = 1;

        let v1_config = v1_tests::Config { operator, bump };
        let v2_config = Config { operator, bump };

        assert_eq!(bytes_of(&v1_config), bytes_of(&v2_config));
    }

    #[test]
    fn test_matches_v1() {
        assert_eq!(v1_tests::FlowState::DISCRIMINATOR, FlowState::DISCRIMINATOR);

        let v1_flow = v1_tests::FlowState {
            flow_limit: Some(100),
            flow_in: 50,
            flow_out: 30,
            epoch: 1,
        };
        let v2_flow = FlowState {
            flow_limit: Some(100),
            flow_in: 50,
            flow_out: 30,
            epoch: 1,
        };

        let mut v2_bytes = vec![];
        v2_flow.try_serialize(&mut v2_bytes).unwrap();

        assert_eq!(v1_borsh::to_vec(&v1_flow).unwrap(), v2_bytes);
    }
}
