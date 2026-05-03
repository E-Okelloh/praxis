//! Proc macros for Praxis: `#[invariant_test]` and `#[profile]` (Phase 2).
#![deny(unsafe_code)]
extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Marks a function as a Praxis invariant test.
///
/// The function must accept no arguments and return `()`. It expands to a
/// normal `#[test]` so `cargo test` discovers it automatically. Additional
/// metadata is recorded via a static so the `praxis test` CLI can enumerate
/// and run only invariant tests from a binary.
///
/// # Example
/// ```ignore
/// #[praxis_macros::invariant_test]
/// fn no_lamport_drain() {
///     // ... build Ctx, run fuzzer ...
/// }
/// ```
#[proc_macro_attribute]
pub fn invariant_test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let _ = attr; // no arguments in Phase 1
    let func = parse_macro_input!(item as ItemFn);
    let expanded = expand_invariant_test(func);
    TokenStream::from(expanded)
}

fn expand_invariant_test(func: ItemFn) -> TokenStream2 {
    let attrs = &func.attrs;
    let vis = &func.vis;
    let sig = &func.sig;
    let block = &func.block;
    let fn_name = &sig.ident;
    let fn_name_str = fn_name.to_string();

    // Registration entry so praxis-cli can list invariant test names at runtime.
    let reg_ident = quote::format_ident!("__PRAXIS_INVARIANT_{}", fn_name_str.to_uppercase());

    quote! {
        #[test]
        #(#attrs)*
        #vis #sig #block

        // Static registration — name only; CLI reads these at link time.
        #[used]
        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        static #reg_ident: &'static str = #fn_name_str;
    }
}

/// Phase 2 placeholder — CU profiling annotation.
#[proc_macro_attribute]
pub fn profile(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
