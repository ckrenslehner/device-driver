use proc_macro::TokenStream;
use syn::Error;

pub struct Ast {}

pub fn parse(attr: TokenStream, item: TokenStream) -> Result<Ast, ()> {
    Ok(Ast {})
}
