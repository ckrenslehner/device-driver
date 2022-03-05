use crate::{analysis::Analysis, ast::Ast};
use quote::quote;

pub fn generate(ast: &Ast, anaysis: &Analysis) -> proc_macro::TokenStream {
    quote! {}.into()
}
