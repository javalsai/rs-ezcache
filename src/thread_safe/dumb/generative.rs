//! Thread safe abstractions for [generative][crate::generative] stores.
//!
//! Check [thread_safe][super] for more info on what this means.

use std::{
    marker::PhantomData,
    sync::{Arc, LockResult, Mutex, MutexGuard},
};

use super::PoisonError;
use crate::prelude::*;

/// Wrapper around a [TryCacheStore] to make it thread safe.
///
/// Generics:
/// - `K`: Type of the key used for cache indexing.
/// - `V`: Type of the value stored in the cache store.
/// - `E`: The error type used on failures.
/// - `S`: [`TryCacheStore<K, V, E>`] which this wraps around.
/// - `F`: [`Fn<&K, A>`] with  `V` return generator function.
/// - `A`: Type of additional arguments of the generator function.
pub struct ThreadSafeTryCacheStore<
    K,
    V,
    E,
    S: TryCacheStore<K, V, E>,
    F: Fn(&K, A) -> Result<V, E>,
    A,
> {
    /// Mutex of the underlying store for manually locking it
    #[allow(clippy::type_complexity)]
    pub store: Arc<Mutex<TryGenCacheStore<K, V, E, S, F, A>>>,
    phantom: PhantomData<(K, V, E)>,
}

/// Default implementations.
impl<K, V, E, S: TryCacheStore<K, V, E>, F: Fn(&K, A) -> Result<V, E>, A>
    ThreadSafeTryCacheStore<K, V, E, S, F, A>
{
    /// Makes a [ThreadSafeTryCacheStore] from a [TryCacheStore]
    pub fn from_try_gen_store(store: TryGenCacheStore<K, V, E, S, F, A>) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            phantom: PhantomData,
        }
    }

    pub fn lock(&self) -> LockResult<MutexGuard<TryGenCacheStore<K, V, E, S, F, A>>> {
        self.store.lock()
    }
}

/// Reimplementation of [TryGenCacheStore] specific methods with non-mutable self references via
/// mutex lock for thread safety.
impl<
        K,
        V,
        E: for<'a> From<PoisonError<MutexGuard<'a, TryGenCacheStore<K, V, E, S, F, A>>>>,
        S: TryCacheStore<K, V, E>,
        F: Fn(&K, A) -> Result<V, E>,
        A,
    > ThreadSafeTryCacheStore<K, V, E, S, F, A>
{
    /// Attempt to generate a new value without checking cache or adding the value to it.
    pub fn try_gen(&self, key: &K, args: A) -> Result<V, E> {
        self.lock()?.try_gen(key, args)
    }

    /// Attempt to get the value from cache or generate a new one without adding it.
    pub fn try_get_or_gen(&self, key: &K, args: A) -> Result<V, E> {
        let store_lock = self.store.lock().map_err(|_| PoisonError)?;
        store_lock.try_get_or_gen(key, args)
    }

    /// Attempt to get the value from cache or generate a new one attempting to add it.
    pub fn try_get_or_new(&self, key: &K, args: A) -> Result<V, E> {
        let store_lock = self.store.lock().map_err(|_| PoisonError)?;
        store_lock.try_get_or_gen(key, args)
    }

    /// Attempt to generate a new value without checking cache and attempting to add the value to
    /// it, possibly overwriting previous values.
    pub fn try_gen_new(&mut self, key: &K, args: A) -> Result<V, E> {
        let mut store_lock = self.store.lock().map_err(|_| PoisonError)?;
        store_lock.try_gen_new(key, args)
    }
}

/// Reimplement inner store fns for easy access
///
/// Reimplementation of [TryGenCacheStore] methods (which themselves come from [TryCacheStore])
/// with non-mutable self references via mutex lock for thread safety.
impl<K, V, E: From<PoisonError>, S: TryCacheStore<K, V, E>, F: Fn(&K, A) -> Result<V, E>, A>
    ThreadSafeTryCacheStore<K, V, E, S, F, A>
{
    /// Attempts to return an option of the owned cache element if present.
    pub fn try_get(&self, key: &K) -> Result<Option<V>, E> {
        let store_lock = self.store.lock().map_err(|_| PoisonError)?;
        store_lock.try_get(key)
    }

    /// Attempts to set a value given its key.
    pub fn try_set(&self, key: &K, value: &V) -> Result<(), E> {
        let mut store_lock = self.store.lock().map_err(|_| PoisonError)?;
        store_lock.try_set(key, value)
    }

    /// Attempts to check if the cache key entry exists.
    pub fn try_exists(&self, key: &K) -> Result<bool, E> {
        let store_lock = self.store.lock().map_err(|_| PoisonError)?;
        store_lock.try_exists(key)
    }
}
