//! Easy library with some abstractions to implement cache stores.
//!
//! Provides several features like:
//! - Traits to implement cache stores. Feature faillible and infallible variants.
//! - Cache stores with default generators that activate by default when needed.
//! - Thread safe variants of everything possible under the "thread-safe" feature.
//! - Default cache stores implemented for filesystem, memory, etc. (might require some features)
//!
//!
//! # Examples
//! - [stores]: For examples on some common stores implemented.
//! - [generative]: For examples on the concept of generative cache stores.
//!
//! # Contributing, Issues & Discussions
//! For anything related, please consult the official repository:
//! <https://github.com/javalsai/rs-ezcache>

#![no_std]
#[cfg(feature = "std")]
extern crate std;

pub mod generative;
#[cfg(feature = "std")]
pub mod stores;
#[cfg(feature = "thread-safe")]
pub mod thread_safe;

use crate::__internal_prelude::*;

/// Trait for a infallible cache store
#[delegatable_trait]
pub trait CacheStore {
    type Key;
    type Value;

    /// Returns an option of the owned cache element if present
    fn get(&self, key: impl Borrow<Self::Key>) -> Option<Self::Value>;
    /// Sets a value given its key
    fn set(&mut self, key: impl Borrow<Self::Key>, value: impl Borrow<Self::Value>);
    /// Checks if the cache entry exists
    fn exists(&self, key: impl Borrow<Self::Key>) -> bool {
        self.get(key).is_some()
    }
}

/// Trait for a fallible cache store, analogous to [CacheStore]
#[delegatable_trait]
#[allow(clippy::missing_errors_doc)]
pub trait TryCacheStore {
    type Key;
    type Value;
    type Error;

    /// Attempts to return an option of the owned cache element if present
    fn try_get(&self, key: impl Borrow<Self::Key>) -> Result<Option<Self::Value>, Self::Error>;
    /// Attempts to set a value given its key.
    fn try_set(
        &mut self,
        key: impl Borrow<Self::Key>,
        value: impl Borrow<Self::Value>,
    ) -> Result<(), Self::Error>;
    /// Attempts to check if the cache key entry exists.
    fn try_exists(&self, key: impl Borrow<Self::Key>) -> Result<bool, Self::Error> {
        self.try_get(key).map(|v| v.is_some())
    }
}

/// Allow any [`CacheStore`] to behave as a [`TryCacheStore`] that never fails.
impl<T: CacheStore> TryCacheStore for T {
    type Key = T::Key;
    type Value = T::Value;
    type Error = Infallible;

    fn try_get(&self, key: impl Borrow<Self::Key>) -> Result<Option<Self::Value>, Self::Error> {
        Ok(self.get(key))
    }

    fn try_set(
        &mut self,
        key: impl Borrow<Self::Key>,
        value: impl Borrow<Self::Value>,
    ) -> Result<(), Self::Error> {
        #[allow(clippy::unit_arg)]
        Ok(self.set(key, value))
    }

    fn try_exists(&self, key: impl Borrow<Self::Key>) -> Result<bool, Self::Error> {
        Ok(self.exists(key))
    }
}

/// Struct to convert the error type of a [`TryCacheStore`] into another
pub struct TryCacheStoreErrorMap<K, V, E, ET, S: TryCacheStore<Key = K, Value = V, Error = E>> {
    pub store: S,
    __phantom: PhantomData<ET>,
}

impl<K, V, E, ET: From<E>, S: TryCacheStore<Key = K, Value = V, Error = E>>
    TryCacheStoreErrorMap<K, V, E, ET, S>
{
    pub fn from_store(store: S) -> Self {
        Self::from(store)
    }
}

impl<K, V, E, ET: From<E>, S: TryCacheStore<Key = K, Value = V, Error = E>> TryCacheStore
    for TryCacheStoreErrorMap<K, V, E, ET, S>
{
    type Key = K;
    type Value = V;
    type Error = ET;

    fn try_get(&self, key: impl Borrow<Self::Key>) -> Result<Option<Self::Value>, Self::Error> {
        self.store.try_get(key).map_err(Into::into)
    }

    fn try_set(
        &mut self,
        key: impl Borrow<Self::Key>,
        value: impl Borrow<Self::Value>,
    ) -> Result<(), Self::Error> {
        self.store.try_set(key, value).map_err(Into::into)
    }

    fn try_exists(&self, key: impl Borrow<Self::Key>) -> Result<bool, Self::Error> {
        self.store.try_exists(key).map_err(Into::into)
    }
}

impl<K, V, E, ET: From<E>, T: TryCacheStore<Key = K, Value = V, Error = E>> From<T>
    for TryCacheStoreErrorMap<K, V, E, ET, T>
{
    fn from(value: T) -> Self {
        Self {
            store: value,
            __phantom: PhantomData,
        }
    }
}

pub mod prelude {
    //! Prelude of the module.
    //!
    //! Provides basic types across the module whose names shouldn't conflict with any other
    //! imported elements from other crates.

    // pub use crate::generative::{GenCacheStore, TryGenCacheStore};
    pub use crate::generative::{TryGenCacheStore, TryGenCacheStoreWrapper};
    #[cfg(feature = "std")]
    pub use crate::stores::MemoryStore;
    #[cfg(feature = "thread-safe")]
    pub use crate::thread_safe::ThreadSafeTryCacheStore;
    pub use crate::{CacheStore, TryCacheStore};
}

mod __internal_prelude {
    pub use core::{borrow::Borrow, convert::Infallible, marker::PhantomData};

    pub use crate::prelude::*;
    #[allow(unused_imports)]
    pub use crate::TryCacheStoreErrorMap;
    #[allow(unused_imports)]
    pub use ambassador::{delegatable_trait, Delegate};
}
