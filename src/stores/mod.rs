//! Several implementations of cache stores for common use cases, all of require std for now:
//! - [`MemoryStore`]: So just HashMap cool wrapping around. You'll see it most for examples.

use crate::__internal_prelude::*;

use core::{borrow::Borrow, hash::Hash};
use std::collections::HashMap;

#[derive(Default)]
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

// /// Testing stores
// pub mod __stores {
//     use super::*;
//     use core::hash::Hash;
//     use std::{
//         collections::HashMap,
//         sync::{RwLock, RwLockWriteGuard},
//     };

//     pub struct ThreadSafeMemoryCache<K, V> {
//         store: RwLock<HashMap<K, RwLock<Option<V>>>>,
//     }

//     pub struct MemoryCachePoisonError;
//     impl<T> From<PoisonError<T>> for MemoryCachePoisonError {
//         fn from(_: PoisonError<T>) -> Self {
//             Self
//         }
//     }

//     impl<K: Eq + Hash + ?Sized + Clone, V> ThreadSafeTryCacheStore for ThreadSafeMemoryCache<K, V> {
//         type Key = K;
//         type Value = V;
//         type LockDeref = Option<V>;
//         type Error<G> = MemoryCachePoisonError;

//         fn ts_try_xlock<G>(
//             &self,
//             key: &Self::Key,
//         ) -> Result<impl DerefMut<Target = Self::LockDeref>, Self::Error<G>> {
//             let mut hashmap_guard = self.store.write()?;
//             let x = Ok(hashmap_guard.get(key).unwrap().write()?);
//             x
//             // match hashmap_guard.get(key) {
//             //     Some(v) => {
//             //         let inner_handle = v.write();
//             //         // drop(hashmap_guard);
//             //         Ok(inner_handle?)
//             //     }
//             //     None => {
//             //         let inner_val = RwLock::new(None);
//             //         let inner_lock = inner_val.write();
//             //         hashmap_guard.insert(key.clone(), inner_val);
//             //         todo!()
//             //     }
//             // }
//         }
//     }
// }
