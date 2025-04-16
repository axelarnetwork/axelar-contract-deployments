//! This crate provides a procedural macro for deriving the [`event_utils::Event`] trait for structs.
//!
//! The `Event` is emitted and parsed differently compared to how Anchor does it, where the
//! structures are serialized and deserialized with `Borsh`. Here we simply use `sol_log_data` to
//! log each field as bytes.
use keccak_const::Shake128;
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, Error, Fields, GenericArgument, PathArguments, Type,
    TypeArray,
};

fn get_u8_array_size(ty: &Type) -> Option<usize> {
    let Type::Array(TypeArray { elem, len, .. }) = ty else {
        return None;
    };
    let Type::Path(type_path) = elem.as_ref() else {
        return None;
    };

    if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "u8" {
        if let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(lit_int),
            ..
        }) = len
        {
            lit_int.base10_parse::<usize>().ok()
        } else {
            None
        }
    } else {
        None
    }
}

fn is_vec_u8(ty: &Type) -> bool {
    let Type::Path(type_path) = ty else {
        return false;
    };
    let Some(segment) = type_path.path.segments.last() else {
        return false;
    };
    if segment.ident != "Vec" {
        return false;
    }
    let PathArguments::AngleBracketed(ref args) = segment.arguments else {
        return false;
    };
    if args.args.len() != 1 {
        return false;
    }
    let GenericArgument::Type(ref inner_ty) = args.args[0] else {
        return false;
    };
    let Type::Path(ref inner_path) = inner_ty else {
        return false;
    };
    let segments = &inner_path.path.segments;
    segments.len() == 1 && segments[0].ident == "u8"
}

fn get_simple_type_ident_str(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(ref type_path) if !type_path.path.segments.is_empty() => {
            Some(type_path.path.segments.last().unwrap().ident.to_string())
        }
        _ => None,
    }
}

#[proc_macro_derive(Event)]
pub fn derive_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_ident = &input.ident;

    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            _ => {
                return Error::new_spanned(
                    &input.ident,
                    "Event requires a struct with named fields",
                )
                .to_compile_error()
                .into()
            }
        },
        _ => {
            return Error::new_spanned(&input.ident, "Event can only be derived for structs")
                .to_compile_error()
                .into()
        }
    };

    let type_name_str = struct_ident.to_string();
    let discriminant: [u8; 16] = Shake128::new().update(type_name_str.as_bytes()).finalize();
    let discriminant_tokens = quote! { &[ #(#discriminant),* ] };

    let mut emit_slices = Vec::new();
    emit_slices.push(quote! { Self::DISC });

    for field in fields {
        let field_ident = field.ident.as_ref().unwrap();
        let ty = &field.ty;

        let slice_expr = if get_u8_array_size(ty).is_some() || is_vec_u8(ty) {
            quote! { &self.#field_ident[..] }
        } else if let Some(type_name) = get_simple_type_ident_str(ty) {
            match type_name.as_str() {
                "Pubkey" => quote! { self.#field_ident.as_ref() },
                "String" => quote! { self.#field_ident.as_bytes() },
                "u8" | "u16" | "u32" | "u64" | "u128" | "i8" | "i16" | "i32" | "i64" | "i128"
                | "bool" | "f32" | "f64" | "U256" => {
                    quote! { &(self.#field_ident.to_le_bytes()[..]) }
                }
                _ => {
                    return Error::new_spanned(
                        ty,
                        format!("Unsupported simple type '{type_name}' for emit."),
                    )
                    .to_compile_error()
                    .into()
                }
            }
        } else {
            return Error::new_spanned(ty, "Unsupported field type for emit.")
                .to_compile_error()
                .into();
        };
        emit_slices.push(slice_expr);
    }

    let emit_impl = quote! {
        fn emit(&self) {
            ::solana_program::log::sol_log_data(&[
                #(#emit_slices),*
            ]);
        }
    };

    let mut deserialize_steps = Vec::new();
    let mut field_idents_for_struct = Vec::new();

    for field in fields.iter() {
        let field_ident = field.ident.as_ref().unwrap();
        field_idents_for_struct.push(field_ident.clone());
        let field_name_str = field_ident.to_string();
        let ty = &field.ty;

        deserialize_steps.push(quote! {
            let segment_data = data.next()
                .ok_or(::event_utils::EventParseError::MissingData(#field_name_str))?;
        });

        // Call the appropriate `read_*` utility function from `event_utils`.
        let parse_expr = if let Some(size) = get_u8_array_size(ty) {
            quote! { ::event_utils::read_array::<#size>(#field_name_str, &segment_data)? }
        } else if is_vec_u8(ty) {
            quote! { segment_data }
        } else if let Some(type_name) = get_simple_type_ident_str(ty) {
            match type_name.as_str() {
                "String" => quote! { ::event_utils::read_string(#field_name_str, segment_data)? },
                "Pubkey" => quote! { ::event_utils::read_pubkey(#field_name_str, &segment_data)? },
                "u8" => quote! { ::event_utils::read_u8(#field_name_str, &segment_data)? },
                "u16" => quote! { ::event_utils::read_u16(#field_name_str, &segment_data)? },
                "u32" => quote! { ::event_utils::read_u32(#field_name_str, &segment_data)? },
                "u64" => quote! { ::event_utils::read_u64(#field_name_str, &segment_data)? },
                "u128" => quote! { ::event_utils::read_u128(#field_name_str, &segment_data)? },
                "i8" => quote! { ::event_utils::read_i8(#field_name_str, &segment_data)? },
                "i16" => quote! { ::event_utils::read_i16(#field_name_str, &segment_data)? },
                "i32" => quote! { ::event_utils::read_i32(#field_name_str, &segment_data)? },
                "i64" => quote! { ::event_utils::read_i64(#field_name_str, &segment_data)? },
                "i128" => quote! { ::event_utils::read_i128(#field_name_str, &segment_data)? },
                "bool" => quote! { ::event_utils::read_bool(#field_name_str, &segment_data)? },
                "f32" => quote! { ::event_utils::read_f32(#field_name_str, &segment_data)? },
                "f64" => quote! { ::event_utils::read_f64(#field_name_str, &segment_data)? },
                "U256" => quote! { ::event_utils::read_u256(#field_name_str, &segment_data)? },
                _ => {
                    return Error::new_spanned(
                        ty,
                        format!("Unsupported simple type '{type_name}' for deserialize."),
                    )
                    .to_compile_error()
                    .into()
                }
            }
        } else {
            return Error::new_spanned(ty, "Unsupported field type for deserialize.")
                .to_compile_error()
                .into();
        };

        deserialize_steps.push(quote! {
            let #field_ident = #parse_expr;
        });
    }

    deserialize_steps.push(quote! {
        if data.next().is_some() {
            return Err(::event_utils::EventParseError::Other("Trailing segments found after parsing"));
        }
    });

    let deserialize_impl = quote! {
        fn deserialize<I: Iterator<Item = Vec<u8>>>(mut data: I) -> Result<Self, ::event_utils::EventParseError> {
             #(#deserialize_steps)*

             Ok(Self {
                 #(#field_idents_for_struct),*
             })
        }
    };

    let try_from_log_impl = quote! {
         fn try_from_log(log: &str) -> Result<Self, ::event_utils::EventParseError> {
             use ::std::convert::TryInto;
             use ::event_utils::base64::engine::Engine as _;

             const LOG_PREFIX_PROGRAM_DATA: &str = "Program data: ";

             let data_part = if let Some(part) = log.strip_prefix(LOG_PREFIX_PROGRAM_DATA) {
                 part
             } else {
                 return Err(::event_utils::EventParseError::Other("Log prefix mismatch"));
             };

             let segments: Result<Vec<Vec<u8>>, _> = data_part
                 .split(' ')
                 .map(|s| ::event_utils::base64::engine::general_purpose::STANDARD.decode(s))
                 .collect();

             let mut decoded_segments = segments.map_err(|e| {
                 ::event_utils::EventParseError::Other("Base64 decode error")
             })?;

             if decoded_segments.is_empty() {
                 return Err(::event_utils::EventParseError::MissingData("discriminant"));
             }

             let discriminant_segment = decoded_segments.remove(0); // Take ownership
             let discriminant_bytes: &[u8; 16] = discriminant_segment
                 .as_slice()
                 .try_into()
                 .map_err(|_| ::event_utils::EventParseError::InvalidLength {
                     field: "discriminant",
                     expected: 16,
                     actual: discriminant_segment.len(),
                 })?;

             if discriminant_bytes != Self::DISC {
                 return Err(::event_utils::EventParseError::Other("Discriminant mismatch"));
             }

             Self::deserialize(decoded_segments.into_iter())
         }
    };

    let expanded = quote! {
        impl ::event_utils::Event for #struct_ident {
            const DISC: &'static [u8; 16] = #discriminant_tokens;

            #emit_impl
            #try_from_log_impl
            #deserialize_impl
        }
    };

    expanded.into()
}
