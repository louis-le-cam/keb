use crate::key_vec::{Index, KeyVec, Sentinel, Val};

#[derive(Debug)]
pub enum TypeData {
    Function {
        argument_type: Type,
        return_type: Type,
    },
    Product {
        fields: Vec<(String, Type)>,
    },
}

#[repr(u32)]
#[derive(Sentinel, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeSentinel {
    Unknown = u32::MAX - 5,
    Unit,
    Uint32,
    Bool,
    False,
    True,
}

pub type Type = Index<TypeSentinel>;
pub type Types = KeyVec<TypeSentinel, TypeData>;

// Combine two source of informations into one, panicking if there is any
// mismatch.
pub fn combine_types(types: &mut Types, lhs: Type, rhs: Type) -> Type {
    match (types.get_val(lhs), types.get_val(rhs)) {
        (Val::Sentinel(TypeSentinel::Unknown), _) => rhs,
        (_, Val::Sentinel(TypeSentinel::Unknown)) => lhs,
        (Val::Sentinel(TypeSentinel::Unit), Val::Sentinel(TypeSentinel::Unit)) => {
            TypeSentinel::Unit.to_index()
        }
        (Val::Sentinel(TypeSentinel::Uint32), Val::Sentinel(TypeSentinel::Uint32)) => {
            TypeSentinel::Uint32.to_index()
        }
        (Val::Sentinel(TypeSentinel::Bool), Val::Sentinel(TypeSentinel::Bool))
        | (Val::Sentinel(TypeSentinel::True), Val::Sentinel(TypeSentinel::False))
        | (Val::Sentinel(TypeSentinel::False), Val::Sentinel(TypeSentinel::True)) => {
            TypeSentinel::Bool.to_index()
        }
        (Val::Sentinel(TypeSentinel::False), Val::Sentinel(TypeSentinel::False)) => {
            TypeSentinel::False.to_index()
        }
        (Val::Sentinel(TypeSentinel::True), Val::Sentinel(TypeSentinel::True)) => {
            TypeSentinel::True.to_index()
        }
        (
            Val::Value(&TypeData::Function {
                argument_type: lhs_arg,
                return_type: lhs_ret,
            }),
            Val::Value(&TypeData::Function {
                argument_type: rhs_arg,
                return_type: rhs_ret,
            }),
        ) => {
            let type_ = TypeData::Function {
                argument_type: combine_types(types, lhs_arg, rhs_arg),
                return_type: combine_types(types, lhs_ret, rhs_ret),
            };
            types.push(type_)
        }
        // TODO: actually merge both products
        (Val::Value(TypeData::Product { .. }), Val::Value(TypeData::Product { .. })) => lhs,
        (a, b) => panic!("No rules to merge types {a:?} and {b:?}"),
    }
}

pub fn types_equals(types: &Types, lhs: Type, rhs: Type) -> bool {
    match (types.get_val(lhs), types.get_val(rhs)) {
        (Val::Sentinel(TypeSentinel::Unknown), Val::Sentinel(TypeSentinel::Unknown)) => true,
        (Val::Sentinel(TypeSentinel::Unit), Val::Sentinel(TypeSentinel::Unit)) => true,
        (Val::Sentinel(TypeSentinel::Uint32), Val::Sentinel(TypeSentinel::Uint32)) => true,
        (Val::Sentinel(TypeSentinel::Bool), Val::Sentinel(TypeSentinel::Bool)) => true,
        (Val::Sentinel(TypeSentinel::False), Val::Sentinel(TypeSentinel::False)) => true,
        (Val::Sentinel(TypeSentinel::True), Val::Sentinel(TypeSentinel::True)) => true,
        (
            Val::Sentinel(TypeSentinel::Unit)
            | Val::Sentinel(TypeSentinel::Uint32)
            | Val::Sentinel(TypeSentinel::Bool)
            | Val::Sentinel(TypeSentinel::False)
            | Val::Sentinel(TypeSentinel::True),
            _,
        ) => false,
        (
            Val::Value(TypeData::Product { fields: lhs_fields }),
            Val::Value(TypeData::Product { fields: rhs_fields }),
        ) => {
            lhs_fields.len() == rhs_fields.len()
                && lhs_fields
                    .iter()
                    .zip(rhs_fields)
                    .all(|((_, lhs_field), (_, rhs_field))| {
                        types_equals(types, *lhs_field, *rhs_field)
                    })
        }
        (_, _) => false,
    }
}
