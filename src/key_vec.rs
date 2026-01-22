//! A [`KeyVec`] equivalent to a [`Vec`] with typed index and sentinel values in
//! the index itself.
//!
//! [`KeyVec<S, V>`] are indexed using the [`Index<S>`] type. You can obtain
//! [`Index<S>`] whenever you push a value to the [`KeyVec<S, V>`].
//!
//! [`KeyVec<S, V>`] do not currently support removal of items.
//!
//! The `S` generic in [`KeyVec<S, V>`] both serves as a marker for the
//! [`KeyVec`] and as a enumeration of sentinel values stored in the index
//! itself.
//!
//! The purpose of it as a marker is to avoid indexing a [`KeyVec`] with an
//! index from another [`KeyVec`].
//!
//! The `S` must implements the [`Sentinel`] trait, you should implement that
//! trait using the associated derive-macro.
//!
//! The [`Sentinel`] derive-macro also derive on of [`NonEmptySentinel`] or
//! [`EmptySentinel`] based on wether you sentinel enum has variants or not.
//!
//! [`NonEmptySentinel`] and [`EmptySentinel`] are used to provide more
//! ergonomics to the indexing of the [`KeyVec`].
//!
//! On a [`KeyVec`] with [`EmptySentinel`], we can use indexing syntax for
//! getting elements.
//!
//! On a [`KeyVec`] with a [`NonEmptySentinel`], we can use [`KeyVec::get`] and
//! [`KeyVec::get_mut`].
//!
//! A [`EmptySentinel`] kind of defy the purpose of a sentinel, it is only used
//! as a marker for the collection.

use std::{fmt::Debug, hash::Hash, marker::PhantomData};

pub struct KeyVec<S: Sentinel, V>(Vec<V>, PhantomData<S>);

#[derive(Debug)]
pub enum Val<S, V> {
    None,
    Sentinel(S),
    Value(V),
}

pub trait Sentinel: Sized + Clone + Copy {
    fn from_index(index: Index<Self>) -> Option<Self>;

    fn to_index(self) -> Index<Self>;
}

pub trait NonEmptySentinel: Sentinel {}

pub unsafe trait EmptySentinel: Sentinel {}

#[derive(Clone, Copy)]
pub struct Index<S: Sentinel> {
    index: u32,
    __phantom_data: PhantomData<S>,
}

impl<S: Sentinel, V> KeyVec<S, V> {
    pub fn from_vec(vec: Vec<V>) -> Self {
        Self(vec, PhantomData)
    }

    pub fn push(&mut self, value: V) -> Index<S> {
        let index = Index {
            index: self.0.len() as u32,
            __phantom_data: PhantomData,
        };
        self.0.push(value);
        index
    }

    pub fn entries(&self) -> impl Iterator<Item = (Index<S>, &V)> {
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

impl<S: NonEmptySentinel, V> KeyVec<S, V> {
    pub fn get(&self, index: Index<S>) -> Val<S, &V> {
        match S::from_index(index) {
            Some(sentinel) => Val::Sentinel(sentinel),
            None => match self.0.get(index.index as usize) {
                None => Val::None,
                Some(value) => Val::Value(value),
            },
        }
    }

    pub fn get_mut(&mut self, index: Index<S>) -> Val<S, &mut V> {
        match S::from_index(index) {
            Some(sentinel) => Val::Sentinel(sentinel),
            None => match self.0.get_mut(index.index as usize) {
                Some(value) => Val::Value(value),
                None => Val::None,
            },
        }
    }
}

impl<S: EmptySentinel, V> core::ops::Index<Index<S>> for KeyVec<S, V> {
    type Output = V;

    fn index(&self, index: Index<S>) -> &Self::Output {
        &self.0[index.index as usize]
    }
}

impl<S: EmptySentinel, V> core::ops::IndexMut<Index<S>> for KeyVec<S, V> {
    fn index_mut(&mut self, index: Index<S>) -> &mut Self::Output {
        &mut self.0[index.index as usize]
    }
}

impl<S: Sentinel, V> Default for KeyVec<S, V> {
    fn default() -> Self {
        Self(Vec::new(), PhantomData)
    }
}

impl<S: Sentinel, V: Debug> Debug for KeyVec<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<S: Sentinel> PartialEq for Index<S> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<S: Sentinel> Eq for Index<S> {}

impl<S: Sentinel> Hash for Index<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl<S: Sentinel> Index<S> {
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
    macro_rules! Sentinel {
        derive() (
            $(#[$($attr:tt)*])* $vis:vis enum $name:ident {}
        ) => {
            // SAFETY: Guarrantee that the enum has size `0`.
            const _: () = ::core::assert!(::core::mem::size_of::<$name>() == 0);
            // SAFETY: Guarrantee that the enum has no variant, i.e. the enum
            // is not inhabited.
            const _: () = {
                let _ = |value: $name| match value {};
            };

            // SAFETY: We triple check that the enum is empty.
            // - The derive macro syntax parses only enum with no variants
            // - We assert at compile-time that the size of the `enum` is `0`
            // - We assert that the `enum` is not inhabited with a empty `match`
            unsafe impl $crate::key_vec::EmptySentinel for $name { }

            impl $crate::key_vec::Sentinel for $name {
                fn from_index(
                    index: $crate::key_vec::Index<Self>,
                ) -> Option<Self> {
                    None
                }

                fn to_index(self) -> $crate::key_vec::Index<Self> {
                    match self {}
                }
            }

            impl ::core::fmt::Debug for $crate::key_vec::Index<$name> {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.debug_tuple(stringify!($name)).field(&self.as_u32()).finish()
                }
            }
        };

        derive() (
            $(#[$($attr:tt)*])* $vis:vis enum $name:ident {
                $($variant:ident $(= $value:expr)?),* $(,)?
            }
        ) => {
            impl $crate::key_vec::NonEmptySentinel for $name {}

            impl $crate::key_vec::Sentinel for $name {
                fn from_index(
                    index: $crate::key_vec::Index<Self>,
                ) -> Option<Self> {
                    $(
                        #[allow(non_upper_case_globals)]
                        const $variant: u32 = $name::$variant as u32;
                    )*

                    match index.as_u32() {
                        $(
                            #[allow(non_upper_case_globals)]
                            $variant => Some($name::$variant),
                        )*
                        _ => None,
                    }
                }

                fn to_index(self) -> $crate::key_vec::Index<Self> {
                    $crate::key_vec::Index::<Self>::from_u32_index(self as u32)
                }
            }

            impl ::core::fmt::Debug for $crate::key_vec::Index<$name> {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    if let Some(sentinel) = <$name as $crate::key_vec::Sentinel<>>::from_index(*self) {
                        ::core::fmt::Debug::fmt(&sentinel, f)
                    } else {
                        f.debug_tuple(stringify!($name)).field(&self.as_u32()).finish()
                    }
                }
            }
        };
    }

    pub(crate) use Sentinel;
}

pub(crate) use derive::Sentinel;
