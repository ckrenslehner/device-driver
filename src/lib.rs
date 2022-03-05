use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};

mod analysis;
mod ast;
mod codegen;

#[proc_macro_attribute]
#[proc_macro_error]
pub fn device_driver(attr: TokenStream, item: TokenStream) -> TokenStream {
    println!("attr: \"{}\"", attr.to_string());
    println!("item: \"{}\"", item.to_string());

    let ast = match ast::parse(attr, item) {
        Ok(ast) => ast,
        Err(err) => todo!("parse error {:#?}", err),
    };

    let analysis = match analysis::analyze(&ast) {
        Ok(analysis) => analysis,
        Err(err) => todo!("analysis error {:#?}", err),
    };

    codegen::generate(&ast, &analysis)
}
