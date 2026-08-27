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
//!     #[nearest(off)]
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
//! (`nearest(target: u32)`, etc.).
//!
//! # The `off` variant
//! Mark a variant `#[nearest(off)]` (no explicit value - `off` always means `0`) to declare a
//! dedicated "off" state. `off` variants are excluded from `nearest`/`ceil` searches *unless*
//! the target is exactly `0`.
//! This keeps an "off" state from being silently picked as the "closest" match
//! for some unrelated nonzero target. `exact` is unaffected - since `off` is always `0`, plain
//! equality already only matches it when the target is `0`.
//!
//! ```ignore
//! #[nearest(off)]
//! Off = 0x0,
//! ```
//!
//! # Families
//! Enums may have internal organization, referred to as 'family'.
//! As soon as one variant has a 'family', an `<Enum>Family` is generated, and every
//! lookup function takes an extra `family: <Enum>Family` argument.
//!
//! **Once any variant declares a `family`, every other non-`off` variant must declare one too.**
//! The `off` variant may still be left unfamilied (allowing it to be reachable from any family), or
//! may declare a `family` if you want it scoped like anything else.
//!
//! In this example Odr is matched with a high-accuracy parameter (Ha):
//!
//! ```ignore
//! #[nearest(off)]
//! Off = 0x0,
//!
//! // family ha00
//! #[nearest(1_875, family = "ha00")]
//! _1_875hz = 0x1,
//! // ...
//!
//! // family ha01
//! #[nearest(15_625, family = "ha01")]
//! Ha01At15_625hz = 0x13,
//! // ...
//!
//! // -> Odr::nearest_mhz(target_mhz: u32, family: OdrFamily) -> Odr
//! ```
//!
//! ## Special Family Behaviors
//! Reserved family names are `"base"`, `"any"`, and `"default"`. These cannot be used 
//! explicitly as a variant's `family` name. Instead, they are automatically generated as 
//! special selectors inside `<Enum>Family` with the following behaviors:
//!
//! - `Any`: A global search selector. Passing `Any` to a lookup function searches 
//!   every entry unconditionally, ignoring family configurations entirely.
//! - `Base`: Acts as an automatic, universal fallback. If required, may be used in the future for
//!   some use-cases, but actually the *only* way a variant ends up  in `Base` is by leaving an
//!   `off` variant unfamilied. Allowing `off` variant to be shared among all families.
//! - `Default`: Generated only if the container declares `#[nearest(default_family = "...")]`. 
//!   It restricts the search to your configured default family (plus `Base`). This lets 
//!   users query "whatever family the driver is normally set up for" without selecting a family.
//!
//! ```ignore
//! #[nearest(unit = "mhz", default_family = "ha01")]
//! pub enum Odr { /* ... */ }
//! // Odr::nearest_mhz(target, OdrFamily::Default)  searches Ha01 (+ Base) entries only
//! // Odr::nearest_mhz(target, OdrFamily::Any)      searches every entry, ignoring family entirely
//! ```
//!
//! The `off`-skip rule is applied also for `Any` and `Default` selection. Users must request a 0
//! value to select it.
//!
//!
//! Container-level overrides: `#[nearest(ty = "u32", unit = "mhz", default_family = "...")]`
//! (`ty` defaults to `u32` - override to `u64` if a value would overflow it; `unit` defaults to
//! none; `default_family` defaults to none and requires at least one variant with a `family`).


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
    off: bool,
}

enum NearestArg {
    Value(LitInt),
    Family(LitStr),
    Off,
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
        if ident == "off" {
            return Ok(NearestArg::Off);
        }
        input.parse::<Token![=]>()?;
        if ident == "family" {
            let s: LitStr = input.parse()?;
            Ok(NearestArg::Family(s))
        } else {
            Err(syn::Error::new_spanned(
                    ident,
                    "unknown `nearest` argument, expected an integer value, `off`, or `family = \"...\"`",
            ))
        }
    }
}

struct NearestArgs {
    value: Option<u64>,
    family: Option<String>,
    off: bool,
}

impl Parse for NearestArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let items: Punctuated<NearestArg, Token![,]> = Punctuated::parse_terminated(input)?;
        let mut value = None;
        let mut family = None;
        let mut off = false;
        for item in items {
            match item {
                NearestArg::Value(lit) => value = Some(lit.base10_parse::<u64>()?),
                NearestArg::Family(s) => family = Some(s.value()),
                NearestArg::Off => off = true,
            }
        }
        Ok(NearestArgs { value, family, off })
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
    let mut default_family: Option<String> = None;

    // Container-level #[nearest(ty = "...", unit = "...", default_family = "...")].
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
            } else if meta.path.is_ident("default_family") {
                let s: LitStr = meta.value()?.parse()?;
                default_family = Some(s.value());
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

            if let Some(family) = &args.family {
                if family.eq_ignore_ascii_case("any")
                    || family.eq_ignore_ascii_case("default")
                    || family.eq_ignore_ascii_case("base")
                {
                    return TokenStream::from(
                        syn::Error::new_spanned(
                            &variant.ident,
                            "`any`, `default`, and `base` are reserved - `any`/`default` are lookup-only selectors, and `base` is only reachable implicitly by leaving an `off` variant unfamilied; none of them can be assigned as a variant's own family",
                        )
                        .to_compile_error(),
                    )
                    .into();
                }
            }

            if args.off && args.value.is_some() {
                return TokenStream::from(
                    syn::Error::new_spanned(
                        &variant.ident,
                        "`off` cannot be combined with an explicit value - `off` implies 0",
                    )
                    .to_compile_error(),
                )
                .into();
            }

            if args.off {
                entries.push(VariantEntry {
                    ident: variant.ident.clone(),
                    value: 0,
                    family: args.family,
                    off: true,
                });
            } else if let Some(value) = args.value {
                entries.push(VariantEntry {
                    ident: variant.ident.clone(),
                    value,
                    family: args.family,
                    off: false,
                });
            }
        }
    }
 
    if entries.is_empty() {
        panic!("Nearest: no variant carries a #[nearest(...)] value");
    }
 
    let has_families = entries.iter().any(|e| e.family.is_some());

    // Once families are in play, every non-`off` variant must opt in explicitly - no implicit
    // "Base" fallback for a variant that simply forgot to tag one. `off` alone is exempt, since
    // it defaults to `Base` (universally reachable) when left untagged.
    if has_families {
        for e in &entries {
            if e.family.is_none() && !e.off {
                return TokenStream::from(
                    syn::Error::new_spanned(
                        &e.ident,
                        "once any variant declares a `family`, every non-`off` variant must declare one too (`family = \"...\"`) - only `off` variants may omit it, defaulting to `Base`",
                    )
                    .to_compile_error(),
                )
                .into();
            }
        }
    }

    if default_family.is_some() && !has_families {
        return TokenStream::from(
            syn::Error::new_spanned(
                &input.ident,
                "`default_family` requires at least one variant to declare a `family`",
            )
            .to_compile_error(),
        )
        .into();
    }

    let has_off = entries.iter().any(|e| e.off);
 
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

    let family_doc = if has_families {
        let mandatory = "every non-`off` variant declares an explicit `family`; only `off` may omit it, defaulting to `Base`";
        let default_selector_doc = default_family.as_ref()
            .map(|f| format!(" Pass `Default` to restrict to the configured default family (`{f}`, plus `Base`)."))
            .unwrap_or_default();
        format!(
            ", restricted to `family` (plus `Base`). Pass `Any` to search every family, ignoring the restriction entirely.{default_selector_doc} ({mandatory}.)"
        )
    } else {
        String::new()
    };
    let unit_doc = unit.as_ref()
        .map(|u| format!(" (input in {u} unit)"))
        .unwrap_or_default();
    let off_doc = if has_off {
        if has_families {
            " `off` variants are skipped unless the target is exactly `0`, regardless of which family selector is used (a named family, `Any`, or `Default`)."
        } else {
            " `off` variants are skipped unless the target is exactly `0`."
        }
    } else {
        ""
    };

    let nearest_doc = format!(
        "Choose the variant closest to the input{family_doc}{unit_doc}.{off_doc} \n\nStart matching by selecting the first element and then search for values that minimize the difference."
    );
    let exact_doc = format!(
        "Exact match only{family_doc}{unit_doc}. \n\nReturn `None` if nothing matches"
    );
    let ceil_doc = format!(
        "Smallest value that is `>= {target_ident}`{family_doc}{unit_doc}.{off_doc}\n\nSaturates to the max if `target` exceeds everything."
    );
 
    let expanded = if has_families {
        let family_enum_ident = format_ident!("{}Family", enum_ident);
 
        // `Base` always matches; `Any` unconditionally matches every family, regardless of
        // `default_family` (see `matches` below). Both are reserved for the lookup side, not
        // assignable as a variant's own family (enforced above).
        let mut family_names = vec!["Base".to_string(), "Any".to_string()];
        for e in &entries {
            if let Some(f) = &e.family {
                let pascal = to_pascal_case(f);
                if !family_names.contains(&pascal) {
                    family_names.push(pascal);
                }
            }
        }

        // Resolve `default_family` (if any) against the *declared* family names only - `Default`
        // itself isn't added to `family_names` yet, so it can't resolve to itself.
        let default_family_variant: Option<syn::Ident> = match &default_family {
            Some(f) => {
                let pascal = to_pascal_case(f);
                if pascal == "Any" || pascal == "Default" || pascal == "Base" {
                    return TokenStream::from(
                        syn::Error::new_spanned(
                            &input.ident,
                            "`default_family` cannot be `any`, `default`, or `base` - those are reserved selectors, not assignable families",
                        )
                        .to_compile_error(),
                    )
                    .into();
                }
                if !family_names.contains(&pascal) {
                    return TokenStream::from(
                        syn::Error::new_spanned(
                            &input.ident,
                            format!("`default_family = \"{f}\"` does not match any declared family"),
                        )
                        .to_compile_error(),
                    )
                    .into();
                }
                Some(format_ident!("{}", pascal))
            }
            None => None,
        };

        // Only add the reserved `Default` selector when a `default_family` is actually
        // configured - otherwise there'd be nothing meaningful for it to mean.
        if default_family_variant.is_some() {
            family_names.push("Default".to_string());
        }

        let family_variant_idents: Vec<syn::Ident> =
            family_names.iter().map(|n| format_ident!("{}", n)).collect();

        // `Any` always matches everything. `Default` (only generated when `default_family` is
        // configured) matches the configured family, plus `Base` - same rule as any other family.
        let matches_body = match &default_family_variant {
            Some(default_ident) => quote! {
                match requested {
                    #family_enum_ident::Any => true,
                    #family_enum_ident::Default => {
                        (self as u8 == #family_enum_ident::#default_ident as u8) || (self as u8 == #family_enum_ident::Base as u8)
                    }
                    _ => (self as u8 == requested as u8) || (self as u8 == #family_enum_ident::Base as u8),
                }
            },
            None => quote! {
                match requested {
                    #family_enum_ident::Any => true,
                    _ => (self as u8 == requested as u8) || (self as u8 == #family_enum_ident::Base as u8),
                }
            },
        };

        let family_enum_doc = {
            let default_line = match &default_family {
                Some(f) => format!(
                    "- `Default` behaves as if the configured default family (`{f}`, plus `Base`) had been requested. \n"
                ),
                None => String::new(),
            };
            format!(
                "Family selector generated for [`{enum_ident}`]. \n\n\
                 - `Base` variants are always eligible, no matter which family is requested - the only \
                 way to reach `Base` is by leaving an `off` variant unfamilied; it can't be assigned \
                 explicitly. \n\
                 - `Any` unconditionally matches every family - it ignores any restriction entirely, \
                 including `default_family`. \n\
                 {default_line}\n\
                 Once any variant of `{enum_ident}` declares a `family`, every other non-`off` variant must \
                 declare one too - only `off` may omit it, in which case it defaults to `Base` and stays \
                 reachable regardless of the requested family. The `off`-skip rule (excluded from \
                 `nearest`/`ceil` unless the target is exactly `0`) applies identically under every \
                 selector: a named family, `Any`, or `Default`."
            )
        };
 
        let table_entries = entries.iter().map(|e| {
            let ident = &e.ident;
            let value = syn::LitInt::new(&e.value.to_string(), proc_macro2::Span::call_site());
            let off = e.off;
            let family_variant = format_ident!(
                "{}",
                e.family
                    .as_deref()
                    .map(to_pascal_case)
                    .unwrap_or_else(|| "Base".to_string())
            );
            quote! { (#enum_ident::#ident, #value, #off, #family_enum_ident::#family_variant) }
        });
 
        quote! {
            #[doc = #family_enum_doc]
            #[derive(Clone, Copy, PartialEq, Eq, Debug)]
            pub enum #family_enum_ident {
                #(#family_variant_idents),*
            }
 
            impl #family_enum_ident {
                const fn matches(self, requested: Self) -> bool {
                    #matches_body
                }
            }
 
            impl #enum_ident {
                const TABLE_: &'static [(#enum_ident, #value_ty, bool, #family_enum_ident)] = &[
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
                        let (_, v, off, f) = table[i];
                        if f.matches(family) && (!off || #target_ident == 0) {
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
                        let (variant, v, _off, f) = table[i];
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
                    let mut max_idx: Option<usize> = None;
                    let mut first_family_idx: Option<usize> = None;
                    let mut i = 0usize;

                    while i < table.len() {
                        let (_, v, off, f) = table[i];
                        if f.matches(family) && (!off || #target_ident == 0) {
                            // Track the very first variant belonging to this family as a safety fallback
                            if first_family_idx.is_none() {
                                first_family_idx = Some(i);
                            }

                            // Track the maximum value within the requested family
                            match max_idx {
                                None => max_idx = Some(i),
                                Some(mi) => {
                                    if v > table[mi].1 {
                                        max_idx = Some(i);
                                    }
                                }
                            }

                            // Track the smallest value >= target_ident
                            if v >= #target_ident {
                                best_idx = Some(match best_idx {
                                    None => i,
                                    Some(bi) => if v < table[bi].1 { i } else { bi },
                                });
                            }
                        }
                        i += 1;
                    }

                    match (best_idx, max_idx, first_family_idx) {
                        (Some(i), _, _) => table[i].0,
                        (None, Some(i), _) => table[i].0,
                        (None, None, Some(i)) => table[i].0,
                        // Unreachable if families are constructed properly, but safe guard against empty lookup
                        (None, None, None) => table[0].0,
                    }
                }
            }
        }
    } else {
        let table_entries = entries.iter().map(|e| {
            let ident = &e.ident;
            let value = syn::LitInt::new(&e.value.to_string(), proc_macro2::Span::call_site());
            let off = e.off;
            quote! { (#enum_ident::#ident, #value, #off) }
        });
 
        quote! {
            impl #enum_ident {
                const TABLE_: &'static [(#enum_ident, #value_ty, bool)] = &[
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
                        let (_, v, off) = table[i];
                        if !off || #target_ident == 0 {
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
                pub const fn #exact_fn(#target_ident: #value_ty) -> Option<Self> {
                    let table = Self::TABLE_;
                    let mut i = 0usize;
                    while i < table.len() {
                        let (variant, v, _off) = table[i];
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
                        let (_, v, off) = table[i];
                        if !off || #target_ident == 0 {
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
    };
 
    TokenStream::from(expanded).into()
}
