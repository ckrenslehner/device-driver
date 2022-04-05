use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{
    braced,
    parse::{self, Parse, ParseStream},
    token::Brace,
    Error, ExprRange, Ident, Item, ItemEnum, ItemStruct, Token,
};

mod helpers;

/// Parser for the main body of a device driver module.
#[derive(Debug)]
pub struct Input {
    _mod_token: Token![mod],
    pub ident: Ident,
    _brace_token: Brace,
    pub items: Vec<Item>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let content;

        let _mod_token = input.parse()?;
        let ident = input.parse()?;
        let _brace_token = braced!(content in input);
        let items = content.call(|input| {
            let mut items = vec![];

            while !input.is_empty() {
                items.push(input.parse()?);
            }

            Ok(items)
        })?;

        Ok(Input {
            _mod_token,
            ident,
            _brace_token,
            items,
        })
    }
}

pub struct AppArgs {}

impl Parse for AppArgs {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        Ok(AppArgs {})
    }
}

/// Main AST structure for the macro to operate on.
pub struct Ast {
    /// All user defined fields.
    pub fields: HashMap<Ident, Field>,
    /// All user derfined registers.
    pub registers: ModRegisters,
}

/// A field inside a register, an enum marked with `#[field(attrs)]`.
pub struct Field {
    /// Attributes on the field.
    pub attrs: FieldAttributes,
    /// The enum marked as a field.
    pub item: ItemEnum,
}

pub struct FieldAttributes {}

pub struct ModRegisters {
    pub attrs: ModRegistersAttributes,
    pub registers: HashMap<Ident, Register>,
}

pub struct ModRegistersAttributes {}

pub struct Register {
    pub attts: RegisterAttributes,
    pub strct: ItemStruct,
    pub fields: Vec<RegisterFields>,
}

pub struct RegisterAttributes {}

pub struct RegisterFields {
    pub at: HashMap<Ident, ExprRange>,
}

pub fn parse(attr: TokenStream2, input: TokenStream2) -> Result<Ast, parse::Error> {
    let input: Input = syn::parse2(input)?;
    let app_args: AppArgs = syn::parse2(attr)?;

    Ok(Ast {
        fields: HashMap::new(),
        registers: ModRegisters {
            attrs: ModRegistersAttributes {},
            registers: HashMap::new(),
        },
    })
}
