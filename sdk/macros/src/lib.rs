use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn contract(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    // let input = parse_macro_input!(input);
    // let output = state::impl_contract_state(input);

    // match output {
    //     syn::Result::Ok(token_stream) => token_stream,
    //     syn::Result::Err(err) => err.to_compile_error(),
    // }
    // .into()
    input
}

#[proc_macro_attribute]
pub fn state(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    // let input = parse_macro_input!(input);
    // let output = state::impl_contract_state(input);

    // match output {
    //     syn::Result::Ok(token_stream) => token_stream,
    //     syn::Result::Err(err) => err.to_compile_error(),
    // }
    // .into()
    input
}

#[proc_macro_attribute]
pub fn action(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    // let input = parse_macro_input!(input);
    // let output = state::impl_contract_state(input);

    // match output {
    //     syn::Result::Ok(token_stream) => token_stream,
    //     syn::Result::Err(err) => err.to_compile_error(),
    // }
    // .into()
    input
}
