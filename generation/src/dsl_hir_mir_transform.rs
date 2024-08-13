use proc_macro2::Span;
use quote::ToTokens;

use crate::{dsl_hir, mir};

pub fn transform(device: dsl_hir::Device) -> Result<mir::Device, syn::Error> {
    let global_config = device.global_config_list.try_into()?;
    let objects = transform_object_list(device.object_list, &global_config)?;

    Ok(mir::Device {
        global_config,
        objects,
    })
}

impl From<dsl_hir::Access> for mir::Access {
    fn from(value: dsl_hir::Access) -> Self {
        match value {
            dsl_hir::Access::RW => mir::Access::RW,
            dsl_hir::Access::RC => mir::Access::RC,
            dsl_hir::Access::RO => mir::Access::RO,
            dsl_hir::Access::WO => mir::Access::WO,
            dsl_hir::Access::CO => mir::Access::CO,
        }
    }
}

impl From<dsl_hir::ByteOrder> for mir::ByteOrder {
    fn from(value: dsl_hir::ByteOrder) -> Self {
        match value {
            dsl_hir::ByteOrder::LE => mir::ByteOrder::LE,
            dsl_hir::ByteOrder::BE => mir::ByteOrder::BE,
        }
    }
}

impl From<dsl_hir::BitOrder> for mir::BitOrder {
    fn from(value: dsl_hir::BitOrder) -> Self {
        match value {
            dsl_hir::BitOrder::LSB0 => mir::BitOrder::LSB0,
            dsl_hir::BitOrder::MSB0 => mir::BitOrder::MSB0,
        }
    }
}

impl TryFrom<syn::Ident> for mir::Integer {
    type Error = syn::Error;

    fn try_from(value: syn::Ident) -> Result<Self, Self::Error> {
        match value.to_string().as_str() {
            "u8" => Ok(mir::Integer::U8),
            "u16" => Ok(mir::Integer::U16),
            "u32" => Ok(mir::Integer::U32),
            "u64" => Ok(mir::Integer::U64),
            "u128" => Ok(mir::Integer::U128),
            "i8" => Ok(mir::Integer::I8),
            "i16" => Ok(mir::Integer::I16),
            "i32" => Ok(mir::Integer::I32),
            "i64" => Ok(mir::Integer::I64),
            "i128" => Ok(mir::Integer::I128),
            _ => Err(syn::Error::new(value.span(), "Must be an integer type")),
        }
    }
}

impl From<dsl_hir::NameCase> for mir::NameCase {
    fn from(value: dsl_hir::NameCase) -> Self {
        match value {
            dsl_hir::NameCase::Varying => mir::NameCase::Varying,
            dsl_hir::NameCase::Pascal => mir::NameCase::Pascal,
            dsl_hir::NameCase::Snake => mir::NameCase::Snake,
            dsl_hir::NameCase::ScreamingSnake => mir::NameCase::ScreamingSnake,
            dsl_hir::NameCase::Camel => mir::NameCase::Camel,
            dsl_hir::NameCase::Kebab => mir::NameCase::Kebab,
            dsl_hir::NameCase::Cobol => mir::NameCase::Cobol,
        }
    }
}

impl TryFrom<dsl_hir::Repeat> for mir::Repeat {
    type Error = syn::Error;

    fn try_from(value: dsl_hir::Repeat) -> Result<Self, Self::Error> {
        Ok(Self {
            count: value.count.base10_parse()?,
            stride: value.stride.base10_parse()?,
        })
    }
}

impl From<dsl_hir::BaseType> for mir::BaseType {
    fn from(value: dsl_hir::BaseType) -> Self {
        match value {
            dsl_hir::BaseType::Bool => mir::BaseType::Bool,
            dsl_hir::BaseType::Uint => mir::BaseType::Uint,
            dsl_hir::BaseType::Int => mir::BaseType::Int,
        }
    }
}

impl TryFrom<dsl_hir::GlobalConfigList> for mir::GlobalConfig {
    type Error = syn::Error;

    fn try_from(value: dsl_hir::GlobalConfigList) -> Result<Self, Self::Error> {
        let mut global_config = mir::GlobalConfig::default();

        for config in value.configs.iter() {
            let same_config_count = value
                .configs
                .iter()
                .filter(|check_config| {
                    std::mem::discriminant(*check_config) == std::mem::discriminant(config)
                })
                .count();

            if same_config_count > 1 {
                return Err(syn::Error::new(
                    Span::call_site(),
                    format!("Duplicate global config found: `{config:?}`"),
                ));
            }

            match config.clone() {
                dsl_hir::GlobalConfig::DefaultRegisterAccess(value) => {
                    global_config.default_register_access = value.into()
                }
                dsl_hir::GlobalConfig::DefaultFieldAccess(value) => {
                    global_config.default_field_access = value.into()
                }
                dsl_hir::GlobalConfig::DefaultBufferAccess(value) => {
                    global_config.default_buffer_access = value.into()
                }
                dsl_hir::GlobalConfig::DefaultByteOrder(value) => {
                    global_config.default_byte_order = value.into()
                }
                dsl_hir::GlobalConfig::DefaultBitOrder(value) => {
                    global_config.default_bit_order = value.into()
                }
                dsl_hir::GlobalConfig::RegisterAddressType(value) => {
                    global_config.register_address_type = Some(value.try_into()?)
                }
                dsl_hir::GlobalConfig::CommandAddressType(value) => {
                    global_config.command_address_type = Some(value.try_into()?)
                }
                dsl_hir::GlobalConfig::BufferAddressType(value) => {
                    global_config.buffer_address_type = Some(value.try_into()?)
                }
                dsl_hir::GlobalConfig::NameCase(value) => global_config.name_case = value.into(),
            }
        }

        Ok(global_config)
    }
}

fn get_description(attrs: &dsl_hir::AttributeList) -> Option<String> {
    let str = attrs
        .attributes
        .iter()
        .filter_map(|attr| match attr {
            dsl_hir::Attribute::Doc(val, _) => Some(val.as_str()),
            dsl_hir::Attribute::Cfg(_, _) => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    if str.is_empty() {
        None
    } else {
        Some(str)
    }
}

fn get_cfg_attr(attrs: &dsl_hir::AttributeList) -> Result<Option<String>, syn::Error> {
    let mut cfg_attrs = attrs
        .attributes
        .iter()
        .filter_map(|attr| match attr {
            dsl_hir::Attribute::Cfg(val, span) => Some((val, span)),
            dsl_hir::Attribute::Doc(_, _) => None,
        })
        .collect::<Vec<_>>();

    match cfg_attrs.len() {
        0 => Ok(None),
        1 => Ok(Some(cfg_attrs.remove(0).0.clone())),
        n => Err(syn::Error::new(
            cfg_attrs.remove(1).1.clone(),
            &format!("Only one cfg attribute is allowed, but {n} are found"),
        )),
    }
}

fn transform_object_list(
    list: dsl_hir::ObjectList,
    global_config: &mir::GlobalConfig,
) -> Result<Vec<mir::Object>, syn::Error> {
    let mut objects = Vec::new();

    for object in list.objects.into_iter() {
        let object = match object {
            dsl_hir::Object::Block(_) => todo!(),
            dsl_hir::Object::Register(_) => todo!(),
            dsl_hir::Object::Command(command) => {
                mir::Object::Command(transform_command(command, global_config)?)
            }
            dsl_hir::Object::Buffer(buffer) => {
                mir::Object::Buffer(transform_buffer(buffer, global_config)?)
            }
            dsl_hir::Object::Ref(ref_object) => {
                mir::Object::Ref(transform_ref(ref_object, global_config)?)
            }
        };

        objects.push(object);
    }

    Ok(objects)
}

fn transform_command(
    command: dsl_hir::Command,
    global_config: &mir::GlobalConfig,
) -> Result<mir::Command, syn::Error> {
    let command_value = command.value.ok_or_else(|| {
        syn::Error::new(
            command.identifier.span(),
            &format!(
                "Command `{}` must have a value",
                command.identifier.to_string()
            ),
        )
    })?;
    Ok(mir::Command {
        cfg_attr: get_cfg_attr(&command.attribute_list)?,
        description: get_description(&command.attribute_list).unwrap_or_default(),
        name: command.identifier.to_string(),
        address: match &command_value {
            dsl_hir::CommandValue::Basic(lit) => lit,
            dsl_hir::CommandValue::Extended {
                command_item_list, ..
            } => command_item_list
                .items
                .iter()
                .find_map(|item| match item {
                    dsl_hir::CommandItem::Address(lit) => Some(lit),
                    _ => None,
                })
                .ok_or_else(|| {
                    syn::Error::new(
                        command.identifier.span(),
                        &format!(
                            "Command `{}` must have an address",
                            command.identifier.to_string()
                        ),
                    )
                })?,
        }
        .base10_parse()?,
        byte_order: match &command_value {
            dsl_hir::CommandValue::Basic(_) => None,
            dsl_hir::CommandValue::Extended {
                command_item_list, ..
            } => command_item_list.items.iter().find_map(|item| match item {
                dsl_hir::CommandItem::ByteOrder(order) => Some(order.clone().into()),
                _ => None,
            }),
        }
        .unwrap_or(global_config.default_byte_order),
        bit_order: match &command_value {
            dsl_hir::CommandValue::Basic(_) => None,
            dsl_hir::CommandValue::Extended {
                command_item_list, ..
            } => command_item_list.items.iter().find_map(|item| match item {
                dsl_hir::CommandItem::BitOrder(order) => Some(order.clone().into()),
                _ => None,
            }),
        }
        .unwrap_or(global_config.default_bit_order),
        size_bits_in: match &command_value {
            dsl_hir::CommandValue::Basic(_) => None,
            dsl_hir::CommandValue::Extended {
                command_item_list, ..
            } => command_item_list.items.iter().find_map(|item| match item {
                dsl_hir::CommandItem::SizeBitsIn(size) => Some(size.base10_parse()),
                _ => None,
            }),
        }
        .unwrap_or(Ok(0))?,
        size_bits_out: match &command_value {
            dsl_hir::CommandValue::Basic(_) => None,
            dsl_hir::CommandValue::Extended {
                command_item_list, ..
            } => command_item_list.items.iter().find_map(|item| match item {
                dsl_hir::CommandItem::SizeBitsOut(size) => Some(size.base10_parse()),
                _ => None,
            }),
        }
        .unwrap_or(Ok(0))?,
        repeat: match &command_value {
            dsl_hir::CommandValue::Basic(_) => None,
            dsl_hir::CommandValue::Extended {
                command_item_list, ..
            } => command_item_list.items.iter().find_map(|item| match item {
                dsl_hir::CommandItem::Repeat(repeat) => Some(repeat.clone().try_into()),
                _ => None,
            }),
        }
        .transpose()?,
        in_fields: match &command_value {
            dsl_hir::CommandValue::Basic(_)
            | dsl_hir::CommandValue::Extended {
                in_field_list: None,
                ..
            } => Vec::new(),
            dsl_hir::CommandValue::Extended {
                in_field_list: Some(in_field_list),
                ..
            } => in_field_list
                .fields
                .iter()
                .map(|field| transform_field(field, global_config))
                .collect::<Result<_, _>>()?,
        },
        out_fields: match &command_value {
            dsl_hir::CommandValue::Basic(_)
            | dsl_hir::CommandValue::Extended {
                out_field_list: None,
                ..
            } => Vec::new(),
            dsl_hir::CommandValue::Extended {
                out_field_list: Some(out_field_list),
                ..
            } => out_field_list
                .fields
                .iter()
                .map(|field| transform_field(field, global_config))
                .collect::<Result<_, _>>()?,
        },
    })
}

fn transform_field(
    field: &dsl_hir::Field,
    global_config: &mir::GlobalConfig,
) -> Result<mir::Field, syn::Error> {
    Ok(mir::Field {
        cfg_attr: get_cfg_attr(&field.attribute_list)?,
        description: get_description(&field.attribute_list).unwrap_or_default(),
        name: field.identifier.to_string(),
        access: field
            .access
            .map(Into::into)
            .unwrap_or(global_config.default_field_access),
        base_type: field.base_type.into(),
        field_conversion: field.field_conversion.as_ref().map(|fc| transform_field_conversion(fc)).transpose()?,
        field_address: match &field.field_address {
            dsl_hir::FieldAddress::Integer(start) if field.base_type.is_bool() =>
                start.base10_parse()?..start.base10_parse()?,
            dsl_hir::FieldAddress::Integer(_) =>
                return Err(syn::Error::new(
                    field.identifier.span(),
                    &format!(
                        "Field `{}` has a non-bool base type and must specify the start and the end address",
                        field.identifier.to_string()
                    )
                )),
            dsl_hir::FieldAddress::Range { start, end } => {
                start.base10_parse()?..end.base10_parse()?
            }
            dsl_hir::FieldAddress::RangeInclusive { start, end } => {
                start.base10_parse()?..(end.base10_parse::<u64>()? + 1)
            }
        },
    })
}

fn transform_field_conversion(
    field_conversion: &dsl_hir::FieldConversion,
) -> Result<mir::FieldConversion, syn::Error> {
    match field_conversion {
        dsl_hir::FieldConversion::Direct(path) => Ok(mir::FieldConversion::Direct(
            path.to_token_stream()
                .to_string()
                .replace(char::is_whitespace, ""),
        )),
        dsl_hir::FieldConversion::Enum {
            identifier,
            enum_variant_list,
        } => Ok(mir::FieldConversion::Enum {
            name: identifier.to_string(),
            variants: enum_variant_list
                .variants
                .iter()
                .map(|v| {
                    Ok(mir::EnumVariant {
                        cfg_attr: get_cfg_attr(&v.attribute_list)?,
                        description: get_description(&v.attribute_list).unwrap_or_default(),
                        name: v.identifier.to_string(),
                        value: match &v.enum_value {
                            None => mir::EnumValue::Unspecified,
                            Some(dsl_hir::EnumValue::Specified(val)) => {
                                mir::EnumValue::Specified(val.base10_parse()?)
                            }
                            Some(dsl_hir::EnumValue::Default) => mir::EnumValue::Default,
                            Some(dsl_hir::EnumValue::CatchAll) => mir::EnumValue::CatchAll,
                        },
                    })
                })
                .collect::<Result<_, syn::Error>>()?,
        }),
    }
}

fn transform_buffer(
    buffer: dsl_hir::Buffer,
    global_config: &mir::GlobalConfig,
) -> Result<mir::Buffer, syn::Error> {
    Ok(mir::Buffer {
        cfg_attr: get_cfg_attr(&buffer.attribute_list)?,
        description: get_description(&buffer.attribute_list).unwrap_or_default(),
        name: buffer.identifier.to_string(),
        access: buffer
            .access
            .map(Into::into)
            .unwrap_or(global_config.default_buffer_access),
        address: buffer
            .address
            .ok_or_else(|| {
                syn::Error::new(
                    buffer.identifier.span(),
                    &format!(
                        "Buffer `{}` must have an address",
                        buffer.identifier.to_string()
                    ),
                )
            })?
            .base10_parse()?,
    })
}

fn transform_ref(
    ref_object: dsl_hir::RefObject,
    global_config: &mir::GlobalConfig,
) -> Result<mir::RefObject, syn::Error> {
    Ok(mir::RefObject {
        cfg_attr: get_cfg_attr(&ref_object.attribute_list)?,
        description: get_description(&ref_object.attribute_list).unwrap_or_default(),
        name: ref_object.identifier.to_string(),
        object: match *ref_object.object {
            dsl_hir::Object::Block(block_override) => Box::new(mir::ObjectOverride::Block(
                transform_block_override(block_override)?,
            )),
            dsl_hir::Object::Register(register_override) => Box::new(
                mir::ObjectOverride::Register(transform_register_override(register_override)?),
            ),
            dsl_hir::Object::Command(command_override) => Box::new(mir::ObjectOverride::Command(
                transform_command_override(command_override, global_config)?,
            )),
            dsl_hir::Object::Buffer(buffer_override) => Box::new(mir::ObjectOverride::Buffer(
                transform_buffer_override(buffer_override)?,
            )),
            dsl_hir::Object::Ref(_) => {
                return Err(syn::Error::new(
                    ref_object.identifier.span(),
                    &format!(
                        "Ref `{}` cannot ref another ref object",
                        ref_object.identifier.to_string()
                    ),
                ))
            }
        },
    })
}

fn transform_block_override(
    block_override: dsl_hir::Block,
) -> Result<mir::BlockOverride, syn::Error> {
    todo!()
}

fn transform_register_override(
    register_override: dsl_hir::Register,
) -> Result<mir::RegisterOverride, syn::Error> {
    todo!()
}

fn transform_command_override(
    command_override: dsl_hir::Command,
    global_config: &mir::GlobalConfig,
) -> Result<mir::CommandOverride, syn::Error> {
    Ok(mir::CommandOverride {
        name: command_override.identifier.to_string(),
        address: match &command_override.value {
            None => None,
            Some(dsl_hir::CommandValue::Basic(lit)) => Some(lit),
            Some(dsl_hir::CommandValue::Extended {
                command_item_list, ..
            }) => command_item_list.items.iter().find_map(|item| match item {
                dsl_hir::CommandItem::Address(lit) => Some(lit),
                _ => None,
            }),
        }
        .map(|lit| lit.base10_parse())
        .transpose()?,
        byte_order: match &command_override.value {
            None | Some(dsl_hir::CommandValue::Basic(_)) => None,
            Some(dsl_hir::CommandValue::Extended {
                command_item_list, ..
            }) => command_item_list.items.iter().find_map(|item| match item {
                dsl_hir::CommandItem::ByteOrder(order) => Some(order.clone().into()),
                _ => None,
            }),
        },
        bit_order: match &command_override.value {
            None | Some(dsl_hir::CommandValue::Basic(_)) => None,
            Some(dsl_hir::CommandValue::Extended {
                command_item_list, ..
            }) => command_item_list.items.iter().find_map(|item| match item {
                dsl_hir::CommandItem::BitOrder(order) => Some(order.clone().into()),
                _ => None,
            }),
        },
        size_bits_in: match &command_override.value {
            None | Some(dsl_hir::CommandValue::Basic(_)) => None,
            Some(dsl_hir::CommandValue::Extended {
                command_item_list, ..
            }) => command_item_list.items.iter().find_map(|item| match item {
                dsl_hir::CommandItem::SizeBitsIn(size) => Some(size.base10_parse()),
                _ => None,
            }),
        }
        .transpose()?,
        size_bits_out: match &command_override.value {
            None | Some(dsl_hir::CommandValue::Basic(_)) => None,
            Some(dsl_hir::CommandValue::Extended {
                command_item_list, ..
            }) => command_item_list.items.iter().find_map(|item| match item {
                dsl_hir::CommandItem::SizeBitsOut(size) => Some(size.base10_parse()),
                _ => None,
            }),
        }
        .transpose()?,
        repeat: match &command_override.value {
            None | Some(dsl_hir::CommandValue::Basic(_)) => None,
            Some(dsl_hir::CommandValue::Extended {
                command_item_list, ..
            }) => command_item_list
                .items
                .iter()
                .find_map(|item| match item {
                    dsl_hir::CommandItem::Repeat(repeat) => {
                        Some(mir::Repeat::try_from(repeat.clone()))
                    }
                    _ => None,
                })
                .transpose()?,
        },
        in_fields: match &command_override.value {
            None => None,
            Some(dsl_hir::CommandValue::Basic(_)) => None,
            Some(dsl_hir::CommandValue::Extended { in_field_list, .. }) => in_field_list
                .as_ref()
                .map(|in_field_list| {
                    in_field_list
                        .fields
                        .iter()
                        .map(|field| transform_field(field, global_config))
                        .collect::<Result<_, _>>()
                })
                .transpose()?,
        },
        out_fields: match &command_override.value {
            None => None,
            Some(dsl_hir::CommandValue::Basic(_)) => None,
            Some(dsl_hir::CommandValue::Extended { out_field_list, .. }) => out_field_list
                .as_ref()
                .map(|out_field_list| {
                    out_field_list
                        .fields
                        .iter()
                        .map(|field| transform_field(field, global_config))
                        .collect::<Result<_, _>>()
                })
                .transpose()?,
        },
    })
}

fn transform_buffer_override(
    buffer_override: dsl_hir::Buffer,
) -> Result<mir::BufferOverride, syn::Error> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_double_global_settings() {
        let device = syn::parse_str::<dsl_hir::Device>(
            "config { type DefaultRegisterAccess = RW; type DefaultRegisterAccess = RW; }",
        )
        .unwrap();

        assert_eq!(
            transform(device).unwrap_err().to_string(),
            "Duplicate global config found: `DefaultRegisterAccess(RW)`"
        );
    }

    #[test]
    fn global_settings_correct() {
        let device = syn::parse_str::<dsl_hir::Device>(
            "config {
                type DefaultRegisterAccess = RO;
                type DefaultFieldAccess = RC;
                type DefaultBufferAccess = WO;
                type DefaultByteOrder = LE;
                type DefaultBitOrder = MSB0;
                type RegisterAddressType = i8;
                type CommandAddressType = u128;
                type BufferAddressType = u32;
                type NameCase = Pascal;
            }",
        )
        .unwrap();

        let device = transform(device).unwrap();

        assert_eq!(
            device.global_config,
            mir::GlobalConfig {
                default_register_access: mir::Access::RO,
                default_field_access: mir::Access::RC,
                default_buffer_access: mir::Access::WO,
                default_byte_order: mir::ByteOrder::LE,
                default_bit_order: mir::BitOrder::MSB0,
                register_address_type: Some(mir::Integer::I8),
                command_address_type: Some(mir::Integer::U128),
                buffer_address_type: Some(mir::Integer::U32),
                name_case: mir::NameCase::Pascal,
            }
        );
    }

    #[test]
    fn buffer() {
        assert_eq!(
            transform(
                syn::parse_str::<dsl_hir::Device>(
                    "
                    /// Hello world!
                    #[cfg(feature = \"foo\")]
                    /// This should be in order!
                    buffer Foo: RO = 5
                    ",
                )
                .unwrap()
            )
            .unwrap()
            .objects,
            &[mir::Object::Buffer(mir::Buffer {
                cfg_attr: Some("feature = \"foo\"".into()),
                description: " Hello world!\n This should be in order!".into(),
                name: "Foo".into(),
                access: mir::Access::RO,
                address: 5,
            })]
        );

        assert_eq!(
            transform(
                syn::parse_str::<dsl_hir::Device>(
                    "
                    buffer Foo
                    ",
                )
                .unwrap()
            )
            .unwrap_err()
            .to_string(),
            "Buffer `Foo` must have an address"
        );
    }

    #[test]
    fn command() {
        assert_eq!(
            transform(
                syn::parse_str::<dsl_hir::Device>(
                    "
                    command Foo
                    ",
                )
                .unwrap()
            )
            .unwrap_err()
            .to_string(),
            "Command `Foo` must have a value"
        );

        assert_eq!(
            transform(
                syn::parse_str::<dsl_hir::Device>(
                    "
                    command Foo {}
                    ",
                )
                .unwrap()
            )
            .unwrap_err()
            .to_string(),
            "Command `Foo` must have an address"
        );

        assert_eq!(
            transform(
                syn::parse_str::<dsl_hir::Device>(
                    "
                    /// Hello world!
                    #[cfg(feature = \"foo\")]
                    /// This should be in order!
                    command Foo = 5
                    ",
                )
                .unwrap()
            )
            .unwrap()
            .objects,
            &[mir::Object::Command(mir::Command {
                cfg_attr: Some("feature = \"foo\"".into()),
                description: " Hello world!\n This should be in order!".into(),
                name: "Foo".into(),
                address: 5,
                byte_order: Default::default(),
                bit_order: Default::default(),
                size_bits_in: 0,
                size_bits_out: 0,
                repeat: Default::default(),
                in_fields: Default::default(),
                out_fields: Default::default()
            })]
        );

        assert_eq!(
            transform(
                syn::parse_str::<dsl_hir::Device>(
                    "
                    config {
                        type DefaultByteOrder = LE;
                        type DefaultFieldAccess = RO;
                    }
                    command Bar {
                        const SIZE_BITS_IN = 32;
                        const SIZE_BITS_OUT = 16;
                        const REPEAT = {
                            count: 4,
                            stride: 0x10,
                        };
                        const ADDRESS = 10;

                        in {
                            /// Hello!
                            #[cfg(bla)]
                            val: WO bool = 0,
                            foo: uint as crate::my_mod::MyStruct = 1..=5,
                        }
                        out {
                            val: int as enum Val {
                                One,
                                /// Two!
                                Two = 2,
                                Three = default,
                                #[cfg(yes)]
                                Four = catch_all,
                            } = 0..16,
                        }
                    }
                    ",
                )
                .unwrap()
            )
            .unwrap()
            .objects,
            &[mir::Object::Command(mir::Command {
                cfg_attr: None,
                description: Default::default(),
                name: "Bar".into(),
                address: 10,
                byte_order: mir::ByteOrder::LE,
                bit_order: Default::default(),
                size_bits_in: 32,
                size_bits_out: 16,
                repeat: Some(mir::Repeat {
                    count: 4,
                    stride: 16
                }),
                in_fields: vec![
                    mir::Field {
                        cfg_attr: Some("bla".into()),
                        description: " Hello!".into(),
                        name: "val".into(),
                        access: mir::Access::WO,
                        base_type: mir::BaseType::Bool,
                        field_conversion: None,
                        field_address: 0..0,
                    },
                    mir::Field {
                        cfg_attr: None,
                        description: Default::default(),
                        name: "foo".into(),
                        access: mir::Access::RO,
                        base_type: mir::BaseType::Uint,
                        field_conversion: Some(mir::FieldConversion::Direct(
                            "crate::my_mod::MyStruct".into()
                        )),
                        field_address: 1..6,
                    }
                ],
                out_fields: vec![mir::Field {
                    cfg_attr: None,
                    description: Default::default(),
                    name: "val".into(),
                    access: mir::Access::RO,
                    base_type: mir::BaseType::Int,
                    field_conversion: Some(mir::FieldConversion::Enum {
                        name: "Val".into(),
                        variants: vec![
                            mir::EnumVariant {
                                cfg_attr: None,
                                description: Default::default(),
                                name: "One".into(),
                                value: mir::EnumValue::Unspecified,
                            },
                            mir::EnumVariant {
                                cfg_attr: None,
                                description: " Two!".into(),
                                name: "Two".into(),
                                value: mir::EnumValue::Specified(2),
                            },
                            mir::EnumVariant {
                                cfg_attr: None,
                                description: Default::default(),
                                name: "Three".into(),
                                value: mir::EnumValue::Default,
                            },
                            mir::EnumVariant {
                                cfg_attr: Some("yes".into()),
                                description: Default::default(),
                                name: "Four".into(),
                                value: mir::EnumValue::CatchAll,
                            }
                        ]
                    }),
                    field_address: 0..16,
                }]
            })]
        );

        assert_eq!(
            transform(
                syn::parse_str::<dsl_hir::Device>(
                    "
                    command Foo {
                        const ADDRESS = 0;

                        in {
                            val: int = 0,
                        }
                    }
                    ",
                )
                .unwrap()
            )
            .unwrap_err()
            .to_string(),
            "Field `val` has a non-bool base type and must specify the start and the end address"
        );

        assert_eq!(
            transform(
                syn::parse_str::<dsl_hir::Device>(
                    "
                    config {
                        type DefaultByteOrder = LE;
                        type DefaultBitOrder = MSB0;
                    }
                    command Bar {
                        type ByteOrder = BE;
                        type BitOrder = LSB0;
                        const ADDRESS = 10;

                        in {
                            val: bool = 0,
                        }
                    }
                    ",
                )
                .unwrap()
            )
            .unwrap()
            .objects,
            &[mir::Object::Command(mir::Command {
                cfg_attr: None,
                description: Default::default(),
                name: "Bar".into(),
                address: 10,
                byte_order: mir::ByteOrder::BE,
                bit_order: mir::BitOrder::LSB0,
                size_bits_in: 0,
                size_bits_out: 0,
                repeat: None,
                in_fields: vec![mir::Field {
                    cfg_attr: None,
                    description: Default::default(),
                    name: "val".into(),
                    access: mir::Access::default(),
                    base_type: mir::BaseType::Bool,
                    field_conversion: None,
                    field_address: 0..0,
                },],
                out_fields: vec![]
            })]
        );
    }

    #[test]
    fn max_one_cfg_attr() {
        assert_eq!(
            transform(
                syn::parse_str::<dsl_hir::Device>(
                    "
                    buffer Foo = 5
                    "
                )
                .unwrap()
            )
            .unwrap()
            .objects,
            &[mir::Object::Buffer(mir::Buffer {
                cfg_attr: None,
                description: "".into(),
                name: "Foo".into(),
                access: mir::Access::default(),
                address: 5,
            })]
        );
        assert_eq!(
            transform(
                syn::parse_str::<dsl_hir::Device>(
                    "
                    #[cfg(foo)]
                    buffer Foo = 5
                    "
                )
                .unwrap()
            )
            .unwrap()
            .objects,
            &[mir::Object::Buffer(mir::Buffer {
                cfg_attr: Some("foo".into()),
                description: "".into(),
                name: "Foo".into(),
                access: mir::Access::default(),
                address: 5,
            })]
        );
        assert_eq!(
            transform(
                syn::parse_str::<dsl_hir::Device>(
                    "
                    #[cfg(foo)]
                    #[cfg(too_many)]
                    buffer Foo = 5
                    "
                )
                .unwrap()
            )
            .unwrap_err()
            .to_string(),
            "Only one cfg attribute is allowed, but 2 are found"
        );
    }
}
