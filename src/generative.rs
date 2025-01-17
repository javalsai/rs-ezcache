//! This module provides cache stores with generator functions.
//!
//! This means that if a entry is not found, it's possible to automatically call the generator
//! function to generate such key.
//!
//! Traits:
//! - [`GenCacheStore`]: The default infallible trait.
//! - [`TryGenCacheStore`]: The fallible flavour.
//!
//! This also provides wrappers for normal stores to attach a generator to:
//! - [`GenCacheStoreWrapper`]: The default infallible wrapper.
//! - [`TryGenCacheStoreWrapper`]: The fallible flavour.
//!
//! # Examples
//! ```rust
//! # use ezcache::{
//! #     generative::{GenCacheStore, GenCacheStoreWrapper},
//! #     stores::MemoryStore
//! # };
//! # use ezcache::prelude::*;
//! #
//! // This would obviously be something more complex, perhaps even handling long-awaited io
//! let very_heavy_computation = |&n: &usize, ()| n * 2;
//! // You wrap this around a normal store that you want
//! let store = MemoryStore::<usize, usize>::default();
//!
//! // And combine them (here is where the magic happens)
//! let mut gen_store = GenCacheStoreWrapper::new(store, very_heavy_computation);
//!
//! assert_eq!(gen_store.get(2), None);
//! assert_eq!(gen_store.get_or_new(2, ()), 4);
//! assert_eq!(gen_store.get(2), Some(4));
//! ```
//!
//! ```rust
//! # use ezcache::{
//! #     generative::{GenCacheStore, GenCacheStoreWrapper},
//! #     stores::MemoryStore
//! # };
//! # use ezcache::prelude::*;
//! #
//! // We can eve pass additional arguments that we dont want to cache the keys by.
//! // This could be a one-time source that changes but is a valid to represent the cache key.
//!
//! let very_heavy_computation = |&n: &usize, offset: usize| n * 2 + offset;
//! let store = MemoryStore::<usize, usize>::default();
//!
//! let mut gen_store = GenCacheStoreWrapper::new(store, very_heavy_computation);
//!
//! assert_eq!(gen_store.get(2), None); // Key hasn't been generated so far
//! assert_eq!(gen_store.get_or_new(2, 0), 4); // We generate such entry
//! assert_eq!(gen_store.get(2), Some(4)); // Now it exists
//!
//! // As it exists, it won't generate a new value, even if the result would change
//! assert_eq!(gen_store.get_or_new(2, 1), 4);
//! assert_eq!(gen_store.gen_new(2, 1), 5); // Unless we explicitly tell it to
//! assert_eq!(gen_store.get(2), Some(5)); // And then it's saved
//!
//! // WARNING: Extra arguments should NOT be important for the cache key, should only have
//! // information that you do NOT want to index in the underlying cache store. But you still want
//! // to pass to the generator for any reasons.
//! ```

use crate::__internal_prelude::*;

/// Infallible generative cache store.
#[delegatable_trait]
pub trait GenCacheStore:
    CacheStore<Key = <Self as GenCacheStore>::Key, Value = <Self as GenCacheStore>::Value>
{
    type Key;
    type Value;
    type Args;

    /// Generate a new value without checking cache or adding the value to it.
    fn gen(
        &self,
        key: impl Borrow<<Self as GenCacheStore>::Key>,
        args: Self::Args,
    ) -> <Self as GenCacheStore>::Value;
    /// Get the value from cache or generate a new one without adding it.
    fn get_or_gen(
        &self,
        key: impl Borrow<<Self as GenCacheStore>::Key>,
        args: Self::Args,
    ) -> <Self as GenCacheStore>::Value;
    /// Get the value from cache or generate a new one adding it.
    fn get_or_new(
        &mut self,
        key: impl Borrow<<Self as GenCacheStore>::Key>,
        args: Self::Args,
    ) -> <Self as GenCacheStore>::Value;
    /// Generate a new value without checking cache and add the value to it, possibly overwriting
    /// previous values.
    fn gen_new(
        &mut self,
        key: impl Borrow<<Self as GenCacheStore>::Key>,
        args: Self::Args,
    ) -> <Self as GenCacheStore>::Value;
}

use super::ambassador_impl_CacheStore;
#[derive(Delegate)]
#[delegate(CacheStore, target = "store")]
/// Infallible generative cache store wrapper around a [`CacheStore`] and a generator function.
///
/// Generics:
/// - `K`: Type of the key used for cache indexing.
/// - `V`: Type of the value stored in the cache store.
/// - `A`: Type of additional arguments of the generator function.
/// - `S`: [`CacheStore`] which this wraps around.
/// - `F`: [`Fn<&K, A>`] with `V` return generator function.
pub struct GenCacheStoreWrapper<K, V, A, S: CacheStore<Key = K, Value = V>, F: Fn(&K, A) -> V> {
    pub store: S,
    pub generator: F,
    phantom: PhantomData<(K, V, A)>,
}

/// Default implementation
impl<K, V, A, F: Fn(&K, A) -> V, S: CacheStore<Key = K, Value = V>>
    GenCacheStoreWrapper<K, V, A, S, F>
{
    /// Make a new [`GenCacheStoreWrapper`] from a infallible store and a generator function.
    pub fn new(store: S, generator: F) -> Self {
        Self {
            store,
            generator,
            phantom: PhantomData,
        }
    }
}

/// Implement [`GenCacheStore`]
impl<K, V, A, S: CacheStore<Key = K, Value = V>, F: Fn(&K, A) -> V> GenCacheStore
    for GenCacheStoreWrapper<K, V, A, S, F>
{
    type Key = K;
    type Value = V;
    type Args = A;

    fn gen(&self, key: impl Borrow<K>, args: A) -> V {
        (self.generator)(key.borrow(), args)
    }

    fn get_or_gen(&self, key: impl Borrow<K>, args: A) -> V {
        self.store
            .get(key.borrow())
            .unwrap_or_else(|| self.gen(key, args))
    }

    fn get_or_new(&mut self, key: impl Borrow<K>, args: A) -> V {
        let value = self.get_or_gen(key.borrow(), args);
        self.store.set(key, &value);
        value
    }

    fn gen_new(&mut self, key: impl Borrow<K>, args: A) -> V {
        let value = self.gen(key.borrow(), args);
        self.store.set(key.borrow(), &value);
        value
    }
}

// --------------------- **TRY**
// ----

/// Fallible generative cache store.
#[delegatable_trait]
#[allow(clippy::missing_errors_doc)]
pub trait TryGenCacheStore:
    TryCacheStore<
    Key = <Self as TryGenCacheStore>::Key,
    Value = <Self as TryGenCacheStore>::Value,
    Error = <Self as TryGenCacheStore>::Error,
>
{
    type Key;
    type Value;
    type Error;
    type Args;

    /// Attempt to generate a new value without checking cache or adding the value to it.
    fn try_gen(
        &self,
        key: impl Borrow<<Self as TryGenCacheStore>::Key>,
        args: <Self as TryGenCacheStore>::Args,
    ) -> Result<<Self as TryGenCacheStore>::Value, <Self as TryCacheStore>::Error>;
    /// Attempt to get the value from cache or generate a new one without adding it.
    fn try_get_or_gen(
        &self,
        key: impl Borrow<<Self as TryGenCacheStore>::Key>,
        args: <Self as TryGenCacheStore>::Args,
    ) -> Result<<Self as TryGenCacheStore>::Value, <Self as TryCacheStore>::Error>;
    /// Attempt to get the value from cache or generate a new one attempting to add it.
    fn try_get_or_new(
        &mut self,
        key: impl Borrow<<Self as TryGenCacheStore>::Key>,
        args: <Self as TryGenCacheStore>::Args,
    ) -> Result<<Self as TryGenCacheStore>::Value, <Self as TryCacheStore>::Error>;
    /// Attempt to generate a new value without checking cache and attempting to add the value to
    /// it, possibly overwriting previous values.
    fn try_gen_new(
        &mut self,
        key: impl Borrow<<Self as TryGenCacheStore>::Key>,
        args: <Self as TryGenCacheStore>::Args,
    ) -> Result<<Self as TryGenCacheStore>::Value, <Self as TryCacheStore>::Error>;
}

use crate::ambassador_impl_TryCacheStore;
#[derive(Delegate)]
#[delegate(TryCacheStore, target = "store")]
/// Infallible generative cache store wrapper around a [`CacheStore`] and a generator function.
///
/// Generics:
/// - `K`: Type of the key used for cache indi.
/// - `V`: Type of the value stored in the cache store.
/// - `E`: Error type used for [`Result`]s
/// - `A`: Type of additional arguments of the generator function.
/// - `FnErr`: Error type of the function.
/// - `S`: [`CacheStore`] which this wraps around.
/// - `F`: [`Fn<&K, A>`] with  `V` return generator function.
pub struct TryGenCacheStoreWrapper<
    K,
    V,
    E,
    A,
    FnErr: Into<E>,
    S: TryCacheStore<Key = K, Value = V, Error = E>,
    F: Fn(&K, A) -> Result<V, FnErr>,
> {
    pub store: S,
    pub try_generator: F,
    phantom: PhantomData<(K, V, E, A)>,
}

/// Default implementation
impl<
        K,
        V,
        E,
        A,
        FnErr: Into<E>,
        F: Fn(&K, A) -> Result<V, FnErr>,
        S: TryCacheStore<Key = K, Value = V, Error = E>,
    > TryGenCacheStoreWrapper<K, V, E, A, FnErr, S, F>
{
    /// Make a new [`TryGenCacheStore`] from a fallible store and fallible generator function.
    pub fn new(store: S, try_generator: F) -> Self {
        Self {
            store,
            try_generator,
            phantom: PhantomData,
        }
    }
}

/// Functions with multiple stages will return the same type of error without any way to detect at
/// what point it failed, and not undoing the changes. If you don't like this you'll have to
/// manually follow the steps done by the function and handle the errors yourself.
impl<
        K,
        V,
        E,
        A,
        FnErr: Into<E>,
        F: Fn(&K, A) -> Result<V, FnErr>,
        S: TryCacheStore<Key = K, Value = V, Error = E>,
    > TryGenCacheStore for TryGenCacheStoreWrapper<K, V, E, A, FnErr, S, F>
{
    type Key = K;
    type Value = V;
    type Error = E;
    type Args = A;

    /// Attempt to generate a new value without checking cache or adding the value to it.
    fn try_gen(&self, key: impl Borrow<K>, args: A) -> Result<V, E> {
        (self.try_generator)(key.borrow(), args).map_err(Into::into)
    }

    /// Attempt to get the value from cache or generate a new one without adding it.
    fn try_get_or_gen(&self, key: impl Borrow<K>, args: A) -> Result<V, E> {
        let value = self.store.try_get(key.borrow())?;
        if let Some(value) = value {
            Ok(value)
        } else {
            self.try_gen(key, args)
        }
    }

    /// Attempt to get the value from cache or generate a new one attempting to add it.
    fn try_get_or_new(&mut self, key: impl Borrow<K>, args: A) -> Result<V, E> {
        let value = self.try_get_or_gen(key.borrow(), args)?;
        self.store.try_set(key, &value)?;
        Ok(value)
    }

    /// Attempt to generate a new value without checking cache and attempting to add the value to
    /// it, possibly overwriting previous values.
    fn try_gen_new(&mut self, key: impl Borrow<K>, args: A) -> Result<V, E> {
        let value = self.try_gen(key.borrow(), args)?;
        self.store.try_set(key.borrow(), &value)?;
        Ok(value)
    }
}

/// Implement [`TryGenCacheStore`]
impl<K, V, A, T: GenCacheStore<Key = K, Value = V, Args = A>> TryGenCacheStore for T {
    type Key = K;
    type Value = V;
    type Error = core::convert::Infallible;
    type Args = A;

    fn try_gen(
        &self,
        key: impl Borrow<<Self as TryGenCacheStore>::Key>,
        args: <Self as TryGenCacheStore>::Args,
    ) -> Result<<Self as TryGenCacheStore>::Value, <Self as TryCacheStore>::Error> {
        Ok(self.gen(key, args))
    }

    fn try_gen_new(
        &mut self,
        key: impl Borrow<<Self as TryGenCacheStore>::Key>,
        args: <Self as TryGenCacheStore>::Args,
    ) -> Result<<Self as TryGenCacheStore>::Value, <Self as TryCacheStore>::Error> {
        Ok(self.gen_new(key, args))
    }

    fn try_get_or_gen(
        &self,
        key: impl Borrow<<Self as TryGenCacheStore>::Key>,
        args: <Self as TryGenCacheStore>::Args,
    ) -> Result<<Self as TryGenCacheStore>::Value, <Self as TryCacheStore>::Error> {
        Ok(self.get_or_gen(key, args))
    }

    fn try_get_or_new(
        &mut self,
        key: impl Borrow<<Self as TryGenCacheStore>::Key>,
        args: <Self as TryGenCacheStore>::Args,
    ) -> Result<<Self as TryGenCacheStore>::Value, <Self as TryCacheStore>::Error> {
        Ok(self.get_or_new(key, args))
    }
}
