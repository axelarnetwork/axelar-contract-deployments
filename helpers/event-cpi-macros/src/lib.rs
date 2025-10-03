#![cfg(not(doctest))]

extern crate proc_macro;

use anchor_discriminators::sighash;
use quote::quote;
use syn::{parse_macro_input, token::Colon};

// https://github.com/solana-foundation/anchor/blob/56b21edd1f4c1865e5f943537fb7f89a0ffe5ede/lang/syn/src/codegen/program/common.rs#L21
fn gen_discriminator(namespace: &str, name: impl ToString) -> proc_macro2::TokenStream {
    let discriminator = sighash(namespace, name.to_string().as_str());
    format!("&{discriminator:?}").parse().unwrap()
}

/// Attribute macro that transforms a struct into an event that can be emitted via CPI.
///
/// This macro automatically:
/// - Adds `BorshSerialize` and `BorshDeserialize` derives
/// - Implements `event_cpi::CpiEvent` trait with proper data serialization
/// - Implements `event_cpi::Discriminator` trait with a computed 8-byte discriminator
///
/// # External Dependencies
/// - Requires `event_cpi` crate to be available
/// - Requires `borsh` crate for serialization
///
/// # Example
/// ```ignore
/// #[event]
/// #[derive(Debug, Clone)]
/// pub struct MyEvent {
///     pub user: Pubkey,
///     pub amount: u64,
/// }
/// ```
// https://github.com/solana-foundation/anchor/blob/d5d7eb97979234eb1e9e32fcef66ce171a928b62/lang/attribute/event/src/lib.rs#L32
#[proc_macro_attribute]
pub fn event(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let event_strct = parse_macro_input!(input as syn::ItemStruct);
    let event_name = &event_strct.ident;

    let discriminator = gen_discriminator(event_cpi::SIGHASH_EVENT_NAMESPACE, event_name);

    let ret = quote! {
        #[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
        #event_strct

        impl event_cpi::CpiEvent for #event_name {
            fn data(&self) -> Vec<u8> {
                use borsh::BorshSerialize;

                let mut data = Vec::with_capacity(256);
                data.extend_from_slice(#event_name::DISCRIMINATOR);
                self.serialize(&mut data).unwrap();
                data
            }
        }

        impl anchor_discriminators::Discriminator for #event_name {
            const DISCRIMINATOR: &'static [u8] = #discriminator;
        }
    };

    #[allow(unreachable_code)]
    proc_macro::TokenStream::from(ret)
}

/// Function-like macro that extracts and validates event CPI accounts from an account iterator.
///
/// This macro consumes the next two accounts from the provided iterator and validates them:
/// 1. Event authority account (must match the derived PDA)
/// 2. Program account (must match the current program ID)
///
/// # Arguments
/// - `accounts_iterator_name` (optional): Name of the accounts iterator variable
///   - Default: `accounts` if not provided
///   - Type: `&mut Iterator<Item = &AccountInfo>`
///
/// # External Dependencies
/// - Requires `crate::ID` to be defined (current program's ID)
/// - Requires `event_cpi` crate
/// - Requires `solana_program` crate
///
/// # Variables Created in Scope
/// - `__event_cpi_authority_info: &AccountInfo` - The event authority account
/// - `__event_cpi_program_account: &AccountInfo` - The program account
/// - `__event_cpi_derived_authority_info: Pubkey` - The expected authority PDA
/// - `__event_cpi_authority_bump: u8` - The bump seed for the authority PDA
///
/// # Example
/// ```ignore
/// let accounts = &mut accounts.iter();
/// event_cpi_accounts!(accounts);
/// // or with default iterator name:
/// event_cpi_accounts!();
/// ```
// https://github.com/solana-foundation/anchor/blob/d5d7eb97979234eb1e9e32fcef66ce171a928b62/lang/syn/src/parser/accounts/event_cpi.rs#L28
#[proc_macro]
pub fn event_cpi_accounts(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input to get the accounts list name (optional)
    let accounts_list_name = if input.is_empty() {
        // Default to "accounts" if no argument provided
        quote::format_ident!("accounts")
    } else {
        // Parse the provided identifier
        let accounts_ident = parse_macro_input!(input as syn::Ident);
        accounts_ident
    };

    proc_macro::TokenStream::from(quote! {
        let __event_cpi_authority_info = solana_program::account_info::next_account_info(#accounts_list_name)?;
        let __event_cpi_program_account = solana_program::account_info::next_account_info(#accounts_list_name)?;

        let (__event_cpi_derived_authority_info, __event_cpi_authority_bump) =
            solana_program::pubkey::Pubkey::find_program_address(&[event_cpi::EVENT_AUTHORITY_SEED], &crate::ID);

        // Check that the event authority public key matches
        if *__event_cpi_authority_info.key != __event_cpi_derived_authority_info {
            return Err(solana_program::program_error::ProgramError::InvalidAccountData);
        }

        if *__event_cpi_program_account.key != crate::ID {
            return Err(solana_program::program_error::ProgramError::IncorrectProgramId);
        }
    })
}

/// Function-like macro that emits an event via Cross-Program Invocation (CPI).
///
/// This macro creates a CPI instruction to emit the provided event. The event data is serialized
/// with the event discriminator prefix and sent as a self-invoke to the current program.
///
/// # Arguments
/// - `event_expression` (required): An expression that evaluates to a struct implementing `CpiEvent`
///   - Type: Any type implementing `event_cpi::CpiEvent`
///
/// # External Dependencies
/// - Requires `crate::ID` to be defined (current program's ID)
/// - Requires `event_cpi`
/// - Requires `solana_program` crate
///
/// # Variables Required in Scope
/// These variables must be available in the current scope (typically created by `event_cpi_accounts!`):
/// - `__event_cpi_authority_info: &AccountInfo` - The event authority account
/// - `__event_cpi_authority_bump: u8` - The bump seed for authority PDA
///
/// # Example
/// ```ignore
/// let event = MyEvent {
///     user: *user_account.key,
///     amount: 1000,
/// };
/// emit_cpi!(event);
/// ```
// https://github.com/solana-foundation/anchor/blob/d5d7eb97979234eb1e9e32fcef66ce171a928b62/lang/attribute/event/src/lib.rs#L157
#[proc_macro]
pub fn emit_cpi(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let event_struct = parse_macro_input!(input as syn::Expr);

    proc_macro::TokenStream::from(quote! {
    {
        // 1. Assumes these two values are in scope from event_cpi_accounts! macro
        // __event_cpi_authority_info
        // __event_cpi_authority_bump

        let __event_cpi_inner_data = event_cpi::CpiEvent::data(&#event_struct);
        let __event_cpi_ix_data: Vec<u8> = event_cpi::EVENT_IX_TAG_LE
            .into_iter()
            .map(|b| *b)
            .chain(__event_cpi_inner_data.into_iter())
            .collect();

        // 2. construct the instruction (non-anchor style)
        let __event_cpi_ix = solana_program::instruction::Instruction::new_with_bytes(
            crate::ID,
            &__event_cpi_ix_data,
            vec![
                solana_program::instruction::AccountMeta::new_readonly(
                    *__event_cpi_authority_info.key,
                    true,
                ),
            ],
        );
        // 3. invoke_signed the instruction
        solana_program::program::invoke_signed(
            &__event_cpi_ix,
            &[__event_cpi_authority_info.clone()],
            &[&[event_cpi::EVENT_AUTHORITY_SEED, &[__event_cpi_authority_bump]]],
        )?;
    }
    })
}

/// Attribute macro that automatically adds event CPI account fields and implements the `EventAccounts` trait.
///
/// This macro:
/// 1. Adds two fields to the struct:
///    - `__event_cpi_authority_info: &'a AccountInfo<'a>`
///    - `__event_cpi_program_account: &'a AccountInfo<'a>`
/// 2. Implements the `EventAccounts` trait for the struct
///
/// The fields are added as the last two fields, unless a `remaining_accounts` field exists,
/// in which case they are added before it.
///
/// # Requirements
/// - Struct must have a lifetime parameter `'a` if it holds references
///
/// # Example
/// ```ignore
/// #[event_cpi]
/// pub struct MyAccounts<'a> {
///     pub user: &'a AccountInfo<'a>,
///     pub token: &'a AccountInfo<'a>,
/// }
/// ```
///
/// Expands to:
/// ```ignore
/// pub struct MyAccounts<'a> {
///     pub user: &'a AccountInfo<'a>,
///     pub token: &'a AccountInfo<'a>,
///     pub __event_cpi_authority_info: &'a AccountInfo<'a>,
///     pub __event_cpi_program_account: &'a AccountInfo<'a>,
/// }
///
/// impl<'a> EventAccounts<'a> for MyAccounts<'a> {
///     fn event_accounts(&self) -> [&'a AccountInfo<'a>; 2] {
///         [
///             self.__event_cpi_authority_info,
///             self.__event_cpi_program_account,
///         ]
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn event_cpi(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut input_struct = parse_macro_input!(input as syn::ItemStruct);
    let struct_name = &input_struct.ident;

    // Extract generics - we expect a lifetime 'a
    let generics = &input_struct.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Add the two event CPI fields to the struct
    if let syn::Fields::Named(ref mut fields) = input_struct.fields {
        // Find if there's a remaining_accounts field
        let remaining_accounts_pos = fields
            .named
            .iter()
            .position(|f| f.ident.as_ref().is_some_and(|i| i == "remaining_accounts"));

        // Create the two new fields we want to add
        let account_info_type: syn::Type = syn::parse2(quote! {
            &'a solana_program::account_info::AccountInfo<'a>
        })
        .unwrap();

        let authority_field = syn::Field {
            attrs: vec![],
            vis: syn::Visibility::Inherited,
            ident: Some(syn::Ident::new(
                "__event_cpi_authority_info",
                proc_macro2::Span::call_site(),
            )),
            colon_token: Some(Colon::default()),
            ty: account_info_type.clone(),
        };

        let program_field = syn::Field {
            attrs: vec![],
            vis: syn::Visibility::Inherited,
            ident: Some(syn::Ident::new(
                "__event_cpi_program_account",
                proc_macro2::Span::call_site(),
            )),
            colon_token: Some(Colon::default()),
            ty: account_info_type,
        };

        // Insert before remaining_accounts if it exists, otherwise append
        if let Some(pos) = remaining_accounts_pos {
            fields.named.insert(pos, authority_field);
            fields.named.insert(pos + 1, program_field);
        } else {
            fields.named.push(authority_field);
            fields.named.push(program_field);
        }
    }

    // Generate the implementation
    let expanded = quote! {
        #input_struct

        impl #impl_generics event_cpi::EventAccounts<'a> for #struct_name #ty_generics #where_clause {
            fn event_accounts(&self) -> [&'a solana_program::account_info::AccountInfo<'a>; 2] {
                [
                    self.__event_cpi_authority_info,
                    self.__event_cpi_program_account,
                ]
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

/// Function-like macro that handles incoming event CPI instructions in your program's processor.
///
/// This macro should be placed at the beginning of your instruction processor to intercept
/// and handle event CPI calls. When an instruction starts with the event tag, it validates
/// the event authority and returns early, preventing further instruction processing.
///
/// # Arguments
/// - `instruction_data_name` (optional): Name of the instruction data variable
///   - Default: `instruction_data` if not provided
///   - Type: `&[u8]` - The raw instruction data bytes
///
/// # External Dependencies
/// - Requires `event_cpi` crate
/// - Requires `solana_program` crate
///
/// # Variables Required in Scope
/// - `program_id: &Pubkey` - The current program's ID
/// - `accounts: &[AccountInfo]` - The accounts passed to the instruction
/// - The instruction data variable (default name: `instruction_data`)
///
/// # Behavior
/// - If instruction data starts with event tag: validates authority and returns `Ok(())`
/// - If instruction data doesn't match: continues normal execution (no early return)
///
/// # Example
/// ```ignore
/// pub fn process_instruction(
///     program_id: &Pubkey,
///     accounts: &[AccountInfo],
///     instruction_data: &[u8],
/// ) -> ProgramResult {
///     event_cpi_handler!(instruction_data);
///
///     // Your normal instruction processing continues here...
///     let instruction = MyInstruction::try_from_slice(instruction_data)?;
///     // ...
/// }
/// ```
// https://github.com/solana-foundation/anchor/blob/5300d7cf8aaf52da08ce331db3fc8182cd821228/lang/syn/src/codegen/program/handlers.rs#L213
#[proc_macro]
pub fn event_cpi_handler(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input to get the accounts list name (optional)
    let instruction_data_name = if input.is_empty() {
        // Default to "instruction_data" if no argument provided
        quote::format_ident!("instruction_data")
    } else {
        // Parse the provided identifier
        let data_ident = parse_macro_input!(input as syn::Ident);
        data_ident
    };

    proc_macro::TokenStream::from(quote! {
        // Dispatch Event CPI instruction
        if #instruction_data_name.starts_with(event_cpi::EVENT_IX_TAG_LE) {
            solana_program::msg!("EventCpiInstruction");

            let given_event_authority = solana_program::account_info::next_account_info(&mut accounts.iter())?;
            if !given_event_authority.is_signer {
                return Err(solana_program::program_error::ProgramError::MissingRequiredSignature);
            }

            let (expected_event_authority, _) =
                solana_program::pubkey::Pubkey::find_program_address(&[event_cpi::EVENT_AUTHORITY_SEED], program_id);

            if *given_event_authority.key != expected_event_authority {
                return Err(solana_program::program_error::ProgramError::InvalidAccountData);
            }

            // Early return
            return Ok(())
        }
    })
}
