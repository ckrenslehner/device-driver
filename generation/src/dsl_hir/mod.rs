use std::mem::Discriminant;

use convert_case::Boundary;
use proc_macro2::Span;
use syn::{
    Ident, LitBool, LitInt, LitStr, Token, braced, bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
};

pub mod mir_transform;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub global_config_list: GlobalConfigList,
    pub object_list: ObjectList,
}

impl Parse for Device {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            global_config_list: input.parse()?,
            object_list: input.parse()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalConfigList {
    pub configs: Vec<GlobalConfig>,
}

impl Parse for GlobalConfigList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if !input.peek(kw::config) {
            return Ok(Self {
                configs: Vec::new(),
            });
        }

        input.parse::<kw::config>()?;
        let config_input;
        braced!(config_input in input);

        let mut configs = Vec::new();

        while !config_input.is_empty() {
            configs.push(config_input.parse()?);
        }

        Ok(Self { configs })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlobalConfig {
    DefaultRegisterAccess(Access),
    DefaultFieldAccess(Access),
    DefaultBufferAccess(Access),
    DefaultByteOrder(ByteOrder),
    DefaultBitOrder(BitOrder),
    RegisterAddressType(syn::Ident),
    CommandAddressType(syn::Ident),
    BufferAddressType(syn::Ident),
    NameWordBoundaries(Vec<Boundary>),
    DefmtFeature(syn::LitStr),
}

impl Parse for GlobalConfig {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![type]>()?;

        let lookahead = input.lookahead1();

        if lookahead.peek(kw::DefaultRegisterAccess) {
            input.parse::<kw::DefaultRegisterAccess>()?;
            input.parse::<Token![=]>()?;
            let value = input.parse()?;
            input.parse::<Token![;]>()?;
            Ok(Self::DefaultRegisterAccess(value))
        } else if lookahead.peek(kw::DefaultFieldAccess) {
            input.parse::<kw::DefaultFieldAccess>()?;
            input.parse::<Token![=]>()?;
            let value = input.parse()?;
            input.parse::<Token![;]>()?;
            Ok(Self::DefaultFieldAccess(value))
        } else if lookahead.peek(kw::DefaultBufferAccess) {
            input.parse::<kw::DefaultBufferAccess>()?;
            input.parse::<Token![=]>()?;
            let value = input.parse()?;
            input.parse::<Token![;]>()?;
            Ok(Self::DefaultBufferAccess(value))
        } else if lookahead.peek(kw::DefaultByteOrder) {
            input.parse::<kw::DefaultByteOrder>()?;
            input.parse::<Token![=]>()?;
            let value = input.parse()?;
            input.parse::<Token![;]>()?;
            Ok(Self::DefaultByteOrder(value))
        } else if lookahead.peek(kw::DefaultBitOrder) {
            input.parse::<kw::DefaultBitOrder>()?;
            input.parse::<Token![=]>()?;
            let value = input.parse()?;
            input.parse::<Token![;]>()?;
            Ok(Self::DefaultBitOrder(value))
        } else if lookahead.peek(kw::RegisterAddressType) {
            input.parse::<kw::RegisterAddressType>()?;
            input.parse::<Token![=]>()?;
            let value = input.parse()?;
            input.parse::<Token![;]>()?;
            Ok(Self::RegisterAddressType(value))
        } else if lookahead.peek(kw::CommandAddressType) {
            input.parse::<kw::CommandAddressType>()?;
            input.parse::<Token![=]>()?;
            let value = input.parse()?;
            input.parse::<Token![;]>()?;
            Ok(Self::CommandAddressType(value))
        } else if lookahead.peek(kw::BufferAddressType) {
            input.parse::<kw::BufferAddressType>()?;
            input.parse::<Token![=]>()?;
            let value = input.parse()?;
            input.parse::<Token![;]>()?;
            Ok(Self::BufferAddressType(value))
        } else if lookahead.peek(kw::NameWordBoundaries) {
            input.parse::<kw::NameWordBoundaries>()?;
            input.parse::<Token![=]>()?;

            let value = if input.peek(syn::token::Bracket) {
                let bracket_input;
                bracketed!(bracket_input in input);

                let boundaries = Punctuated::<Ident, Token![,]>::parse_terminated(&bracket_input)?;
                boundaries
                    .into_iter()
                    .map(|ident| {
                        for b in Boundary::all() {
                            if format!("{b:?}").to_lowercase() == ident.to_string().to_lowercase() {
                                return Ok(b);
                            }
                        }

                        Err(syn::Error::new(ident.span(), format!("`{}` is not a valid boundary name. One of the following was expected: {:?}", ident, Boundary::all())))
                    }).collect::<Result<Vec<_>, _>>()?
            } else {
                let string_value = match input.parse::<LitStr>() {
                    Ok(lit) => lit.value(),
                    Err(e) => {
                        return Err(syn::Error::new(
                            e.span(),
                            "Expected an array of boundaries or a string",
                        ));
                    }
                };

                Boundary::list_from(&string_value)
            };

            input.parse::<Token![;]>()?;
            Ok(Self::NameWordBoundaries(value))
        } else if lookahead.peek(kw::DefmtFeature) {
            input.parse::<kw::DefmtFeature>()?;
            input.parse::<Token![=]>()?;
            let value = input.parse()?;
            input.parse::<Token![;]>()?;
            Ok(Self::DefmtFeature(value))
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectList {
    pub objects: Vec<Object>,
}

impl Parse for ObjectList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let punctuated_objects = Punctuated::<Object, Token![,]>::parse_terminated(input)?;
        Ok(Self {
            objects: punctuated_objects.into_iter().collect(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Object {
    Block(Block),
    Register(Register),
    Command(Command),
    Buffer(Buffer),
    Ref(RefObject),
}

impl Parse for Object {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Perform the lookahead on a fork where any attribute list is skipped
        let fork = input.fork();
        let _ = fork.parse::<AttributeList>();
        let lookahead = fork.lookahead1();

        if lookahead.peek(kw::block) {
            Ok(Self::Block(input.parse()?))
        } else if lookahead.peek(kw::register) {
            Ok(Self::Register(input.parse()?))
        } else if lookahead.peek(kw::command) {
            Ok(Self::Command(input.parse()?))
        } else if lookahead.peek(kw::buffer) {
            Ok(Self::Buffer(input.parse()?))
        } else if lookahead.peek(Token![ref]) {
            Ok(Self::Ref(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefObject {
    pub attribute_list: AttributeList,
    pub identifier: syn::Ident,
    pub object: Box<Object>,
}

impl Parse for RefObject {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attribute_list = input.parse()?;

        input.parse::<Token![ref]>()?;

        let identifier = input.parse()?;

        input.parse::<Token![=]>()?;

        let object = input.parse()?;

        Ok(Self {
            attribute_list,
            identifier,
            object,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AttributeList {
    pub attributes: Vec<Attribute>,
}

#[cfg(test)]
impl AttributeList {
    pub fn new() -> Self {
        Self {
            attributes: Vec::new(),
        }
    }
}

impl Parse for AttributeList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attributes = syn::Attribute::parse_outer(input)?;

        Ok(Self {
            attributes: attributes
                .into_iter()
                .map(
                    |attr| match attr.path().require_ident()?.to_string().as_str() {
                        "doc" => match &attr.meta.require_name_value()?.value {
                            syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(value),
                                ..
                            }) => Ok(Attribute::Doc(value.value())),
                            _ => Err(syn::Error::new_spanned(
                                attr,
                                "Invalid doc attribute format",
                            )),
                        },
                        "cfg" => {
                            Ok(Attribute::Cfg(attr.meta.require_list()?.tokens.to_string(), attr.span()))
                        }
                        val => {
                            Err(syn::Error::new_spanned(
                                attr,
                                format!("Unsupported attribute '{val}'. Only `doc` and `cfg` attributes are allowed"),
                            ))
                        }
                    },
                )
                .collect::<Result<_, _>>()?,
        })
    }
}

#[derive(Debug, Clone)]
pub enum Attribute {
    Doc(String),
    Cfg(String, Span),
}

impl Eq for Attribute {}

impl PartialEq for Attribute {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Doc(l0), Self::Doc(r0)) => l0 == r0,
            (Self::Cfg(l0, _), Self::Cfg(r0, _)) => l0 == r0,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub attribute_list: AttributeList,
    pub identifier: syn::Ident,
    pub block_item_list: BlockItemList,
    pub object_list: ObjectList,
}

impl Parse for Block {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attribute_list = input.parse()?;
        input.parse::<kw::block>()?;
        let identifier = input.parse()?;

        let braced_input;
        braced!(braced_input in input);

        let block_item_list = braced_input.parse()?;
        let object_list = braced_input.parse()?;

        Ok(Self {
            attribute_list,
            identifier,
            block_item_list,
            object_list,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockItemList {
    pub block_items: Vec<BlockItem>,
}

impl Parse for BlockItemList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut block_items = Vec::new();

        let err_if_contains = |items: &[BlockItem], discr: Discriminant<BlockItem>, span: Span| {
            if items.iter().any(|i| core::mem::discriminant(i) == discr) {
                Err(syn::Error::new(span, "duplicate item found"))
            } else {
                Ok(())
            }
        };

        while !input.is_empty() && input.peek(Token![const]) {
            let item = if input.peek2(kw::ADDRESS_OFFSET) {
                input.parse::<Token![const]>()?;

                err_if_contains(
                    &block_items,
                    core::mem::discriminant(&BlockItem::AddressOffset(LitInt::new(
                        "0",
                        Span::call_site(),
                    ))),
                    input.span(),
                )?;

                input.parse::<kw::ADDRESS_OFFSET>()?;
                input.parse::<Token![=]>()?;
                let value = input.parse()?;
                input.parse::<Token![;]>()?;

                BlockItem::AddressOffset(value)
            } else if input.peek2(kw::REPEAT) {
                input.parse::<Token![const]>()?;

                err_if_contains(
                    &block_items,
                    core::mem::discriminant(&BlockItem::Repeat(Repeat {
                        count: RepeatCount::Value(LitInt::new("0", Span::call_site())),
                        stride: LitInt::new("0", Span::call_site()),
                    })),
                    input.span(),
                )?;

                BlockItem::Repeat(input.parse()?)
            } else {
                return Err(syn::Error::new(
                    input.span(),
                    "Invalid value. Must be an `ADDRESS_OFFSET` or `REPEAT`",
                ));
            };

            block_items.push(item);
        }

        Ok(Self { block_items })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockItem {
    AddressOffset(LitInt),
    Repeat(Repeat),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Register {
    pub attribute_list: AttributeList,
    pub identifier: syn::Ident,
    pub register_item_list: RegisterItemList,
    pub field_list: FieldList,
}

impl Parse for Register {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attribute_list = input.parse()?;
        input.parse::<kw::register>()?;
        let identifier = input.parse()?;

        let braced_input;
        braced!(braced_input in input);

        let register_item_list = braced_input.parse()?;
        let field_list = braced_input.parse()?;

        Ok(Self {
            attribute_list,
            identifier,
            register_item_list,
            field_list,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RegisterItemList {
    pub register_items: Vec<RegisterItem>,
}

#[cfg(test)]
impl RegisterItemList {
    pub fn new() -> Self {
        Self {
            register_items: Vec::new(),
        }
    }
}

impl Parse for RegisterItemList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut register_items = Vec::new();

        let err_if_contains =
            |items: &[RegisterItem], discr: Discriminant<RegisterItem>, span: Span| {
                if items.iter().any(|i| core::mem::discriminant(i) == discr) {
                    Err(syn::Error::new(span, "duplicate item found"))
                } else {
                    Ok(())
                }
            };

        loop {
            if input.peek(Token![type]) {
                input.parse::<Token![type]>()?;

                let lookahead = input.lookahead1();

                if lookahead.peek(kw::Access) {
                    err_if_contains(
                        &register_items,
                        core::mem::discriminant(&RegisterItem::Access(Access::RW)),
                        input.span(),
                    )?;

                    input.parse::<kw::Access>()?;
                    input.parse::<Token![=]>()?;
                    let value = input.parse()?;
                    input.parse::<Token![;]>()?;
                    register_items.push(RegisterItem::Access(value));
                } else if lookahead.peek(kw::ByteOrder) {
                    err_if_contains(
                        &register_items,
                        core::mem::discriminant(&RegisterItem::ByteOrder(ByteOrder::LE)),
                        input.span(),
                    )?;

                    input.parse::<kw::ByteOrder>()?;
                    input.parse::<Token![=]>()?;
                    let value = input.parse()?;
                    input.parse::<Token![;]>()?;
                    register_items.push(RegisterItem::ByteOrder(value));
                } else if lookahead.peek(kw::BitOrder) {
                    err_if_contains(
                        &register_items,
                        core::mem::discriminant(&RegisterItem::BitOrder(BitOrder::LSB0)),
                        input.span(),
                    )?;

                    input.parse::<kw::BitOrder>()?;
                    input.parse::<Token![=]>()?;
                    let value = input.parse()?;
                    input.parse::<Token![;]>()?;
                    register_items.push(RegisterItem::BitOrder(value));
                } else {
                    return Err(lookahead.error());
                }
            } else if input.peek(Token![const]) {
                input.parse::<Token![const]>()?;

                let lookahead = input.lookahead1();

                if lookahead.peek(kw::ADDRESS) {
                    err_if_contains(
                        &register_items,
                        core::mem::discriminant(&RegisterItem::Address(LitInt::new(
                            "0",
                            Span::call_site(),
                        ))),
                        input.span(),
                    )?;

                    input.parse::<kw::ADDRESS>()?;
                    input.parse::<Token![=]>()?;
                    let value = input.parse()?;
                    input.parse::<Token![;]>()?;
                    register_items.push(RegisterItem::Address(value));
                } else if lookahead.peek(kw::SIZE_BITS) {
                    err_if_contains(
                        &register_items,
                        core::mem::discriminant(&RegisterItem::SizeBits(LitInt::new(
                            "0",
                            Span::call_site(),
                        ))),
                        input.span(),
                    )?;

                    input.parse::<kw::SIZE_BITS>()?;
                    input.parse::<Token![=]>()?;
                    let value = input.parse()?;
                    input.parse::<Token![;]>()?;
                    register_items.push(RegisterItem::SizeBits(value));
                } else if lookahead.peek(kw::RESET_VALUE) {
                    err_if_contains(
                        &register_items,
                        core::mem::discriminant(&RegisterItem::ResetValueInt(LitInt::new(
                            "0",
                            Span::call_site(),
                        ))),
                        input.span(),
                    )?;
                    err_if_contains(
                        &register_items,
                        core::mem::discriminant(&RegisterItem::ResetValueArray(Vec::new())),
                        input.span(),
                    )?;

                    input.parse::<kw::RESET_VALUE>()?;
                    input.parse::<Token![=]>()?;

                    let lookahead = input.lookahead1();
                    let value = if lookahead.peek(syn::LitInt) {
                        RegisterItem::ResetValueInt(input.parse()?)
                    } else if lookahead.peek(syn::token::Bracket) {
                        let bracket_input;
                        bracketed!(bracket_input in input);

                        let elems =
                            Punctuated::<syn::LitInt, Token![,]>::parse_terminated(&bracket_input)?;

                        let mut reset_data = Vec::new();

                        for elem in elems {
                            reset_data.push(elem.base10_parse()?);
                        }

                        RegisterItem::ResetValueArray(reset_data)
                    } else {
                        return Err(lookahead.error());
                    };
                    input.parse::<Token![;]>()?;
                    register_items.push(value);
                } else if lookahead.peek(kw::REPEAT) {
                    err_if_contains(
                        &register_items,
                        core::mem::discriminant(&RegisterItem::Repeat(Repeat {
                            count: RepeatCount::Value(LitInt::new("0", Span::call_site())),
                            stride: LitInt::new("0", Span::call_site()),
                        })),
                        input.span(),
                    )?;

                    register_items.push(RegisterItem::Repeat(input.parse()?));
                } else if lookahead.peek(kw::ALLOW_BIT_OVERLAP) {
                    err_if_contains(
                        &register_items,
                        core::mem::discriminant(&RegisterItem::AllowBitOverlap(LitBool::new(
                            false,
                            Span::call_site(),
                        ))),
                        input.span(),
                    )?;

                    input.parse::<kw::ALLOW_BIT_OVERLAP>()?;
                    input.parse::<Token![=]>()?;
                    let value = input.parse()?;
                    input.parse::<Token![;]>()?;
                    register_items.push(RegisterItem::AllowBitOverlap(value));
                } else if lookahead.peek(kw::ALLOW_ADDRESS_OVERLAP) {
                    err_if_contains(
                        &register_items,
                        core::mem::discriminant(&RegisterItem::AllowAddressOverlap(LitBool::new(
                            false,
                            Span::call_site(),
                        ))),
                        input.span(),
                    )?;

                    input.parse::<kw::ALLOW_ADDRESS_OVERLAP>()?;
                    input.parse::<Token![=]>()?;
                    let value = input.parse()?;
                    input.parse::<Token![;]>()?;
                    register_items.push(RegisterItem::AllowAddressOverlap(value));
                } else {
                    return Err(lookahead.error());
                }
            } else {
                break;
            }
        }

        Ok(Self { register_items })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterItem {
    Access(Access),
    ByteOrder(ByteOrder),
    BitOrder(BitOrder),
    Address(LitInt),
    SizeBits(LitInt),
    ResetValueInt(LitInt),
    ResetValueArray(Vec<u8>),
    Repeat(Repeat),
    AllowBitOverlap(LitBool),
    AllowAddressOverlap(LitBool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    RW,
    RO,
    WO,
}

impl Parse for Access {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(kw::ReadWrite) {
            input.parse::<kw::ReadWrite>()?;
            Ok(Self::RW)
        } else if lookahead.peek(kw::RW) {
            input.parse::<kw::RW>()?;
            Ok(Self::RW)
        } else if lookahead.peek(kw::ReadOnly) {
            input.parse::<kw::ReadOnly>()?;
            Ok(Self::RO)
        } else if lookahead.peek(kw::RO) {
            input.parse::<kw::RO>()?;
            Ok(Self::RO)
        } else if lookahead.peek(kw::WriteOnly) {
            input.parse::<kw::WriteOnly>()?;
            Ok(Self::WO)
        } else if lookahead.peek(kw::WO) {
            input.parse::<kw::WO>()?;
            Ok(Self::WO)
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOrder {
    LE,
    BE,
}

impl Parse for ByteOrder {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(kw::LE) {
            input.parse::<kw::LE>()?;
            Ok(Self::LE)
        } else if lookahead.peek(kw::BE) {
            input.parse::<kw::BE>()?;
            Ok(Self::BE)
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitOrder {
    LSB0,
    MSB0,
}

impl Parse for BitOrder {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(kw::LSB0) {
            input.parse::<kw::LSB0>()?;
            Ok(Self::LSB0)
        } else if lookahead.peek(kw::MSB0) {
            input.parse::<kw::MSB0>()?;
            Ok(Self::MSB0)
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FieldList {
    pub fields: Vec<Field>,
}

#[cfg(test)]
impl FieldList {
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }
}

impl Parse for FieldList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let punctuated_fields = Punctuated::<Field, Token![,]>::parse_terminated(input)?;

        Ok(Self {
            fields: punctuated_fields.into_iter().collect(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub attribute_list: AttributeList,
    pub identifier: syn::Ident,
    pub access: Option<Access>,
    pub base_type: BaseType,
    pub conversion: Option<Conversion>,
    pub field_address: FieldAddress,
}

impl Parse for Field {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attribute_list = input.parse()?;
        let identifier = input.parse()?;
        input.parse::<Token![:]>()?;
        let access = input.parse::<Access>().ok();
        let base_type = input.parse()?;

        let conversion = if input.peek(Token![as]) {
            Some(input.parse()?)
        } else {
            None
        };

        input.parse::<Token![=]>()?;

        let field_address = input.parse()?;

        Ok(Self {
            attribute_list,
            identifier,
            base_type,
            conversion,
            access,
            field_address,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Conversion {
    Direct {
        path: syn::Path,
        use_try: bool,
    },
    Enum {
        identifier: syn::Ident,
        enum_variant_list: EnumVariantList,
        use_try: bool,
    },
}

impl Parse for Conversion {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![as]>()?;

        let use_try = input.parse::<Token![try]>().is_ok();

        if input.parse::<Token![enum]>().is_err() {
            return Ok(Self::Direct {
                path: input.parse::<syn::Path>()?,
                use_try,
            });
        }

        let identifier = input.parse()?;

        let braced_input;
        braced!(braced_input in input);

        let enum_variant_list = braced_input.parse()?;

        Ok(Self::Enum {
            identifier,
            enum_variant_list,
            use_try,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariantList {
    pub variants: Vec<EnumVariant>,
}

impl Parse for EnumVariantList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let variants = Punctuated::<EnumVariant, Token![,]>::parse_terminated(input)?;
        Ok(Self {
            variants: variants.into_iter().collect(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant {
    pub attribute_list: AttributeList,
    pub identifier: syn::Ident,
    pub enum_value: Option<EnumValue>,
}

impl Parse for EnumVariant {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attribute_list = input.parse()?;
        let identifier = input.parse()?;

        let enum_value = if input.parse::<Token![=]>().is_ok() {
            Some(input.parse()?)
        } else {
            None
        };

        Ok(Self {
            attribute_list,
            identifier,
            enum_value,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnumValue {
    Specified(LitInt),
    Default,
    CatchAll,
}

impl Parse for EnumValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if let Ok(specification) = input.parse::<LitInt>() {
            Ok(Self::Specified(specification))
        } else if input.parse::<kw::default>().is_ok() {
            Ok(Self::Default)
        } else if input.parse::<kw::catch_all>().is_ok() {
            Ok(Self::CatchAll)
        } else {
            Err(syn::Error::new(
                input.span(),
                "Specifier not recognized. Must be an integer literal, `default` or `catch_all`",
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldAddress {
    Integer(LitInt),
    Range { start: LitInt, end: LitInt },
    RangeInclusive { start: LitInt, end: LitInt },
}

impl Parse for FieldAddress {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start = input.parse()?;

        if input.peek(Token![..=]) {
            input.parse::<Token![..=]>()?;
            let end = input.parse()?;
            Ok(Self::RangeInclusive { start, end })
        } else if input.peek(Token![..]) {
            input.parse::<Token![..]>()?;
            let end = input.parse()?;
            Ok(Self::Range { start, end })
        } else {
            Ok(FieldAddress::Integer(start))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseType {
    Bool,
    Uint,
    Int,
}

impl BaseType {
    /// Returns `true` if the base type is [`Bool`].
    ///
    /// [`Bool`]: BaseType::Bool
    #[must_use]
    pub const fn is_bool(&self) -> bool {
        matches!(self, Self::Bool)
    }
}

impl Parse for BaseType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(kw::bool) {
            input.parse::<kw::bool>()?;
            Ok(BaseType::Bool)
        } else if lookahead.peek(kw::uint) {
            input.parse::<kw::uint>()?;
            Ok(BaseType::Uint)
        } else if lookahead.peek(kw::int) {
            input.parse::<kw::int>()?;
            Ok(BaseType::Int)
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub attribute_list: AttributeList,
    pub identifier: syn::Ident,
    pub value: Option<CommandValue>,
}

impl Parse for Command {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attribute_list = input.parse()?;
        input.parse::<kw::command>()?;
        let identifier = input.parse()?;

        let value = if !input.is_empty() {
            Some(input.parse()?)
        } else {
            None
        };

        Ok(Self {
            attribute_list,
            identifier,
            value,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandValue {
    Basic(LitInt),
    Extended {
        command_item_list: CommandItemList,
        in_field_list: Option<FieldList>,
        out_field_list: Option<FieldList>,
    },
}

impl Parse for CommandValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.parse::<Token![=]>().is_ok() {
            return Ok(CommandValue::Basic(input.parse()?));
        }

        let braced_input;
        braced!(braced_input in input);

        let command_item_list = braced_input.parse()?;

        let in_field_list = if braced_input.parse::<Token![in]>().is_ok() {
            let braced_input_inner;
            braced!(braced_input_inner in braced_input);

            Some(braced_input_inner.parse()?)
        } else {
            None
        };

        let _ = braced_input.parse::<Token![,]>();

        let out_field_list = if braced_input.parse::<kw::out>().is_ok() {
            let braced_input_inner;
            braced!(braced_input_inner in braced_input);

            Some(braced_input_inner.parse()?)
        } else {
            None
        };

        let _ = braced_input.parse::<Token![,]>();

        if !braced_input.is_empty() {
            return Err(syn::Error::new(
                braced_input.span(),
                "Did not expect any more tokens",
            ));
        }

        Ok(Self::Extended {
            command_item_list,
            in_field_list,
            out_field_list,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandItemList {
    pub items: Vec<CommandItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandItem {
    ByteOrder(ByteOrder),
    BitOrder(BitOrder),
    Address(LitInt),
    SizeBitsIn(LitInt),
    SizeBitsOut(LitInt),
    Repeat(Repeat),
    AllowBitOverlap(LitBool),
    AllowAddressOverlap(LitBool),
}

impl Parse for CommandItemList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();

        let err_if_contains =
            |items: &[CommandItem], discr: Discriminant<CommandItem>, span: Span| {
                if items.iter().any(|i| core::mem::discriminant(i) == discr) {
                    Err(syn::Error::new(span, "duplicate item found"))
                } else {
                    Ok(())
                }
            };

        loop {
            if input.parse::<Token![type]>().is_ok() {
                let lookahead = input.lookahead1();

                if lookahead.peek(kw::ByteOrder) {
                    err_if_contains(
                        &items,
                        core::mem::discriminant(&CommandItem::ByteOrder(ByteOrder::BE)),
                        input.span(),
                    )?;

                    input.parse::<kw::ByteOrder>()?;
                    input.parse::<Token![=]>()?;
                    let value = input.parse()?;
                    input.parse::<Token![;]>()?;
                    items.push(CommandItem::ByteOrder(value));
                } else if lookahead.peek(kw::BitOrder) {
                    err_if_contains(
                        &items,
                        core::mem::discriminant(&CommandItem::BitOrder(BitOrder::LSB0)),
                        input.span(),
                    )?;

                    input.parse::<kw::BitOrder>()?;
                    input.parse::<Token![=]>()?;
                    let value = input.parse()?;
                    input.parse::<Token![;]>()?;
                    items.push(CommandItem::BitOrder(value));
                } else {
                    return Err(lookahead.error());
                }
            } else if input.peek(Token![const]) {
                input.parse::<Token![const]>()?;

                let lookahead = input.lookahead1();

                if lookahead.peek(kw::ADDRESS) {
                    err_if_contains(
                        &items,
                        core::mem::discriminant(&CommandItem::Address(LitInt::new(
                            "0",
                            Span::call_site(),
                        ))),
                        input.span(),
                    )?;

                    input.parse::<kw::ADDRESS>()?;
                    input.parse::<Token![=]>()?;
                    let value = input.parse()?;
                    input.parse::<Token![;]>()?;
                    items.push(CommandItem::Address(value));
                } else if lookahead.peek(kw::SIZE_BITS_IN) {
                    err_if_contains(
                        &items,
                        core::mem::discriminant(&CommandItem::SizeBitsIn(LitInt::new(
                            "0",
                            Span::call_site(),
                        ))),
                        input.span(),
                    )?;

                    input.parse::<kw::SIZE_BITS_IN>()?;
                    input.parse::<Token![=]>()?;
                    let value = input.parse()?;
                    input.parse::<Token![;]>()?;
                    items.push(CommandItem::SizeBitsIn(value));
                } else if lookahead.peek(kw::SIZE_BITS_OUT) {
                    err_if_contains(
                        &items,
                        core::mem::discriminant(&CommandItem::SizeBitsOut(LitInt::new(
                            "0",
                            Span::call_site(),
                        ))),
                        input.span(),
                    )?;

                    input.parse::<kw::SIZE_BITS_OUT>()?;
                    input.parse::<Token![=]>()?;
                    let value = input.parse()?;
                    input.parse::<Token![;]>()?;
                    items.push(CommandItem::SizeBitsOut(value));
                } else if lookahead.peek(kw::REPEAT) {
                    err_if_contains(
                        &items,
                        core::mem::discriminant(&CommandItem::Repeat(Repeat {
                            count: RepeatCount::Value(LitInt::new("0", Span::call_site())),
                            stride: LitInt::new("0", Span::call_site()),
                        })),
                        input.span(),
                    )?;

                    items.push(CommandItem::Repeat(input.parse()?));
                } else if lookahead.peek(kw::ALLOW_BIT_OVERLAP) {
                    err_if_contains(
                        &items,
                        core::mem::discriminant(&CommandItem::AllowBitOverlap(LitBool::new(
                            false,
                            Span::call_site(),
                        ))),
                        input.span(),
                    )?;

                    input.parse::<kw::ALLOW_BIT_OVERLAP>()?;
                    input.parse::<Token![=]>()?;
                    let value = input.parse()?;
                    input.parse::<Token![;]>()?;
                    items.push(CommandItem::AllowBitOverlap(value));
                } else if lookahead.peek(kw::ALLOW_ADDRESS_OVERLAP) {
                    err_if_contains(
                        &items,
                        core::mem::discriminant(&CommandItem::AllowAddressOverlap(LitBool::new(
                            false,
                            Span::call_site(),
                        ))),
                        input.span(),
                    )?;

                    input.parse::<kw::ALLOW_ADDRESS_OVERLAP>()?;
                    input.parse::<Token![=]>()?;
                    let value = input.parse()?;
                    input.parse::<Token![;]>()?;
                    items.push(CommandItem::AllowAddressOverlap(value));
                } else {
                    return Err(lookahead.error());
                }
            } else {
                break;
            }
        }

        Ok(Self { items })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repeat {
    pub count: RepeatCount,
    pub stride: LitInt,
}

impl Parse for Repeat {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<kw::REPEAT>()?;
        input.parse::<Token![=]>()?;

        let braced_input;
        braced!(braced_input in input);

        braced_input.parse::<kw::count>()?;
        braced_input.parse::<Token![:]>()?;
        let count = braced_input.parse()?;
        braced_input.parse::<Token![,]>()?;

        braced_input.parse::<kw::stride>()?;
        braced_input.parse::<Token![:]>()?;
        let stride = braced_input.parse()?;
        if braced_input.peek(Token![,]) {
            braced_input.parse::<Token![,]>()?;
        }

        input.parse::<Token![;]>()?;

        Ok(Repeat { count, stride })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepeatCount {
    Value(LitInt),
    Conversion(Conversion),
}

impl Parse for RepeatCount {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(LitInt) {
            Ok(RepeatCount::Value(input.parse()?))
        } else {
            input.parse::<kw::usize>()?;
            Ok(RepeatCount::Conversion(input.parse()?))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buffer {
    pub attribute_list: AttributeList,
    pub identifier: syn::Ident,
    pub access: Option<Access>,
    pub address: Option<LitInt>,
}

impl Parse for Buffer {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attribute_list = input.parse()?;
        input.parse::<kw::buffer>()?;
        let identifier = input.parse()?;

        let access = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        let address = if input.parse::<Token![=]>().is_ok() {
            Some(input.parse()?)
        } else {
            None
        };

        Ok(Self {
            attribute_list,
            identifier,
            access,
            address,
        })
    }
}

mod kw {
    syn::custom_keyword!(config);

    // Objects
    syn::custom_keyword!(block);
    syn::custom_keyword!(register);
    syn::custom_keyword!(command);
    syn::custom_keyword!(buffer);

    syn::custom_keyword!(ADDRESS);
    syn::custom_keyword!(ADDRESS_OFFSET);
    syn::custom_keyword!(SIZE_BITS);
    syn::custom_keyword!(SIZE_BITS_IN);
    syn::custom_keyword!(SIZE_BITS_OUT);
    syn::custom_keyword!(RESET_VALUE);
    syn::custom_keyword!(ALLOW_BIT_OVERLAP);
    syn::custom_keyword!(ALLOW_ADDRESS_OVERLAP);

    // Repeat
    syn::custom_keyword!(REPEAT);
    syn::custom_keyword!(count);
    syn::custom_keyword!(stride);

    // Global config items
    syn::custom_keyword!(DefaultRegisterAccess);
    syn::custom_keyword!(DefaultFieldAccess);
    syn::custom_keyword!(DefaultBufferAccess);
    syn::custom_keyword!(DefaultByteOrder);
    syn::custom_keyword!(DefaultBitOrder);
    syn::custom_keyword!(RegisterAddressType);
    syn::custom_keyword!(CommandAddressType);
    syn::custom_keyword!(BufferAddressType);
    syn::custom_keyword!(NameWordBoundaries);
    syn::custom_keyword!(DefmtFeature);

    // Access
    syn::custom_keyword!(Access);
    syn::custom_keyword!(RW);
    syn::custom_keyword!(ReadWrite);
    syn::custom_keyword!(RO);
    syn::custom_keyword!(ReadOnly);
    syn::custom_keyword!(WO);
    syn::custom_keyword!(WriteOnly);

    // ByteOrder
    syn::custom_keyword!(ByteOrder);
    syn::custom_keyword!(LE);
    syn::custom_keyword!(BE);

    // BitOrder
    syn::custom_keyword!(BitOrder);
    syn::custom_keyword!(LSB0);
    syn::custom_keyword!(MSB0);

    // BaseType
    syn::custom_keyword!(bool);
    syn::custom_keyword!(uint);
    syn::custom_keyword!(int);
    syn::custom_keyword!(usize);

    // EnumValue
    syn::custom_keyword!(default);
    syn::custom_keyword!(catch_all);

    // CommandValue
    syn::custom_keyword!(out);
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use quote::format_ident;
    use syn::{Ident, parse_quote};

    use super::*;

    #[test]
    fn parse_access() {
        assert_eq!(syn::parse_str::<Access>("RW").unwrap(), Access::RW);
        assert_eq!(syn::parse_str::<Access>("ReadWrite").unwrap(), Access::RW);
        assert_eq!(syn::parse_str::<Access>("RO").unwrap(), Access::RO);
        assert_eq!(syn::parse_str::<Access>("ReadOnly").unwrap(), Access::RO);
        assert_eq!(syn::parse_str::<Access>("WO").unwrap(), Access::WO);
        assert_eq!(syn::parse_str::<Access>("WriteOnly").unwrap(), Access::WO);

        assert_eq!(
            syn::parse_str::<Access>("ABCD").unwrap_err().to_string(),
            "expected one of: `ReadWrite`, `RW`, `ReadOnly`, `RO`, `WriteOnly`, `WO`"
        );
    }

    #[test]
    fn parse_byte_order() {
        assert_eq!(syn::parse_str::<ByteOrder>("LE").unwrap(), ByteOrder::LE);
        assert_eq!(syn::parse_str::<ByteOrder>("BE").unwrap(), ByteOrder::BE);

        assert_eq!(
            syn::parse_str::<ByteOrder>("ABCD").unwrap_err().to_string(),
            "expected `LE` or `BE`"
        );
    }

    #[test]
    fn parse_bit_order() {
        assert_eq!(syn::parse_str::<BitOrder>("LSB0").unwrap(), BitOrder::LSB0);
        assert_eq!(syn::parse_str::<BitOrder>("MSB0").unwrap(), BitOrder::MSB0);

        assert_eq!(
            syn::parse_str::<BitOrder>("ABCD").unwrap_err().to_string(),
            "expected `LSB0` or `MSB0`"
        );
    }

    #[test]
    fn parse_base_type() {
        assert_eq!(syn::parse_str::<BaseType>("bool").unwrap(), BaseType::Bool);
        assert_eq!(syn::parse_str::<BaseType>("uint").unwrap(), BaseType::Uint);
        assert_eq!(syn::parse_str::<BaseType>("int").unwrap(), BaseType::Int);

        assert_eq!(
            syn::parse_str::<BaseType>("ABCD").unwrap_err().to_string(),
            "expected one of: `bool`, `uint`, `int`"
        );
    }

    #[test]
    fn parse_enum_value() {
        assert_eq!(
            syn::parse_str::<EnumValue>("55").unwrap(),
            EnumValue::Specified(LitInt::new("55", Span::call_site()))
        );
        assert_eq!(
            syn::parse_str::<EnumValue>("default").unwrap(),
            EnumValue::Default
        );
        assert_eq!(
            syn::parse_str::<EnumValue>("catch_all").unwrap(),
            EnumValue::CatchAll
        );

        assert_eq!(
            syn::parse_str::<EnumValue>("ABCD").unwrap_err().to_string(),
            "Specifier not recognized. Must be an integer literal, `default` or `catch_all`"
        );
    }

    #[test]
    fn parse_repeat() {
        assert_eq!(
            syn::parse_str::<Repeat>("REPEAT = { count: 55, stride: 0x123, };").unwrap(),
            Repeat {
                count: RepeatCount::Value(LitInt::new("55", Span::call_site())),
                stride: LitInt::new("0x123", Span::call_site())
            }
        );
        assert_eq!(
            syn::parse_str::<Repeat>("REPEAT = { count: 55, stride: 0x123 };").unwrap(),
            Repeat {
                count: RepeatCount::Value(LitInt::new("55", Span::call_site())),
                stride: LitInt::new("0x123", Span::call_site())
            }
        );

        assert_eq!(
            syn::parse_str::<Repeat>("ABCD").unwrap_err().to_string(),
            "expected `REPEAT`"
        );
        assert_eq!(
            syn::parse_str::<Repeat>("REPEAT = { count: 55 stride: 0x123 };")
                .unwrap_err()
                .to_string(),
            "expected `,`"
        );
        assert_eq!(
            syn::parse_str::<Repeat>("REPEAT = ")
                .unwrap_err()
                .to_string(),
            "unexpected end of input, expected curly braces"
        );

        assert_eq!(
            syn::parse_str::<Repeat>(
                "REPEAT = { count: usize as enum R { A, B }, stride: 0x123 };"
            )
            .unwrap(),
            Repeat {
                count: RepeatCount::Conversion(Conversion::Enum {
                    identifier: format_ident!("R"),
                    enum_variant_list: EnumVariantList {
                        variants: vec![
                            EnumVariant {
                                attribute_list: AttributeList::default(),
                                identifier: format_ident!("A"),
                                enum_value: None
                            },
                            EnumVariant {
                                attribute_list: AttributeList::default(),
                                identifier: format_ident!("B"),
                                enum_value: None
                            }
                        ]
                    },
                    use_try: false
                }),
                stride: LitInt::new("0x123", Span::call_site())
            }
        );

        assert_eq!(
            syn::parse_str::<Repeat>("REPEAT = { count: usize as Foo, stride: 0x123 };").unwrap(),
            Repeat {
                count: RepeatCount::Conversion(Conversion::Direct {
                    path: parse_quote!(Foo),
                    use_try: false
                }),
                stride: LitInt::new("0x123", Span::call_site())
            }
        );
    }

    #[test]
    fn parse_command_item_list() {
        assert_eq!(
            syn::parse_str::<CommandItemList>("").unwrap(),
            CommandItemList { items: vec![] }
        );

        assert_eq!(
            syn::parse_str::<CommandItemList>("type ByteOrder = LE;").unwrap(),
            CommandItemList {
                items: vec![CommandItem::ByteOrder(ByteOrder::LE)]
            }
        );

        assert_eq!(
            syn::parse_str::<CommandItemList>("type BitOrder = LSB0;\nconst ADDRESS = 123;")
                .unwrap(),
            CommandItemList {
                items: vec![
                    CommandItem::BitOrder(BitOrder::LSB0),
                    CommandItem::Address(LitInt::new("123", Span::call_site()))
                ]
            }
        );

        assert_eq!(
            syn::parse_str::<CommandItemList>(
                "const SIZE_BITS_IN = 16;\nconst SIZE_BITS_OUT = 32;\nconst REPEAT = { count: 2, stride: 2 };"
            )
            .unwrap(),
            CommandItemList {
                items: vec![
                    CommandItem::SizeBitsIn(LitInt::new("16", Span::call_site())),
                    CommandItem::SizeBitsOut(LitInt::new("32", Span::call_site())),
                    CommandItem::Repeat(Repeat {
                        count: RepeatCount::Value(LitInt::new("2", Span::call_site())),
                        stride: LitInt::new("2", Span::call_site())
                    })
                ]
            }
        );

        assert_eq!(
            syn::parse_str::<CommandItemList>("const ABC = 16;")
                .unwrap_err()
                .to_string(),
            "expected one of: `ADDRESS`, `SIZE_BITS_IN`, `SIZE_BITS_OUT`, `REPEAT`, `ALLOW_BIT_OVERLAP`, `ALLOW_ADDRESS_OVERLAP`"
        );

        assert_eq!(
            syn::parse_str::<CommandItemList>("type ABC = 16;")
                .unwrap_err()
                .to_string(),
            "expected `ByteOrder` or `BitOrder`"
        );

        assert_eq!(
            syn::parse_str::<CommandItemList>("type ByteOrder = LE; type ByteOrder = LE;")
                .unwrap_err()
                .to_string(),
            "duplicate item found"
        );
    }

    #[test]
    fn parse_field_address() {
        assert_eq!(
            syn::parse_str::<FieldAddress>("55").unwrap(),
            FieldAddress::Integer(LitInt::new("55", Span::call_site()))
        );
        assert_eq!(
            syn::parse_str::<FieldAddress>("55..=0x123").unwrap(),
            FieldAddress::RangeInclusive {
                start: LitInt::new("55", Span::call_site()),
                end: LitInt::new("0x123", Span::call_site())
            }
        );
        assert_eq!(
            syn::parse_str::<FieldAddress>("55..0x123").unwrap(),
            FieldAddress::Range {
                start: LitInt::new("55", Span::call_site()),
                end: LitInt::new("0x123", Span::call_site())
            }
        );

        assert_eq!(
            syn::parse_str::<FieldAddress>("ABCD")
                .unwrap_err()
                .to_string(),
            "expected integer literal"
        );
    }

    #[test]
    fn parse_buffer() {
        assert_eq!(
            syn::parse_str::<Buffer>("buffer TestBuffer = 0x123").unwrap(),
            Buffer {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("TestBuffer", Span::call_site()),
                access: None,
                address: Some(LitInt::new("0x123", Span::call_site())),
            }
        );

        assert_eq!(
            syn::parse_str::<Buffer>("buffer TestBuffer").unwrap(),
            Buffer {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("TestBuffer", Span::call_site()),
                access: None,
                address: None,
            }
        );

        assert_eq!(
            syn::parse_str::<Buffer>("buffer TestBuffer: WO").unwrap(),
            Buffer {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("TestBuffer", Span::call_site()),
                access: Some(Access::WO),
                address: None,
            }
        );

        assert_eq!(
            syn::parse_str::<Buffer>("buffer TestBuffer =")
                .unwrap_err()
                .to_string(),
            "unexpected end of input, expected integer literal"
        );

        assert_eq!(
            syn::parse_str::<Buffer>("/// A test buffer\nbuffer TestBuffer: RO = 0x123").unwrap(),
            Buffer {
                attribute_list: AttributeList {
                    attributes: vec![Attribute::Doc(" A test buffer".into())]
                },
                identifier: Ident::new("TestBuffer", Span::call_site()),
                access: Some(Access::RO),
                address: Some(LitInt::new("0x123", Span::call_site())),
            }
        );
    }

    #[test]
    fn parse_field() {
        assert_eq!(
            syn::parse_str::<Field>("TestField: ReadOnly int = 0x123").unwrap(),
            Field {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("TestField", Span::call_site()),
                access: Some(Access::RO),
                base_type: BaseType::Int,
                conversion: None,
                field_address: FieldAddress::Integer(LitInt::new("0x123", Span::call_site()))
            }
        );

        assert_eq!(
            syn::parse_str::<Field>("ExsitingType: RW uint as crate::module::foo::Bar = 0x1234")
                .unwrap(),
            Field {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("ExsitingType", Span::call_site()),
                access: Some(Access::RW),
                base_type: BaseType::Uint,
                conversion: Some(Conversion::Direct {
                    path: syn::parse_str("crate::module::foo::Bar").unwrap(),
                    use_try: false,
                }),
                field_address: FieldAddress::Integer(LitInt::new("0x1234", Span::call_site()))
            }
        );

        assert_eq!(
            syn::parse_str::<Field>(
                "ExsitingType: RW uint as try crate::module::foo::Bar = 0x1234"
            )
            .unwrap(),
            Field {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("ExsitingType", Span::call_site()),
                access: Some(Access::RW),
                base_type: BaseType::Uint,
                conversion: Some(Conversion::Direct {
                    path: syn::parse_str("crate::module::foo::Bar").unwrap(),
                    use_try: true,
                }),
                field_address: FieldAddress::Integer(LitInt::new("0x1234", Span::call_site()))
            }
        );

        assert_eq!(
            syn::parse_str::<Field>(
                "ExsitingType: RW uint as enum crate::module::foo::Bar = 0x1234"
            )
            .unwrap_err()
            .to_string(),
            "expected identifier, found keyword `crate`"
        );

        assert_eq!(
            syn::parse_str::<Field>("ExsitingType: RW uint as enum Bar { } = 0x1234").unwrap(),
            Field {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("ExsitingType", Span::call_site()),
                access: Some(Access::RW),
                base_type: BaseType::Uint,
                conversion: Some(Conversion::Enum {
                    identifier: Ident::new("Bar", Span::call_site()),
                    enum_variant_list: EnumVariantList {
                        variants: Vec::new()
                    },
                    use_try: false,
                }),
                field_address: FieldAddress::Integer(LitInt::new("0x1234", Span::call_site()))
            }
        );
    }

    #[test]
    fn parse_enum_variant_list() {
        assert_eq!(
            syn::parse_str::<EnumVariantList>(
                "A, B = 0xFF,\n/// This is C\nC = default, D = catch_all"
            )
            .unwrap(),
            EnumVariantList {
                variants: vec![
                    EnumVariant {
                        attribute_list: AttributeList::new(),
                        identifier: Ident::new("A", Span::call_site()),
                        enum_value: None
                    },
                    EnumVariant {
                        attribute_list: AttributeList::new(),
                        identifier: Ident::new("B", Span::call_site()),
                        enum_value: Some(EnumValue::Specified(LitInt::new(
                            "0xFF",
                            Span::call_site()
                        )))
                    },
                    EnumVariant {
                        attribute_list: AttributeList {
                            attributes: vec![Attribute::Doc(" This is C".into())]
                        },
                        identifier: Ident::new("C", Span::call_site()),
                        enum_value: Some(EnumValue::Default)
                    },
                    EnumVariant {
                        attribute_list: AttributeList::new(),
                        identifier: Ident::new("D", Span::call_site()),
                        enum_value: Some(EnumValue::CatchAll)
                    },
                ]
            }
        );
    }

    #[test]
    fn parse_command() {
        assert_eq!(
            syn::parse_str::<Command>("/// A command!\n#[cfg(feature = \"std\")]\ncommand Foo = 5")
                .unwrap(),
            Command {
                attribute_list: AttributeList {
                    attributes: vec![
                        Attribute::Doc(" A command!".into()),
                        Attribute::Cfg("feature = \"std\"".into(), Span::call_site()),
                    ]
                },
                identifier: Ident::new("Foo", Span::call_site()),
                value: Some(CommandValue::Basic(LitInt::new("5", Span::call_site()))),
            }
        );
        assert_eq!(
            syn::parse_str::<Command>("command Bar { type BitOrder = LSB0; }").unwrap(),
            Command {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("Bar", Span::call_site()),
                value: Some(CommandValue::Extended {
                    command_item_list: CommandItemList {
                        items: vec![CommandItem::BitOrder(BitOrder::LSB0)]
                    },
                    in_field_list: None,
                    out_field_list: None
                }),
            }
        );

        assert_eq!(
            syn::parse_str::<Command>("command Bar { in { } }").unwrap(),
            Command {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("Bar", Span::call_site()),
                value: Some(CommandValue::Extended {
                    command_item_list: CommandItemList { items: vec![] },
                    in_field_list: Some(FieldList { fields: vec![] }),
                    out_field_list: None
                }),
            }
        );

        assert_eq!(
            syn::parse_str::<Command>("command Bar { in { }, out { }, }").unwrap(),
            Command {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("Bar", Span::call_site()),
                value: Some(CommandValue::Extended {
                    command_item_list: CommandItemList { items: vec![] },
                    in_field_list: Some(FieldList { fields: vec![] }),
                    out_field_list: Some(FieldList { fields: vec![] })
                }),
            }
        );

        assert_eq!(
            syn::parse_str::<Command>("command Bar { out { foo: bool = 0 } }").unwrap(),
            Command {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("Bar", Span::call_site()),
                value: Some(CommandValue::Extended {
                    command_item_list: CommandItemList { items: vec![] },
                    in_field_list: None,
                    out_field_list: Some(FieldList {
                        fields: vec![Field {
                            attribute_list: AttributeList::new(),
                            identifier: Ident::new("foo", Span::call_site()),
                            access: None,
                            base_type: BaseType::Bool,
                            conversion: None,
                            field_address: FieldAddress::Integer(LitInt::new(
                                "0",
                                Span::call_site()
                            ))
                        }]
                    })
                }),
            }
        );

        assert_eq!(
            syn::parse_str::<Command>("command Bar { in { }, out { }, more stuff! }")
                .unwrap_err()
                .to_string(),
            "Did not expect any more tokens"
        );

        assert_eq!(
            syn::parse_str::<Command>("command Bar").unwrap(),
            Command {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("Bar", Span::call_site()),
                value: None,
            }
        );
    }

    #[test]
    fn parse_register_item_list() {
        assert_eq!(
            syn::parse_str::<RegisterItemList>("").unwrap(),
            RegisterItemList {
                register_items: vec![]
            }
        );

        assert_eq!(
            syn::parse_str::<RegisterItemList>("type Access = RW;").unwrap(),
            RegisterItemList {
                register_items: vec![RegisterItem::Access(Access::RW)]
            }
        );

        assert_eq!(
            syn::parse_str::<RegisterItemList>("type Access = RW")
                .unwrap_err()
                .to_string(),
            "expected `;`"
        );

        assert_eq!(
            syn::parse_str::<RegisterItemList>("type ByteOrder = LE;\ntype BitOrder = LSB0;")
                .unwrap(),
            RegisterItemList {
                register_items: vec![
                    RegisterItem::ByteOrder(ByteOrder::LE),
                    RegisterItem::BitOrder(BitOrder::LSB0)
                ]
            }
        );

        assert_eq!(
            syn::parse_str::<RegisterItemList>("const RST_VALUE = 5;")
                .unwrap_err()
                .to_string(),
            "expected one of: `ADDRESS`, `SIZE_BITS`, `RESET_VALUE`, `REPEAT`, `ALLOW_BIT_OVERLAP`, `ALLOW_ADDRESS_OVERLAP`"
        );

        assert_eq!(
            syn::parse_str::<RegisterItemList>("type BT_ORDR = LSB0;")
                .unwrap_err()
                .to_string(),
            "expected one of: `Access`, `ByteOrder`, `BitOrder`"
        );

        assert_eq!(
            syn::parse_str::<RegisterItemList>(
                "const ADDRESS = 0x123;\nconst SIZE_BITS = 16;\nconst RESET_VALUE = 0xFFFF;"
            )
            .unwrap(),
            RegisterItemList {
                register_items: vec![
                    RegisterItem::Address(LitInt::new("0x123", Span::call_site())),
                    RegisterItem::SizeBits(LitInt::new("16", Span::call_site())),
                    RegisterItem::ResetValueInt(LitInt::new("0xFFFF", Span::call_site()))
                ]
            }
        );

        assert_eq!(
            syn::parse_str::<RegisterItemList>("const RESET_VALUE = [0, 1, 2, 0x30];").unwrap(),
            RegisterItemList {
                register_items: vec![RegisterItem::ResetValueArray(vec![0, 1, 2, 0x30])]
            }
        );

        assert_eq!(
            syn::parse_str::<RegisterItemList>("const RESET_VALUE = [0, 1, 2, 0x300];")
                .unwrap_err()
                .to_string(),
            "number too large to fit in target type"
        );

        assert_eq!(
            syn::parse_str::<RegisterItemList>("const REPEAT = { count: 0, stride: 0 };").unwrap(),
            RegisterItemList {
                register_items: vec![RegisterItem::Repeat(Repeat {
                    count: RepeatCount::Value(LitInt::new("0", Span::call_site())),
                    stride: LitInt::new("0", Span::call_site())
                })]
            }
        );

        assert_eq!(
            syn::parse_str::<RegisterItemList>("const RRRRRESET_VALUE = [0, 1, 2, 0x30];")
                .unwrap_err()
                .to_string(),
            "expected one of: `ADDRESS`, `SIZE_BITS`, `RESET_VALUE`, `REPEAT`, `ALLOW_BIT_OVERLAP`, `ALLOW_ADDRESS_OVERLAP`"
        );

        assert_eq!(
            syn::parse_str::<RegisterItemList>("const RESET_VALUE = ;")
                .unwrap_err()
                .to_string(),
            "expected integer literal or square brackets"
        );

        assert_eq!(
            syn::parse_str::<RegisterItemList>("type Access = RW; type Access = RW;")
                .unwrap_err()
                .to_string(),
            "duplicate item found"
        );
    }

    #[test]
    fn parse_attribute_list() {
        assert_eq!(
            syn::parse_str::<AttributeList>("#[custom]")
                .unwrap_err()
                .to_string(),
            "Unsupported attribute 'custom'. Only `doc` and `cfg` attributes are allowed"
        );
        assert_eq!(
            syn::parse_str::<AttributeList>("#[doc(bla)]")
                .unwrap_err()
                .to_string(),
            "expected `=`"
        );
        assert_eq!(
            syn::parse_str::<AttributeList>("#[doc = 1]")
                .unwrap_err()
                .to_string(),
            "Invalid doc attribute format"
        );
    }

    #[test]
    fn parse_ref_object() {
        assert_eq!(
            syn::parse_str::<RefObject>("ref MyRef = command MyOriginal").unwrap(),
            RefObject {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("MyRef", Span::call_site()),
                object: Box::new(Object::Command(Command {
                    attribute_list: AttributeList::new(),
                    identifier: Ident::new("MyOriginal", Span::call_site()),
                    value: None
                }))
            }
        );

        assert_eq!(
            syn::parse_str::<RefObject>("/// Hi!\nref MyRef = command MyOriginal").unwrap(),
            RefObject {
                attribute_list: AttributeList {
                    attributes: vec![Attribute::Doc(" Hi!".into())]
                },
                identifier: Ident::new("MyRef", Span::call_site()),
                object: Box::new(Object::Command(Command {
                    attribute_list: AttributeList::new(),
                    identifier: Ident::new("MyOriginal", Span::call_site()),
                    value: None
                }))
            }
        );
    }

    #[test]
    fn parse_register() {
        assert_eq!(
            syn::parse_str::<Register>("register Foo { }").unwrap(),
            Register {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("Foo", Span::call_site()),
                field_list: FieldList::new(),
                register_item_list: RegisterItemList::new(),
            }
        );

        assert_eq!(
            syn::parse_str::<Register>("register Foo")
                .unwrap_err()
                .to_string(),
            "unexpected end of input, expected curly braces"
        );

        assert_eq!(
            syn::parse_str::<Register>(
                "/// Hello!\nregister Foo { type Access = RW; TestField: ReadWrite int = 0x123, }"
            )
            .unwrap(),
            Register {
                attribute_list: AttributeList {
                    attributes: vec![Attribute::Doc(" Hello!".into())]
                },
                identifier: Ident::new("Foo", Span::call_site()),
                register_item_list: RegisterItemList {
                    register_items: vec![RegisterItem::Access(Access::RW)]
                },
                field_list: FieldList {
                    fields: vec![Field {
                        attribute_list: AttributeList::new(),
                        identifier: Ident::new("TestField", Span::call_site()),
                        access: Some(Access::RW),
                        base_type: BaseType::Int,
                        conversion: None,
                        field_address: FieldAddress::Integer(LitInt::new(
                            "0x123",
                            Span::call_site()
                        ))
                    }]
                },
            }
        );
    }

    #[test]
    fn parse_block_item_list() {
        assert_eq!(
            syn::parse_str::<BlockItemList>("").unwrap(),
            BlockItemList {
                block_items: vec![]
            }
        );

        assert_eq!(
            syn::parse_str::<BlockItemList>("const ADDRESS_OFFSET = 2;").unwrap(),
            BlockItemList {
                block_items: vec![BlockItem::AddressOffset(LitInt::new(
                    "2",
                    Span::call_site()
                ))]
            }
        );

        assert_eq!(
            syn::parse_str::<BlockItemList>(
                "const ADDRESS_OFFSET = 2; const REPEAT = { count: 0, stride: 0 };"
            )
            .unwrap(),
            BlockItemList {
                block_items: vec![
                    BlockItem::AddressOffset(LitInt::new("2", Span::call_site())),
                    BlockItem::Repeat(Repeat {
                        count: RepeatCount::Value(LitInt::new("0", Span::call_site())),
                        stride: LitInt::new("0", Span::call_site())
                    })
                ]
            }
        );

        assert_eq!(
            syn::parse_str::<BlockItemList>("const ADDRESS = 2;")
                .unwrap_err()
                .to_string(),
            "Invalid value. Must be an `ADDRESS_OFFSET` or `REPEAT`"
        );

        assert_eq!(
            syn::parse_str::<BlockItemList>("const ADDRESS_OFFSET = 2; const ADDRESS_OFFSET = 2;")
                .unwrap_err()
                .to_string(),
            "duplicate item found"
        );
    }

    #[test]
    fn parse_block() {
        assert_eq!(
            syn::parse_str::<Block>("block MyBlock {}").unwrap(),
            Block {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("MyBlock", Span::call_site()),
                block_item_list: BlockItemList {
                    block_items: vec![]
                },
                object_list: ObjectList { objects: vec![] },
            }
        );

        assert_eq!(
            syn::parse_str::<Block>("/// Hi there\nblock MyBlock { const ADDRESS_OFFSET = 5; command A = 5, buffer B = 6 }").unwrap(),
            Block {
                attribute_list: AttributeList { attributes: vec![Attribute::Doc(" Hi there".into())] },
                identifier: Ident::new("MyBlock", Span::call_site()),
                block_item_list: BlockItemList {
                    block_items: vec![BlockItem::AddressOffset(LitInt::new("5", Span::call_site()))]
                },
                object_list: ObjectList {
                    objects: vec![
                        Object::Command(Command {
                            attribute_list: AttributeList::new(),
                            identifier: Ident::new("A", Span::call_site()),
                            value: Some(CommandValue::Basic(LitInt::new("5", Span::call_site())))
                        }),
                        Object::Buffer(Buffer {
                            attribute_list: AttributeList::new(),
                            identifier: Ident::new("B", Span::call_site()),
                            access: None,
                            address: Some(LitInt::new("6", Span::call_site()))
                        })
                    ]
                }
            }
        );
    }

    #[test]
    fn parse_global_config_list() {
        assert_eq!(
            syn::parse_str::<GlobalConfigList>("").unwrap(),
            GlobalConfigList { configs: vec![] }
        );

        assert_eq!(
            syn::parse_str::<GlobalConfigList>("config { }").unwrap(),
            GlobalConfigList { configs: vec![] }
        );

        assert_eq!(
            syn::parse_str::<GlobalConfigList>("config { type DefaultRegisterAccess = RW }")
                .unwrap_err()
                .to_string(),
            "expected `;`"
        );

        assert_eq!(
            syn::parse_str::<GlobalConfigList>("config { type DefaultRegisterAccess = RW; }")
                .unwrap(),
            GlobalConfigList {
                configs: vec![GlobalConfig::DefaultRegisterAccess(Access::RW)]
            }
        );

        assert_eq!(
            syn::parse_str::<GlobalConfigList>(
                "config { type DefaultBufferAccess = RO; type DefaultFieldAccess = RW; }"
            )
            .unwrap(),
            GlobalConfigList {
                configs: vec![
                    GlobalConfig::DefaultBufferAccess(Access::RO),
                    GlobalConfig::DefaultFieldAccess(Access::RW)
                ]
            }
        );

        assert_eq!(
            syn::parse_str::<GlobalConfigList>(
                "config { type DefaultByteOrder = LE; type DefaultBitOrder = LSB0; type NameWordBoundaries = \"aA:1B\"; }"
            )
            .unwrap(),
            GlobalConfigList {
                configs: vec![
                    GlobalConfig::DefaultByteOrder(ByteOrder::LE),
                    GlobalConfig::DefaultBitOrder(BitOrder::LSB0),
                    GlobalConfig::NameWordBoundaries(vec![Boundary::LowerUpper, Boundary::DigitUpper])
                ]
            }
        );

        assert_eq!(
            syn::parse_str::<GlobalConfigList>(
                "config { type NameWordBoundaries = [DigitLower, Hyphen]; }"
            )
            .unwrap(),
            GlobalConfigList {
                configs: vec![GlobalConfig::NameWordBoundaries(vec![
                    Boundary::DigitLower,
                    Boundary::Hyphen
                ])]
            }
        );

        assert_eq!(
            syn::parse_str::<GlobalConfigList>("config { type NameWordBoundaries = 5; }")
                .unwrap_err()
                .to_string(),
            "Expected an array of boundaries or a string"
        );

        assert_eq!(
            syn::parse_str::<GlobalConfigList>("config { type NameWordBoundaries = [lol]; }")
                .unwrap_err()
                .to_string(),
            "`lol` is not a valid boundary name. One of the following was expected: [Hyphen, Underscore, Space, LowerUpper, UpperLower, DigitUpper, UpperDigit, DigitLower, LowerDigit, Acronym]"
        );

        assert_eq!(
            syn::parse_str::<GlobalConfigList>(
                "config { type RegisterAddressType = u8; type CommandAddressType = u16; type BufferAddressType = u32; }"
            )
            .unwrap(),
            GlobalConfigList {
                configs: vec![
                    GlobalConfig::RegisterAddressType(Ident::new("u8", Span::call_site())),
                    GlobalConfig::CommandAddressType(Ident::new("u16", Span::call_site())),
                    GlobalConfig::BufferAddressType(Ident::new("u32", Span::call_site()))
                ]
            }
        );

        assert_eq!(
            syn::parse_str::<GlobalConfigList>("config { type DefaultRegisterAccesssss = RW; }")
                .unwrap_err()
                .to_string(),
            "expected one of: `DefaultRegisterAccess`, `DefaultFieldAccess`, `DefaultBufferAccess`, `DefaultByteOrder`, `DefaultBitOrder`, `RegisterAddressType`, `CommandAddressType`, `BufferAddressType`, `NameWordBoundaries`, `DefmtFeature`"
        );
    }

    #[test]
    fn parse_object() {
        assert_eq!(
            syn::parse_str::<Object>("config { }")
                .unwrap_err()
                .to_string(),
            "expected one of: `block`, `register`, `command`, `buffer`, `ref`"
        );

        assert_eq!(
            syn::parse_str::<Object>("block Foo {}").unwrap(),
            Object::Block(Block {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("Foo", Span::call_site()),
                block_item_list: BlockItemList {
                    block_items: vec![]
                },
                object_list: ObjectList { objects: vec![] }
            }),
        );

        assert_eq!(
            syn::parse_str::<Object>("register Foo {}").unwrap(),
            Object::Register(Register {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("Foo", Span::call_site()),
                register_item_list: RegisterItemList {
                    register_items: vec![]
                },
                field_list: FieldList { fields: vec![] }
            }),
        );

        assert_eq!(
            syn::parse_str::<Object>("command Foo").unwrap(),
            Object::Command(Command {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("Foo", Span::call_site()),
                value: None,
            }),
        );

        assert_eq!(
            syn::parse_str::<Object>("buffer Foo").unwrap(),
            Object::Buffer(Buffer {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("Foo", Span::call_site()),
                access: None,
                address: None,
            }),
        );

        assert_eq!(
            syn::parse_str::<Object>("ref Foo2 = buffer Foo").unwrap(),
            Object::Ref(RefObject {
                attribute_list: AttributeList::new(),
                identifier: Ident::new("Foo2", Span::call_site()),
                object: Box::new(Object::Buffer(Buffer {
                    attribute_list: AttributeList::new(),
                    identifier: Ident::new("Foo", Span::call_site()),
                    access: None,
                    address: None,
                }))
            }),
        );

        assert_eq!(
            syn::parse_str::<Object>("/// Comment!\nbuffer Foo").unwrap(),
            Object::Buffer(Buffer {
                attribute_list: AttributeList {
                    attributes: vec![Attribute::Doc(" Comment!".into())]
                },
                identifier: Ident::new("Foo", Span::call_site()),
                access: None,
                address: None,
            }),
        );
    }

    #[test]
    fn parse_device() {
        assert_eq!(
            syn::parse_str::<Device>("").unwrap(),
            Device {
                global_config_list: GlobalConfigList { configs: vec![] },
                object_list: ObjectList { objects: vec![] }
            }
        );

        assert_eq!(
            syn::parse_str::<Device>("config { type DefaultRegisterAccess = RW; }").unwrap(),
            Device {
                global_config_list: GlobalConfigList {
                    configs: vec![GlobalConfig::DefaultRegisterAccess(Access::RW)]
                },
                object_list: ObjectList { objects: vec![] }
            }
        );

        assert_eq!(
            syn::parse_str::<Device>("buffer Foo").unwrap(),
            Device {
                global_config_list: GlobalConfigList { configs: vec![] },
                object_list: ObjectList {
                    objects: vec![Object::Buffer(Buffer {
                        attribute_list: AttributeList::new(),
                        identifier: Ident::new("Foo", Span::call_site()),
                        access: None,
                        address: None,
                    })]
                }
            }
        );

        assert_eq!(
            syn::parse_str::<Device>("config { type DefaultRegisterAccess = RW; }\nbuffer Foo")
                .unwrap(),
            Device {
                global_config_list: GlobalConfigList {
                    configs: vec![GlobalConfig::DefaultRegisterAccess(Access::RW)]
                },
                object_list: ObjectList {
                    objects: vec![Object::Buffer(Buffer {
                        attribute_list: AttributeList::new(),
                        identifier: Ident::new("Foo", Span::call_site()),
                        access: None,
                        address: None,
                    })]
                }
            }
        );
    }

    #[test]
    fn attribute_eq() {
        // Test for equality on Doc variant
        let doc1 = Attribute::Doc(String::from("some doc"));
        let doc2 = Attribute::Doc(String::from("some doc"));
        assert_eq!(doc1, doc2);

        // Test for inequality on Doc variant
        let doc3 = Attribute::Doc(String::from("different doc"));
        assert_ne!(doc1, doc3);

        // Test for equality on Cfg variant
        let cfg1 = Attribute::Cfg(String::from("some cfg"), Span::call_site());
        let cfg2 = Attribute::Cfg(String::from("some cfg"), Span::call_site());
        assert_eq!(cfg1, cfg2);

        // Test for inequality on Cfg variant
        let cfg3 = Attribute::Cfg(String::from("different cfg"), Span::call_site());
        assert_ne!(cfg1, cfg3);

        // Test for inequality between Doc and Cfg variants
        assert_ne!(doc1, cfg1);
    }
}
