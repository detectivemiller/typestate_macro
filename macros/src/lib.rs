use proc_macro::TokenStream;

#[proc_macro]
pub fn typestate(input: TokenStream) -> TokenStream {
    let input2 = proc_macro2::TokenStream::from(input);

    let output2 = macros_impls::expand_type_state(input2);

    TokenStream::from(output2)
}
