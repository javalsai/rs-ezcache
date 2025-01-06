//! Dumb thread safe wrappers around implementations of all possible cache store types.
//!
//! Dumb in this means they don't perform any smart logic, when they are used they lock all the
//! store, such that if two different keys are being accessed at the same time, even though they
//! shouldn't interfere, they will block the store until the other finishes. Due to this, a normal
//! [`CacheStore`] can become dumb thread safe.
//!
//! # Error Handling
//!
//! Note that there are not many unfallible cache stores implemented. This is because most of the
//! thread safe implementations work through internally mutexes that when locked, can fail due to a
//! [`std::sync::PoisonError`].
//!
//! If you want to wrap a [`CacheStore`], use a [`CacheStoreAdapter`] to treat it as as a
//! [`TryCacheStore`]. Such store will only fail on poison errors, returning `()`.
//!
//! If you want to wrap a [`TryCacheStore`], make sure that the error type implements
//! [`From<PoisonError>`][From]. [`PoisonError`] is an empty struct that represents such poison
//! errors, but is not the `std` error type as it contains a [`MutexGuard`][std::sync::MutexGuard]
//! that can't be passed around.

// pub mod generative;

use std::{
    marker::PhantomData,
    sync::{Arc, LockResult, Mutex, MutexGuard, PoisonError},
};

use crate::prelude::*;

/// Wrapper around a [TryCacheStore] to make it thread safe.
///
/// Generics:
/// - `K`: Type of the key used for cache indexing.
/// - `V`: Type of the value stored in the cache store.
/// - `E`: The error type used on failures.
/// - `S`: [`TryCacheStore<K, V, E>`] which this wraps around.
pub struct ThreadSafeTryCacheStore<K, V, E, S: TryCacheStore<K, V, E>> {
    pub store: Arc<Mutex<S>>,
    phantom: PhantomData<(K, V, E)>,
}

/// Default implementations.
impl<K, V, E, S: TryCacheStore<K, V, E>> ThreadSafeTryCacheStore<K, V, E, S> {
    /// Makes a [ThreadSafeTryCacheStore] from a [TryCacheStore]
    pub fn from_try_store(store: S) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            phantom: PhantomData,
        }
    }

    pub fn lock(&self) -> LockResult<MutexGuard<S>> {
        self.store.lock()
    }
}

/// Reimplementation of [TryCacheStore] methods with non-mutable self references via mutex lock for thread safety.
impl<K, V, E: for<'a> From<PoisonError<MutexGuard<'a, S>>>, S: TryCacheStore<K, V, E>>
    ThreadSafeTryCacheStore<K, V, E, S>
{
    /// Attempts to return an option of the owned cache element if present.
    pub fn try_get(&self, key: &K) -> Result<Option<V>, E> {
        self.lock()?.try_get(key)
    }

    /// Attempts to set a value given its key.
    pub fn try_set(&self, key: &K, value: &V) -> Result<(), E> {
        self.lock()?.try_set(key, value)
    }

    /// Attempts to check if the cache key entry exists.
    pub fn try_exists(&self, key: &K) -> Result<bool, E> {
        self.lock()?.try_exists(key)
    }
}

// This was useful when [CacheStore]s implemented [TryCacheStore], now that an adapter is needed
// this no longer makes sense. Also, if we want to explicitly return a PoisonError with no variants,
// we might as well just return `()`, which the adapter method does

// impl<K, V, S: TryCacheStore<K, V, ()>> ThreadSafeTryCacheStore<K, V, (), S>
// where
//     S: CacheStore<K, V>,
// {
//     pub fn get(&self, key: &K) -> Result<Option<V>, PoisonError> {
//         let store_lock = self.store.lock().map_err(|_| PoisonError)?;
//         Ok(store_lock.get(key))
//     }

//     pub fn set(&self, key: &K, value: &V) -> Result<(), PoisonError> {
//         let mut store_lock = self.store.lock().map_err(|_| PoisonError)?;
//         store_lock.set(key, value);
//         Ok(())
//     }

//     pub fn exists(&self, key: &K) -> Result<bool, PoisonError> {
//         let store_lock = self.store.lock().map_err(|_| PoisonError)?;
//         Ok(store_lock.exists(key))
//     }
// }
