//! Thread safe traits for generative cache stores.

use core::borrow::Borrow;
use core::marker::PhantomData;
use std::sync::{MutexGuard, PoisonError};

use crate::__internal_prelude::*;

use super::ThreadSafeCacheStore;

/// Infalible thread safe generative cache store. This trait is **HIGHLY** discouraged for the
/// reasons explained in [`thread_safe`][crate::thread_safe]
#[delegatable_trait]
pub trait ThreadSafeGenCacheStore<'a>:
    super::ThreadSafeCacheStore<
    'a,
    Key = <Self as ThreadSafeGenCacheStore<'a>>::Key,
    Value = <Self as ThreadSafeGenCacheStore<'a>>::Value,
>
where
    Self: 'a,
{
    type Key;
    type Value;
    type Args;

    /// Generate a new value without checking cache or adding the value to it.
    fn ts_gen(
        &'a self,
        key: &'a <Self as ThreadSafeGenCacheStore<'a>>::Key,
        args: Self::Args,
    ) -> <Self as ThreadSafeGenCacheStore<'a>>::Value;
    /// Get the value from cache or generate a new one without adding it.
    fn ts_get_or_gen(
        &'a self,
        key: &'a <Self as ThreadSafeGenCacheStore<'a>>::Key,
        args: Self::Args,
    ) -> <Self as ThreadSafeGenCacheStore<'a>>::Value;
    /// Get the value from cache or generate a new one adding it.
    fn ts_get_or_new(
        &'a self,
        key: &'a <Self as ThreadSafeGenCacheStore<'a>>::Key,
        args: Self::Args,
    ) -> <Self as ThreadSafeGenCacheStore<'a>>::Value;
    /// Generate a new value without checking cache and add the value to it, possibly overwriting
    /// previous values.
    fn ts_gen_new(
        &'a self,
        key: &'a <Self as ThreadSafeGenCacheStore<'a>>::Key,
        args: Self::Args,
    ) -> <Self as ThreadSafeGenCacheStore<'a>>::Value;
}

/// Falible thread safe generative cache store.
#[delegatable_trait]
pub trait ThreadSafeTryGenCacheStore<'a>:
    super::ThreadSafeTryCacheStore<
    'a,
    Key = <Self as ThreadSafeTryGenCacheStore<'a>>::Key,
    Value = <Self as ThreadSafeTryGenCacheStore<'a>>::Value,
>
{
    type Key;
    type Value;
    type Error;
    type Args;

    /// Generate a new value without checking cache or adding the value to it.
    fn ts_try_gen(
        &self,
        key: &<Self as ThreadSafeTryGenCacheStore<'a>>::Key,
        args: Self::Args,
    ) -> Result<
        <Self as ThreadSafeTryGenCacheStore<'a>>::Value,
        <Self as ThreadSafeTryGenCacheStore<'a>>::Error,
    >;
    /// Get the value from cache or generate a new one without adding it.
    fn ts_try_get_or_gen(
        &self,
        key: &<Self as ThreadSafeTryGenCacheStore<'a>>::Key,
        args: Self::Args,
    ) -> Result<
        <Self as ThreadSafeTryGenCacheStore<'a>>::Value,
        <Self as ThreadSafeTryGenCacheStore<'a>>::Error,
    >;
    /// Get the value from cache or generate a new one adding it.
    fn ts_try_get_or_new(
        &self,
        key: &<Self as ThreadSafeTryGenCacheStore<'a>>::Key,
        args: Self::Args,
    ) -> Result<
        <Self as ThreadSafeTryGenCacheStore<'a>>::Value,
        <Self as ThreadSafeTryGenCacheStore<'a>>::Error,
    >;
    /// Generate a new value without checking cache and add the value to it, possibly overwriting
    /// previous values.
    fn ts_try_gen_new(
        &self,
        key: &<Self as ThreadSafeTryGenCacheStore<'a>>::Key,
        args: Self::Args,
    ) -> Result<
        <Self as ThreadSafeTryGenCacheStore<'a>>::Value,
        <Self as ThreadSafeTryGenCacheStore<'a>>::Error,
    >;
}

use super::ambassador_impl_ThreadSafeCacheStore;
#[derive(Delegate)]
#[delegate(ThreadSafeCacheStore<'a>, target = "store")]
/// Infallible thread safe generative cache store wrapper around a
/// [ThreadSafeCacheStore][super::ThreadSafeCacheStore] and a generator function.
///
/// Generics:
/// - `K`: Type of the key used for cache indexing.
/// - `V`: Type of the value stored in the cache store.
/// - `A`: Type of additional arguments of the generator function.
/// - `S`: [`ThreadSafeCacheStore`][super::ThreadSafeCacheStore] which this wraps around.
/// - `F`: [`Fn<&K, A>`] with `V` return generator function.
pub struct ThreadSafeGenCacheStoreWrapper<
    'a,
    K,
    V,
    A,
    S: super::ThreadSafeCacheStore<'a, Key = K, Value = V>,
    F: Fn(&K, A) -> V + 'a,
> {
    pub store: S,
    pub generator: F,
    phantom: PhantomData<&'a (K, V, A)>,
}

/// Default implementation
impl<'a, K, V, A, S: super::ThreadSafeCacheStore<'a, Key = K, Value = V>, F: Fn(&K, A) -> V>
    ThreadSafeGenCacheStoreWrapper<'a, K, V, A, S, F>
{
    /// Make a new [ThreadSafeGenCacheStoreWrapper] from a
    /// [ThreadSafeCacheStore][super::ThreadSafeCacheStore] and a generator function.
    pub fn new(store: S, generator: F) -> Self {
        Self {
            store,
            generator,
            phantom: PhantomData,
        }
    }
}

/// Implement [`ThreadSafeCacheStore`]
impl<'a, K, V, A, S: super::ThreadSafeCacheStore<'a, Key = K, Value = V>, F: Fn(&K, A) -> V>
    ThreadSafeGenCacheStore<'a> for ThreadSafeGenCacheStoreWrapper<'a, K, V, A, S, F>
{
    type Key = K;
    type Value = V;
    type Args = A;

    fn ts_gen(
        &'a self,
        key: &'a <Self as ThreadSafeGenCacheStore<'a>>::Key,
        args: Self::Args,
    ) -> <Self as ThreadSafeGenCacheStore<'a>>::Value {
        (self.generator)(key, args)
    }

    fn ts_get_or_gen(
        &'a self,
        key: &'a <Self as ThreadSafeGenCacheStore<'a>>::Key,
        args: Self::Args,
    ) -> <Self as ThreadSafeGenCacheStore<'a>>::Value {
        self.store
            .ts_one_get(key)
            .unwrap_or_else(|| self.ts_gen(key, args))
    }

    // FIXME: race conditions
    fn ts_get_or_new(
        &'a self,
        key: &'a <Self as ThreadSafeGenCacheStore<'a>>::Key,
        args: Self::Args,
    ) -> <Self as ThreadSafeGenCacheStore<'a>>::Value {
        let handle = self.ts_xlock(key);
        let shandle: &Self::SLock = &(&handle).into();
        let value = self
            .store
            .ts_get(&shandle)
            .unwrap_or_else(|| self.ts_gen(key, args));
        self.store.ts_one_set(key, &value);
        let _ = shandle;
        drop(handle);
        value
    }

    fn ts_gen_new(
        &'a self,
        key: &'a <Self as ThreadSafeGenCacheStore<'a>>::Key,
        args: Self::Args,
    ) -> <Self as ThreadSafeGenCacheStore<'a>>::Value {
        let value = self.ts_gen(key, args);
        self.store.ts_one_set(key, &value);
        value
    }
}
