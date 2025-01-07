//! Several implementations of cache stores for common use cases, all of require std for now:
//! - [`MemoryStore`]: So just HashMap cool wrapping around. You'll see it most for examples.
//! - [`ThreadSafeMemoryStore`]: Concurrent store in memory. Uses unsafe under the hood but should
//!   be optimized enough.
//!
//! # Examples
//!
//! ```rust
//! use ezcache::{CacheStore, stores::MemoryStore};
//!
//! let mut store: MemoryStore<&'static str, String> = MemoryStore::default();
//!
//! let value = store.get("key");
//! assert_eq!(value, None);
//!
//! store.set("key", &("value".to_owned()));
//! let value = store.get("key");
//! assert_eq!(value, Some(String::from("value")));
//! ```
//!
//! ```rust
//! # use std::{thread, sync::Arc};
//! use ezcache::{
//!     TryCacheStore,
//!     TryCacheStoreErrorMap,
//!     stores::MemoryStore,
//!     thread_safe::{
//!         ThreadSafeTryCacheStore,
//!         dumb_wrappers::{
//!             DumbTryThreadSafeWrapper,
//!             EmptyDumbError,
//!         },
//!     },
//! };
//!
//! let memory_store: MemoryStore<(), String> = MemoryStore::default();
//! let try_store: TryCacheStoreErrorMap<_, _, _, EmptyDumbError, _> =
//!     memory_store.into();
//! let store = DumbTryThreadSafeWrapper::new(try_store);
//!
//! let store = Arc::new(store);
//! let store_clone = Arc::clone(&store);
//!
//! thread::spawn(move || {
//!     store_clone.ts_one_try_set(&(), &String::from("value in thread"))
//! }).join().unwrap();
//!
//! let value = store.ts_one_try_get(&()).unwrap();
//! assert_eq!(value, Some(String::from("value in thread")));
//! ```

use crate::{__internal_prelude::*, thread_safe::dumb_wrappers::EmptyDumbError};

use core::{borrow::Borrow, hash::Hash, ops::Deref};
use std::{
    collections::HashMap,
    sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

#[derive(Default)]
/// Simple thread unsafe in memory cache store.
pub struct MemoryStore<K, V> {
    cache: HashMap<K, V>,
}

impl<K, V> MemoryStore<K, V> {
    pub fn new(hashmap: HashMap<K, V>) -> Self {
        Self { cache: hashmap }
    }
}

impl<K: Hash + Eq + Sized + Clone, V: Clone> CacheStore for MemoryStore<K, V> {
    type Key = K;
    type Value = V;

    fn get(&self, key: impl Borrow<Self::Key>) -> Option<Self::Value> {
        self.cache.get(key.borrow()).cloned()
    }

    fn set(&mut self, key: impl Borrow<Self::Key>, value: impl Borrow<Self::Value>) {
        self.cache
            .insert(key.borrow().clone(), value.borrow().clone());
    }

    fn exists(&self, key: impl Borrow<Self::Key>) -> bool {
        self.cache.contains_key(key.borrow())
    }
}

/// Wrapper around a [`RwLockReadGuard`] and a [`RwLockWriteGuard`] to allow any to be used.
pub enum RwLockAnyGuard<'lock, 'guard, T> {
    Read(RwLockReadGuard<'lock, T>),
    Write(&'guard RwLockWriteGuard<'lock, T>),
}

impl<'lock, T> From<RwLockReadGuard<'lock, T>> for RwLockAnyGuard<'lock, '_, T> {
    fn from(value: RwLockReadGuard<'lock, T>) -> Self {
        Self::Read(value)
    }
}

impl<'lock, 'guard, T> From<&'guard RwLockWriteGuard<'lock, T>>
    for RwLockAnyGuard<'lock, 'guard, T>
{
    fn from(value: &'guard RwLockWriteGuard<'lock, T>) -> Self {
        Self::Write(value)
    }
}

impl<T> Deref for RwLockAnyGuard<'_, '_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Read(l) => l,
            Self::Write(l) => l,
        }
    }
}

/// This struct is unsafe under the hood, so you must be careful when using it. No professional
/// reviewed the unsafe usage and the safe code to do this would be too complex for me.
///
/// All unsafe usage is mainly to detach inner locks from the hashmap lock itself tho, so as long
/// as the hashmap itself doesn't move the value or the entry gets deleted, nothing should happen,
/// and I think both can't happen at least now.
#[derive(Default)]
pub struct ThreadSafeMemoryStore<K, V> {
    cache: Mutex<HashMap<K, RwLock<Option<V>>>>,
}

impl<K: Hash + Eq, V> ThreadSafeMemoryStore<K, V> {
    pub fn new(cache: HashMap<K, V>) -> Self {
        Self {
            cache: Mutex::new(
                cache
                    .into_iter()
                    .map(|(k, v)| (k, RwLock::new(Some(v))))
                    .collect(),
            ),
        }
    }
}

impl<'lock, K: Hash + Eq + Sized + Clone, V: Clone> ThreadSafeTryCacheStore<'lock>
    for ThreadSafeMemoryStore<K, V>
where
    Self: 'lock,
{
    type Key = K;
    type Value = V;
    type Error = EmptyDumbError;
    type SLock<'guard>
        = RwLockAnyGuard<'lock, 'guard, Option<V>>
    where
        'lock: 'guard;
    type XLock = RwLockWriteGuard<'lock, Option<V>>;

    fn ts_try_get(
        &'lock self,
        handle: &Self::SLock<'_>,
    ) -> Result<Option<Self::Value>, Self::Error> {
        Ok((*handle).clone())
    }

    fn ts_try_set(
        &'lock self,
        handle: &mut Self::XLock,
        value: &Self::Value,
    ) -> Result<(), Self::Error> {
        **handle = Some(value.clone());
        Ok(())
    }

    fn ts_try_exists(&'lock self, handle: &Self::SLock<'_>) -> Result<bool, Self::Error> {
        Ok((*handle).is_some())
    }

    fn ts_try_xlock(&'lock self, key: &'lock Self::Key) -> Result<Self::XLock, Self::Error> {
        let mut cache_lock = self.cache.lock()?;
        let value = match cache_lock.get(key) {
            Some(thing) => thing,
            None => {
                cache_lock.insert(key.clone(), Default::default());
                cache_lock.get(key).unwrap()
            }
        };

        // Detach the lock itself from the HashMap guard lifetime
        let value: *const _ = value;
        let lock: Self::XLock = unsafe { (*value).write()? };
        drop(cache_lock);

        Ok(lock)
    }

    fn ts_try_slock(&'lock self, key: &'lock Self::Key) -> Result<Self::SLock<'lock>, Self::Error> {
        let mut cache_lock = self.cache.lock()?;
        let value = match cache_lock.get(key) {
            Some(thing) => thing,
            None => {
                cache_lock.insert(key.clone(), Default::default());
                cache_lock.get(key).unwrap()
            }
        };

        // Detach the lock itself from the HashMap guard lifetime
        let value: *const _ = value;
        let lock: Self::SLock<'_> = unsafe { (*value).read()?.into() };
        drop(cache_lock);

        Ok(lock)
    }

    fn ts_try_xlock_nblock(&'lock self, key: &'lock Self::Key) -> Result<Self::XLock, Self::Error> {
        let mut cache_lock = self.cache.lock()?;
        let value = match cache_lock.get(key) {
            Some(thing) => thing,
            None => {
                cache_lock.insert(key.clone(), Default::default());
                cache_lock.get(key).unwrap()
            }
        };

        // Detach the lock itself from the HashMap guard lifetime
        let value: *const _ = value;
        let lock: Self::XLock = unsafe { (*value).try_write()? };
        drop(cache_lock);

        Ok(lock)
    }

    fn ts_try_slock_nblock(
        &'lock self,
        key: &'lock Self::Key,
    ) -> Result<Self::SLock<'lock>, Self::Error> {
        let mut cache_lock = self.cache.lock()?;
        let value = match cache_lock.get(key) {
            Some(thing) => thing,
            None => {
                cache_lock.insert(key.clone(), Default::default());
                cache_lock.get(key).unwrap()
            }
        };

        // Detach the lock itself from the HashMap guard lifetime
        let value: *const _ = value;
        let lock: Self::SLock<'_> = unsafe { (*value).try_read()?.into() };
        drop(cache_lock);

        Ok(lock)
    }
}

#[cfg(test)]
mod test {
    use super::{ThreadSafeMemoryStore, ThreadSafeTryCacheStore};

    #[test]
    fn xlock_2_diff_keys() {
        let store = ThreadSafeMemoryStore::<usize, usize>::default();

        let x1 = store.ts_try_xlock_nblock(&0).expect("to lock first key");
        let x2 = store.ts_try_xlock_nblock(&1).expect("to lock second key");
        drop((x1, x2));
    }

    #[test]
    fn xlock_2_same_keys() {
        let store = ThreadSafeMemoryStore::<usize, usize>::default();

        let x1 = store.ts_try_xlock_nblock(&0).expect("to lock first key");
        let x2 = store
            .ts_try_xlock_nblock(&0)
            .expect_err("to not lock second key");
        drop((x1, x2));
    }
}
