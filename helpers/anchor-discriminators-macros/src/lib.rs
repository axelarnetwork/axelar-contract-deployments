//! Derive macro for generating instruction discriminators for enums.
extern crate proc_macro;

use convert_case::{Case, Casing};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Token,
};

use anchor_discriminators::{sighash, SIGHASH_GLOBAL_NAMESPACE};

// https://github.com/solana-foundation/anchor/blob/56b21edd1f4c1865e5f943537fb7f89a0ffe5ede/lang/syn/src/codegen/program/common.rs#L21
fn gen_discriminator(namespace: &str, name: impl ToString) -> proc_macro2::TokenStream {
    let discriminator = sighash(namespace, name.to_string().as_str());
    // NOTE: keep in mind this is missing the leading &
    // add if needed
    format!("{discriminator:?}").parse().unwrap()
}

/// Derive macro that generates 8-byte instruction discriminators for enums with unit and named field variants.
///
/// This macro automatically:
/// - Generates unique 8-byte discriminators for each enum variant
/// - Implements custom `BorshSerialize` that writes discriminator + field data
/// - Implements custom `BorshDeserialize` that reads discriminator + field data
/// - Creates a `discriminators` module with constants for each variant
///
/// # Supported Variant Types
/// - Unit variants: `Initialize`
/// - Named field variants: `Transfer { amount: u64, recipient: Pubkey }`
///
///
/// ```ignore
/// #[derive(InstructionDiscriminator)]
/// pub enum MyInstruction {
///     Initialize,
///     Transfer { amount: u64, recipient: Pubkey },
///     Close,
/// }
/// ```
#[allow(clippy::too_many_lines)]
#[proc_macro_derive(InstructionDiscriminator)]
pub fn derive_instruction_discriminator(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    // Only support enums
    let enum_data = match &input.data {
        syn::Data::Enum(data) => data,
        syn::Data::Struct(_) | syn::Data::Union(_) => {
            return syn::Error::new_spanned(
                input,
                "InstructionDiscriminator can only be derived for enums",
            )
            .to_compile_error()
            .into();
        }
    };

    let enum_name = &input.ident;
    let enum_vis = &input.vis;

    // Generate discriminator constants and match arms
    let mut discriminator_constants = Vec::new();
    let mut discriminator_match_arms = Vec::new();
    let mut serialize_match_arms = Vec::new();
    let mut deserialize_match_arms = Vec::new();

    for variant in &enum_data.variants {
        let variant_name = &variant.ident;
        let variant_name_snake = variant_name.to_string().to_case(Case::Snake);
        let variant_name_constant = variant_name.to_string().to_case(Case::Constant);
        let const_name = syn::Ident::new(&variant_name_constant, variant.ident.span());

        // Generate discriminator constant
        let discriminator = gen_discriminator(SIGHASH_GLOBAL_NAMESPACE, &variant_name_snake);

        discriminator_constants.push(quote! {
            #[doc = concat!("Discriminator for ", stringify!(#variant_name))]
            #[doc = concat!("sha256(global::", #variant_name_snake, ")[..8]")]
            pub const #const_name: [u8; 8] = #discriminator;
        });

        match &variant.fields {
            // Unit variant: Initialize
            syn::Fields::Unit => {
                discriminator_match_arms.push(quote! {
                    #[doc = concat!("Discriminator for ", stringify!(#variant_name))]
                    #[doc = concat!("sha256(global::", #variant_name_snake, ")[..8]")]
                    Self::#variant_name => &discriminators::#const_name
                });

                serialize_match_arms.push(quote! {
                    Self::#variant_name => {
                        writer.write_all(&discriminators::#const_name)?;
                    }
                });

                deserialize_match_arms.push(quote! {
                    discriminators::#const_name => Ok(Self::#variant_name)
                });
            }

            // Named fields variant: Transfer { amount: u64, recipient: Pubkey }
            syn::Fields::Named(fields) => {
                // Extract field names and types for serialization
                let field_names: Vec<_> = fields
                    .named
                    .iter()
                    .map(|f| {
                        f.ident
                            .as_ref()
                            .expect("Named fields must have identifiers")
                    })
                    .collect();
                let field_types: Vec<_> = fields.named.iter().map(|f| &f.ty).collect();

                discriminator_match_arms.push(quote! {
                    #[doc = concat!("Discriminator for ", stringify!(#variant_name))]
                    #[doc = concat!("sha256(global::", #variant_name_snake, ")[..8]")]
                    Self::#variant_name {..} => &discriminators::#const_name
                });

                // For serialization, we need to serialize each field
                serialize_match_arms.push(quote! {
                    Self::#variant_name { #(#field_names),* } => {
                        writer.write_all(&discriminators::#const_name)?;
                        #(#field_names.serialize(writer)?;)*
                    }
                });

                // For deserialization, we need to deserialize each field
                deserialize_match_arms.push(quote! {
                    discriminators::#const_name => {
                        #(
                            let #field_names = <#field_types>::deserialize_reader(reader)?;
                        )*
                        Ok(Self::#variant_name { #(#field_names),* })
                    }
                });
            }

            // We only support single unnamed field variant: WrapperType(Type)
            syn::Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    return syn::Error::new_spanned(
                        variant,
                        "InstructionDiscriminator only supports a single unnamed field.",
                    )
                    .to_compile_error()
                    .into();
                }

                discriminator_match_arms.push(quote! {
                    #[doc = concat!("Discriminator for ", stringify!(#variant_name))]
                    #[doc = concat!("sha256(global::", #variant_name_snake, ")[..8]")]
                    Self::#variant_name(..) => &discriminators::#const_name
                });

                serialize_match_arms.push(quote! {
                    Self::#variant_name(data) => {
                        writer.write_all(&discriminators::#const_name)?;
                        data.serialize(writer)?;
                    }
                });

                deserialize_match_arms.push(quote! {
                    discriminators::#const_name => {
                        let data = borsh::BorshDeserialize::deserialize_reader(reader)?;
                        Ok(Self::#variant_name(data))
                    }
                });
            }
        }
    }

    let expanded = quote! {
        #[doc = concat!("Discriminators for ", stringify!(#enum_name))]
        #enum_vis mod discriminators {
            #(#discriminator_constants)*
        }

        impl #enum_name {
            /// Get the discriminator for this instruction variant
            pub fn discriminator(&self) -> &'static [u8; 8] {
                match self {
                    #(#discriminator_match_arms,)*
                }
            }
        }

        impl borsh::BorshSerialize for #enum_name {
            fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                match self {
                    #(#serialize_match_arms)*
                }
                Ok(())
            }
        }

        impl borsh::BorshDeserialize for #enum_name {
            fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
                let mut discriminator = [0u8; 8];
                reader.read_exact(&mut discriminator)?;

                match discriminator {
                    #(#deserialize_match_arms,)*
                    _ => Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Unknown {} discriminator: {:?}",  stringify!(#enum_name), discriminator),
                    )),
                }
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

// Define a struct to parse the attribute arguments
#[derive(Debug, Default)]
struct AccountArgs {
    zero_copy: bool,
}

impl Parse for AccountArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut zero_copy = false;

        // Parse comma-separated idents
        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            if ident == "zero_copy" {
                zero_copy = true;
            } else {
                return Err(syn::Error::new_spanned(
                    ident,
                    "Unknown argument. Valid arguments: zero_copy",
                ));
            }

            // Parse trailing comma if present
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(AccountArgs { zero_copy })
    }
}

#[proc_macro_attribute]
#[allow(clippy::wildcard_enum_match_arm)]
pub fn account(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let account_strct = parse_macro_input!(input as syn::ItemStruct);
    let account_name = &account_strct.ident;

    // Parse arguments using our custom Parse implementation
    let args = if args.is_empty() {
        AccountArgs::default()
    } else {
        parse_macro_input!(args as AccountArgs)
    };
    let is_zero_copy = args.zero_copy;

    let (impl_gen, type_gen, where_clause) = account_strct.generics.split_for_impl();

    let discriminator = gen_discriminator(
        anchor_discriminators::SIGHASH_ACCOUNT_NAMESPACE,
        account_name,
    );

    // Extract field names and types for serialization/deserialization
    let (field_names, field_types): (Vec<_>, Vec<_>) = match &account_strct.fields {
        syn::Fields::Named(fields) => fields.named.iter().map(|f| (&f.ident, &f.ty)).unzip(),
        _ => {
            return syn::Error::new_spanned(
                account_strct,
                "account only supports structs with named fields",
            )
            .to_compile_error()
            .into();
        }
    };

    let borsh_impls = if is_zero_copy {
        quote!()
    } else {
        quote! {
            #[automatically_derived]
            impl #impl_gen borsh::BorshSerialize for #account_name #type_gen #where_clause {
                fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                    writer.write_all(Self::DISCRIMINATOR)?;
                    #(borsh::BorshSerialize::serialize(&self.#field_names, writer)?;)*
                    Ok(())
                }
            }

            #[automatically_derived]
            impl #impl_gen borsh::BorshDeserialize for #account_name #type_gen #where_clause {
                fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
                    // Read and verify discriminator
                    let mut discriminator = [0u8; 8];
                    reader.read_exact(&mut discriminator)?;

                    if discriminator != Self::DISCRIMINATOR {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!(
                                "Invalid account discriminator for {}: expected {:?}, got {:?}",
                                stringify!(#account_name),
                                Self::DISCRIMINATOR,
                                discriminator
                            ),
                        ));
                    }

                    // Deserialize each field
                    #(
                        let #field_names = <#field_types as borsh::BorshDeserialize>::deserialize_reader(reader)?;
                    )*

                    Ok(Self {
                        #(#field_names),*
                    })
                }
            }
        }
    };

    let ret = quote! {
        #account_strct

        #[automatically_derived]
        impl #impl_gen anchor_discriminators::Discriminator for #account_name #type_gen #where_clause {
            const DISCRIMINATOR: &'static [u8] = &#discriminator;
        }

        #borsh_impls
    };

    #[allow(unreachable_code)]
    proc_macro::TokenStream::from(ret)
}
