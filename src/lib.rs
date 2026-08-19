//! `#[derive(Nearest)]` attach an integer value to each variant of a
//! fieldless enum and get three `const fn` lookups for free:
//!
//! - `nearest[_<unit>](target[_<unit>])` -> `Self`   (closest value, always succeeds)
//! - `exact[_<unit>](target[_<unit>])`   -> `Option<Self>` (only match if exact correspondence
//!                                                               is found)
//! - `ceil[_<unit>](target[_<unit>])`    -> `Self`    (smallest value >= target; fallbacks to
//!                                                          max)
//!
//! Compile-time conversion avoids runtime overhead.
//!
//! ```ignore
//! const ODR: Odr = Odr::ceil_mhz(1_000); // computed at compile time
//! ```
//!
//! # Naming a unit
//! Adding `unit = "mhz"` in macro to generate functions that contain that unit naming, the
//! signature of the function becomes self-documenting:
//! ```ignore
//! #[derive(..., Nearest)]
//! #[nearest(unit = "mhz")]
//! pub enum Odr {
//!     #[default]
//!     #[nearest(0)]
//!     Off = 0x0,
//!     #[nearest(1_875)] // 1.875 Hz, in mHz (avoid float conversions)
//!     _1_875hz = 0x1,
//! }
//! // -> Odr::nearest_mhz(target_mhz: u32) -> Odr
//! // -> Odr::exact_mhz(target_mhz: u32) -> Option<Odr>
//! // -> Odr::ceil_mhz(target_mhz: u32) -> Odr
//! ```
//!
//! In this example the Odr stands for Output Data Rate, it's a common enum used in sensor's driver
//! development.
//!
//! Omitting `unit` will generate un-suffixed names/params from before
//! (`nearest(target: u32)`, etc.) - backwards compatible.
//!
//! # Families
//! Enums may have some sort of organization, those are named as 'family'.
//! Variants without a 'family' key are implicitly `"base"` and are always eligible.
//! As soon as one variant has a 'family', a generated `<Enum>Family` is generated, and every lookup
//! function takes an extra `family: <Enum>Family` argument.
//!
//! In this example Odr is matched with a high-accuracy parameter (Ha):
//!
//! ```ignore
//! #[nearest(15_625, family = "ha01")]
//! Ha01At15_625hz = 0x13,
//! // -> Odr::nearest_mhz(target_mhz: u32, family: OdrFamily) -> Odr
//! ```
//!
//! Container-level overrides: `#[nearest(ty = "u32", unit = "mhz")]`
//! (`ty` defaults to `u32` - override to `u64` if a value would overflow it; `unit` defaults to none).


use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Data, DeriveInput, Fields, LitFloat, LitInt, LitStr, Token,
};

struct VariantEntry {
    ident: syn::Ident,
    value: u64,
    family: Option<String>,
}

enum NearestArg {
    Value(LitInt),
    Family(LitStr),
}

impl Parse for NearestArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(LitFloat) {
            let lit: LitFloat =  input.parse()?;
            return Err(syn::Error::new_spanned(lit, "Nearest uses integer values only (e.g. mHz) - found a float literal; \
               multiply by 1000 and use an integer instead", 
            ));
        }

        if input.peek(LitInt) {
            let lit: LitInt = input.parse()?;
            return Ok(NearestArg::Value(lit));
        }

        let ident: syn::Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        if ident == "family" {
            let s: LitStr = input.parse()?;
            Ok(NearestArg::Family(s))
        } else {
            Err(syn::Error::new_spanned(
                    ident,
                    "unknown `nearest` argument, expected an integer value or `family = \"...\"`",
            ))
        }
    }
}

struct NearestArgs {
    value: Option<u64>,
    family: Option<String>,
}

impl Parse for NearestArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let items: Punctuated<NearestArg, Token![,]> = Punctuated::parse_terminated(input)?;
        let mut value = None;
        let mut family = None;
        for item in items {
            match item {
                NearestArg::Value(lit) => value = Some(lit.base10_parse::<u64>()?),
                NearestArg::Family(s) => family = Some(s.value()),
            }
        }
        Ok(NearestArgs { value, family })
    }
}

fn to_pascal_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[proc_macro_derive(Nearest, attributes(nearest))]
pub fn derive_nearest(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_ident = &input.ident;

    let mut value_ty: syn::Type = syn::parse_str("u32").unwrap();
    let mut unit: Option<String> = None;

    // Container-level #[nearest(ty = "...", unit = "...")].
    // These are all plain `ident = "literal"` pairs
    for attr in &input.attrs {
        if !attr.path().is_ident("nearest") {
            continue;
        }
        let result = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("ty") {
                let s: LitStr = meta.value()?.parse()?;
                value_ty = syn::parse_str(&s.value())
                    .map_err(|e| meta.error(format!("`ty` must be a valid Rust integer type: {e}")))?;
                Ok(())
            } else if meta.path.is_ident("unit") {
                let s: LitStr = meta.value()?.parse()?;
                unit = Some(s.value());
                Ok(())
            } else {
                Err(meta.error("unsupported `nearest` container argument"))
            }
        });

        if let Err(e) = result {
            return TokenStream::from(e.to_compile_error()).into();
        }
    }

    let data = match &input.data {
        Data::Enum(data) => data,
        _ => panic!("Nearest can only be derived for enums"),
    };
 
    let mut entries = Vec::new();
 
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            panic!("Nearest only supports fieldless enum variants");
        }
 
        for attr in &variant.attrs {
            if !attr.path().is_ident("nearest") {
                continue;
            }
            let args = match attr.parse_args::<NearestArgs>() {
                Ok(args) => args,
                Err(e) => return TokenStream::from(e.to_compile_error()).into(),
            };
            if let Some(value) = args.value {
                entries.push(VariantEntry {
                    ident: variant.ident.clone(),
                    value,
                    family: args.family,
                });
            }
        }
    }
 
    if entries.is_empty() {
        panic!("Nearest: no variant carries a #[nearest(...)] value");
    }
 
    let has_families = entries.iter().any(|e| e.family.is_some());
 
    let target_ident = match &unit {
        Some(u) => format_ident!("target_{}", u),
        None => format_ident!("target"),
    };

    let (nearest_fn, exact_fn, ceil_fn) = match &unit {
        Some(u) => (
            format_ident!("nearest_{}", u),
            format_ident!("exact_{}", u),
            format_ident!("ceil_{}", u),
        ),
        None => (
            format_ident!("nearest"),
            format_ident!("exact"),
            format_ident!("ceil"),
        ),
    };

    let family_doc = if has_families { ", restricted to `family` (plus `Base`)" } else { "" };
    let unit_doc = unit.as_ref()
        .map(|u| format!(" (input in {u} unit)"))
        .unwrap_or_default();

    let nearest_doc = format!(
        "Choose the variant closest to the input{family_doc}{unit_doc}. \n\nStart matching by selecting first element and then search for values that minimize the difference."
    );
    let exact_doc = format!(
        "Exact match only{family_doc}{unit_doc}. \n\nReturn `None` if nothing matches"
    );
    let ceil_doc = format!(
        "Smallest value that is `>= {target_ident}`{family_doc}{unit_doc}.\n\nSaturates to the max if `target` exceeds everything."
    );
 
    let expanded = if has_families {
        let family_enum_ident = format_ident!("{}Family", enum_ident);
 
        let mut family_names = vec!["Base".to_string()];
        for e in &entries {
            if let Some(f) = &e.family {
                let pascal = to_pascal_case(f);
                if pascal != "Base" && !family_names.contains(&pascal) {
                    family_names.push(pascal);
                }
            }
        }
        let family_variant_idents: Vec<syn::Ident> =
            family_names.iter().map(|n| format_ident!("{}", n)).collect();
 
        let table_entries = entries.iter().map(|e| {
            let ident = &e.ident;
            let value = syn::LitInt::new(&e.value.to_string(), proc_macro2::Span::call_site());
            let family_variant = format_ident!(
                "{}",
                e.family
                    .as_deref()
                    .map(to_pascal_case)
                    .unwrap_or_else(|| "Base".to_string())
            );
            quote! { (#enum_ident::#ident, #value, #family_enum_ident::#family_variant) }
        });
 
        quote! {
            #[derive(Clone, Copy, PartialEq, Eq, Debug)]
            pub enum #family_enum_ident {
                #(#family_variant_idents),*
            }
 
            impl #family_enum_ident {
                const fn matches(self, requested: Self) -> bool {
                    (self as u8 == requested as u8) || (self as u8 == #family_enum_ident::Base as u8)
                }
            }
 
            impl #enum_ident {
                const TABLE_: &'static [(#enum_ident, #value_ty, #family_enum_ident)] = &[
                    #(#table_entries),*
                ];
 
                #[doc = #nearest_doc]
                pub const fn #nearest_fn(#target_ident: #value_ty, family: #family_enum_ident) -> Self {
                    let table = Self::TABLE_;
                    let mut best_idx = 0usize;
                    let mut best_diff = #value_ty::MAX;
                    let mut found = false;
                    let mut i = 0usize;
                    while i < table.len() {
                        let (_, v, f) = table[i];
                        if f.matches(family) {
                            // compute the absolute difference
                            let diff = if v > #target_ident { v - #target_ident } else { #target_ident - v };
                            if !found || diff < best_diff {
                                best_diff = diff;
                                best_idx = i;
                                found = true;
                            }
                        }
                        i += 1;
                    }
                    table[best_idx].0
                }
 
                #[doc = #exact_doc]
                pub const fn #exact_fn(#target_ident: #value_ty, family: #family_enum_ident) -> Option<Self> {
                    let table = Self::TABLE_;
                    let mut i = 0usize;
                    while i < table.len() {
                        let (variant, v, f) = table[i];
                        if f.matches(family) && v == #target_ident {
                            return Some(variant);
                        }
                        i += 1;
                    }
                    None
                }
 
                #[doc = #ceil_doc]
                pub const fn #ceil_fn(#target_ident: #value_ty, family: #family_enum_ident) -> Self {
                    let table = Self::TABLE_;
                    let mut best_idx: Option<usize> = None;
                    let mut max_idx = 0usize;
                    let mut i = 0usize;
                    while i < table.len() {
                        let (_, v, f) = table[i];
                        if f.matches(family) {
                            if v > table[max_idx].1 {
                                max_idx = i;
                            }
                            if v >= #target_ident {
                                best_idx = Some(match best_idx {
                                    None => i,
                                    Some(bi) => if v < table[bi].1 { i } else { bi },
                                });
                            }
                        }
                        i += 1;
                    }
                    match best_idx {
                        Some(i) => table[i].0,
                        None => table[max_idx].0,
                    }
                }
            }
        }
    } else {
        let table_entries = entries.iter().map(|e| {
            let ident = &e.ident;
            let value = syn::LitInt::new(&e.value.to_string(), proc_macro2::Span::call_site());
            quote! { (#enum_ident::#ident, #value) }
        });
 
        quote! {
            impl #enum_ident {
                const TABLE_: &'static [(#enum_ident, #value_ty)] = &[
                    #(#table_entries),*
                ];
 
                #[doc = #nearest_doc]
                pub const fn #nearest_fn(#target_ident: #value_ty) -> Self {
                    let table = Self::TABLE_;
                    let mut best_idx = 0usize;
                    let mut best_diff = #value_ty::MAX;
                    let mut found = false;
                    let mut i = 0usize;
                    while i < table.len() {
                        let (_, v) = table[i];
                        let diff = if v > #target_ident { v - #target_ident } else { #target_ident - v };
                        if !found || diff < best_diff {
                            best_diff = diff;
                            best_idx = i;
                            found = true;
                        }
                        i += 1;
                    }
                    table[best_idx].0
                }
 
                #[doc = #exact_doc]
                pub const fn #exact_fn(#target_ident: #value_ty) -> Option<Self> {
                    let table = Self::TABLE_;
                    let mut i = 0usize;
                    while i < table.len() {
                        let (variant, v) = table[i];
                        if v == #target_ident {
                            return Some(variant);
                        }
                        i += 1;
                    }
                    None
                }
 
                #[doc = #ceil_doc]
                pub const fn #ceil_fn(#target_ident: #value_ty) -> Self {
                    let table = Self::TABLE_;
                    let mut best_idx: Option<usize> = None;
                    let mut max_idx = 0usize;
                    let mut i = 0usize;
                    while i < table.len() {
                        let (_, v) = table[i];
                        if v > table[max_idx].1 {
                            max_idx = i;
                        }
                        if v >= #target_ident {
                            best_idx = Some(match best_idx {
                                None => i,
                                Some(bi) => if v < table[bi].1 { i } else { bi },
                            });
                        }
                        i += 1;
                    }
                    match best_idx {
                        Some(i) => table[i].0,
                        None => table[max_idx].0,
                    }
                }
            }
        }
    };
 
    TokenStream::from(expanded).into()
}

