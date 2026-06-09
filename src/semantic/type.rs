use crate::key_vec::{
    Index, KeyVec, Sentinels,
    Value::{Item, Sentinel},
};

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
#[derive(Sentinels, Clone, Copy, Debug, PartialEq, Eq)]
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
    match (types.get(lhs), types.get(rhs)) {
        (Sentinel(TypeSentinel::Unknown), _) => rhs,
        (_, Sentinel(TypeSentinel::Unknown)) => lhs,
        (Sentinel(TypeSentinel::Unit), Sentinel(TypeSentinel::Unit)) => {
            TypeSentinel::Unit.to_index()
        }
        (Sentinel(TypeSentinel::Uint32), Sentinel(TypeSentinel::Uint32)) => {
            TypeSentinel::Uint32.to_index()
        }
        (Sentinel(TypeSentinel::Bool), Sentinel(TypeSentinel::Bool))
        | (Sentinel(TypeSentinel::True), Sentinel(TypeSentinel::False))
        | (Sentinel(TypeSentinel::False), Sentinel(TypeSentinel::True)) => {
            TypeSentinel::Bool.to_index()
        }
        (Sentinel(TypeSentinel::False), Sentinel(TypeSentinel::False)) => {
            TypeSentinel::False.to_index()
        }
        (Sentinel(TypeSentinel::True), Sentinel(TypeSentinel::True)) => {
            TypeSentinel::True.to_index()
        }
        (
            Item(&TypeData::Function {
                argument_type: lhs_arg,
                return_type: lhs_ret,
            }),
            Item(&TypeData::Function {
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
        (Item(TypeData::Product { .. }), Item(TypeData::Product { .. })) => lhs,
        (a, b) => panic!("No rules to merge types {a:?} and {b:?}"),
    }
}

pub fn types_equals(types: &Types, lhs: Type, rhs: Type) -> bool {
    match (types.get(lhs), types.get(rhs)) {
        (Sentinel(TypeSentinel::Unknown), Sentinel(TypeSentinel::Unknown)) => true,
        (Sentinel(TypeSentinel::Unit), Sentinel(TypeSentinel::Unit)) => true,
        (Sentinel(TypeSentinel::Uint32), Sentinel(TypeSentinel::Uint32)) => true,
        (Sentinel(TypeSentinel::Bool), Sentinel(TypeSentinel::Bool)) => true,
        (Sentinel(TypeSentinel::False), Sentinel(TypeSentinel::False)) => true,
        (Sentinel(TypeSentinel::True), Sentinel(TypeSentinel::True)) => true,
        (
            Sentinel(TypeSentinel::Unit)
            | Sentinel(TypeSentinel::Uint32)
            | Sentinel(TypeSentinel::Bool)
            | Sentinel(TypeSentinel::False)
            | Sentinel(TypeSentinel::True),
            _,
        ) => false,
        (
            Item(TypeData::Product { fields: lhs_fields }),
            Item(TypeData::Product { fields: rhs_fields }),
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
