//! A [`KeyVec`] equivalent to a [`Vec`] with typed index and sentinel values in
//! the index itself.
//!
//! [`KeyVec<S, I>`] are indexed using the [`Index<S>`] type. You can obtain
//! [`Index<S>`] whenever you push a value to the [`KeyVec<S, I>`].
//!
//! [`KeyVec<S, I>`] do not currently support removal of items.
//!
//! The `S` generic in [`KeyVec<S, I>`] both serves as a marker for the
//! [`KeyVec`] and as a enumeration of sentinel values stored in the index
//! itself.
//!
//! The purpose of it as a marker is to avoid indexing a [`KeyVec`] with an
//! index from another [`KeyVec`].
//!
//! The `S` must implements the [`Sentinel`] trait, you should implement that
//! trait using the associated derive-macro.

use std::{fmt::Debug, hash::Hash, marker::PhantomData};

pub struct KeyVec<S: Sentinels, I>(Vec<I>, PhantomData<S>);

#[derive(Debug)]
pub enum Value<S, I> {
    Sentinel(S),
    Item(I),
}

pub trait Sentinels: Sized + Clone + Copy {
    fn from_index(index: Index<Self>) -> Option<Self>;

    fn to_index(self) -> Index<Self>;
}

#[derive(Clone, Copy)]
pub struct Index<S: Sentinels> {
    index: u32,
    __phantom_data: PhantomData<S>,
}

impl<S: Sentinels, I> KeyVec<S, I> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn from_vec(vec: Vec<I>) -> Self {
        Self(vec, PhantomData)
    }

    pub fn push(&mut self, value: I) -> Index<S> {
        let index = Index {
            index: self.0.len() as u32,
            __phantom_data: PhantomData,
        };
        self.0.push(value);
        index
    }

    pub fn entries(&self) -> impl Iterator<Item = (Index<S>, &I)> {
        self.0.iter().enumerate().map(|(i, v)| {
            (
                Index {
                    index: i as u32,
                    __phantom_data: PhantomData,
                },
                v,
            )
        })
    }
}

impl<S: Sentinels, I> KeyVec<S, I> {
    pub fn get(&self, index: Index<S>) -> Value<S, &I> {
        match S::from_index(index) {
            Some(sentinel) => Value::Sentinel(sentinel),
            None => Value::Item(&self.0[index.index as usize]),
        }
    }

    pub fn get_mut(&mut self, index: Index<S>) -> Value<S, &mut I> {
        match S::from_index(index) {
            Some(sentinel) => Value::Sentinel(sentinel),
            None => Value::Item(&mut self.0[index.index as usize]),
        }
    }
}

impl<S: Sentinels, I> core::ops::Index<Index<S>> for KeyVec<S, I> {
    type Output = I;

    fn index(&self, index: Index<S>) -> &Self::Output {
        &self.0[index.index as usize]
    }
}

impl<S: Sentinels, I> core::ops::IndexMut<Index<S>> for KeyVec<S, I> {
    fn index_mut(&mut self, index: Index<S>) -> &mut Self::Output {
        &mut self.0[index.index as usize]
    }
}

impl<S: Sentinels, I> Default for KeyVec<S, I> {
    fn default() -> Self {
        Self(Vec::new(), PhantomData)
    }
}

impl<S: Sentinels, I: Debug> Debug for KeyVec<S, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<S: Sentinels> PartialEq for Index<S> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<S: Sentinels> Eq for Index<S> {}

impl<S: Sentinels> Hash for Index<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl<S: Sentinels> Index<S> {
    pub const fn as_u32(self) -> u32 {
        self.index
    }

    pub const fn from_u32_index(index: u32) -> Self {
        Index {
            index,
            __phantom_data: PhantomData,
        }
    }

    pub fn sentinel(self) -> Option<S> {
        S::from_index(self)
    }
}

mod derive {
    macro_rules! Sentinels {
        derive() (
            $(#[$($attr:tt)*])* $vis:vis enum $name:ident {
                $($variant:ident $(= $value:expr)?),* $(,)?
            }
        ) => {
            impl $crate::key_vec::Sentinels for $name {
                fn from_index(
                    index: $crate::key_vec::Index<Self>,
                ) -> ::core::option::Option<Self> {
                    $(
                        #[allow(non_upper_case_globals)]
                        const $variant: u32 = $name::$variant as u32;
                    )*

                    match index.as_u32() {
                        $(
                            #[allow(non_upper_case_globals)]
                            $variant => ::core::option::Option::Some($name::$variant),
                        )*
                        _ => ::core::option::Option::None,
                    }
                }

                fn to_index(self) -> $crate::key_vec::Index<Self> {
                    $crate::key_vec::Index::<Self>::from_u32_index(self as u32)
                }
            }

            impl ::core::fmt::Debug for $crate::key_vec::Index<$name> {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    if let ::core::option::Option::Some(sentinel) = <$name as $crate::key_vec::Sentinels>::from_index(*self) {
                        ::core::fmt::Debug::fmt(&sentinel, f)
                    } else {
                        f.debug_tuple(stringify!($name)).field(&self.as_u32()).finish()
                    }
                }
            }
        };
    }

    pub(crate) use Sentinels;
}

pub(crate) use derive::Sentinels;
