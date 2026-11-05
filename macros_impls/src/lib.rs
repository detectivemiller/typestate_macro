pub mod type_state;
#[macro_use]
pub mod space;
pub mod state_field;
pub mod state;
mod transition;

use proc_macro2::TokenStream;
use quote::ToTokens;

pub fn expand_type_state(input: TokenStream) -> TokenStream {
    let pattern: type_state::TypeState = syn::parse2(input).unwrap();
    pattern.into_token_stream()
}