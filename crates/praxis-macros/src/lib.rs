//! TODO(phase-1): Proc macros #[invariant_test] and #[profile]
extern crate proc_macro;
use proc_macro::TokenStream;

/// Placeholder — full implementation in Week 6.
#[proc_macro_attribute]
pub fn invariant_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Placeholder — full implementation in Phase 2.
#[proc_macro_attribute]
pub fn profile(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
