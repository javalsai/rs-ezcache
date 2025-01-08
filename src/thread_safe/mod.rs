//! Thread safe traits around implementations of all possible cache store types. They are analogous
//! to the default traits at the root of this crate but each method has `ts_` prepended (Thread
//! Safe), this allows the thread safe implementations to implement the thread unsafe methods too.
//!
//! There are a few lifetimes and generics involved in this, so it might be confusing and there was
//! a lot of bodging trying to get it to work, so it's very likely that there's something that
//! doesn't quite work.
//!
//! There are two classifications of a thread-safe cache store:
//! - **Smart:** Means they perform smart logic to allow as much concurrency as possible
//! - **Dumb:** Means they aren't concurrent at all, the simplest example is a wrapper that takes a
//!   [`CacheStore`] and locks it completely for each call, even if they should not interfere. At
//!   most, it could implement a [`RwLock`][std::sync::RwLock] to allow concurrent reads.
//!
//! To understand this better here are some cases in which a smart store would allow concurrency, a
//! dumb store would block until each thread is done for these examples:
//! - `ThreadA` reads `A`, `ThreadB` reads `B`: A smart store allows both reads to be concurrent. A
//!   dumb store with a [`RwLock`][std::sync::RwLock] *could* alloc concurrency in this case.
//! - `ThreadA` writes `A`, `ThreadB` writes `B`: A smart store also allows both reads to be
//!   concurrent.
//! - `ThreadA` and ThreadB write to `A`: The smart store would block until `ThreadA` is done to
//!   allow `ThreadB` to write to it.
//!
//! Due to this, a smart thread safe store can become a normal [`CacheStore`], and a [`CacheStore`]
//! can become a dumb thread safe cache. But there's no way to go back, as they "lose" information
//! on how to handle the store concurrently through these conversions.
//!
//! # Error Handling
//!
//! Note that there are not any unfallible cache stores implemented. This is because all thread
//! safe implementations should work internally through mutexes that when locked, can fail due to a
//! [`PoisonError`]. The unfallible trait is still there in case you want to implement it yourself
//! through panicking in an error variant or something. It's **HIGHLY** discouraged as
//! [`PoisonError`]s come precisely by panicking on the thread holding the lock, but you decide on
//! what to do with this after all. For this reason, there's no default wrapper around it and is
//! not exported in the prelude.
//!
//! ## Tips
//! If you want to wrap a [`CacheStore`], they automatically implement [`TryCacheStore`]. Such
//! store will only fail on poison errors, returning a [`PoisonError`] with the store lock. You'll
//! probably want to use a [`TryCacheStoreErrorMap`] to map errors  into any kind of error that
//! implements [`From<PoisonError<…>>`][From].
//!
//! If you want to wrap a [`TryCacheStore`], make sure that the error type implements
//! [`From<PoisonError<…>>`][From] for [`PoisonError`]s.

pub mod generative;

use crate::__internal_prelude::*;

use core::ops::Deref;
use std::sync::PoisonError;

/// Trait for a thread safe infallible cache store, analogous to [CacheStore]
#[delegatable_trait]
pub trait ThreadSafeCacheStore<'lock>
where
    Self: 'lock,
{
    type Key;
    type Value;
    /// Shared lock over a key, must be possible to make one by borrowing a exclusive lock.
    type SLock<'guard>: From<&'guard Self::XLock>
    where
        'lock: 'guard;
    /// Exclusive lock over a wey.
    type XLock: 'lock;

    /// Returns an option of the owned cache element if present.
    fn ts_get(&'lock self, handle: &Self::SLock<'_>) -> Option<Self::Value>;
    /// Sets a value given its key.
    fn ts_set(&'lock self, handle: &mut Self::XLock, value: &Self::Value);
    /// Checks if the cache entry exists.
    fn ts_exists(&'lock self, handle: &Self::SLock<'_>) -> bool {
        self.ts_get(handle).is_some()
    }

    /// Same as `ts_get` but it performs a one-time lock
    fn ts_one_get(&'lock self, key: &Self::Key) -> Option<Self::Value> {
        let handle = self.ts_slock(key);
        self.ts_get(&handle)
    }
    /// Same as `ts_set` but it performs a one-time lock
    fn ts_one_set(&'lock self, key: &Self::Key, value: &Self::Value) {
        let mut handle = self.ts_xlock(key);
        self.ts_set(&mut handle, value)
    }
    /// Same as `ts_exists` but it performs a one-time lock
    fn ts_one_exists(&'lock self, key: &Self::Key) -> bool {
        let handle = self.ts_slock(key);
        self.ts_exists(&handle)
    }

    /// Exclusively lock a key until the handle is dropped.
    fn ts_xlock(&'lock self, key: &Self::Key) -> Self::XLock;
    /// Acquire a shared lock of a key until the handle is dropped.
    fn ts_slock(&'lock self, key: &Self::Key) -> Self::SLock<'lock>;

    /// Exclusively lock a key until the handle is dropped. Non blocking.
    fn ts_xlock_nblock(&'lock self, key: &Self::Key) -> Self::XLock;
    /// Acquire a shared lock of a key until the handle is dropped. Non blocking.
    fn ts_slock_nblock(&'lock self, key: &Self::Key) -> Self::SLock<'lock>;
}

/// Trait for a thread safe fallible cache store, analogous to [ThreadSafeCacheStore]
#[delegatable_trait]
pub trait ThreadSafeTryCacheStore<'lock>
where
    Self: 'lock,
{
    type Key;
    type Value;
    /// Shared lock over a key, must be possible to make one by borrowing a exclusive lock.
    type SLock<'guard>: From<&'guard Self::XLock>
    where
        'lock: 'guard;
    /// Exclusive lock over a wey.
    type XLock: 'lock;

    type Error;

    /// Attempts to return an option of the owned cache element if present.
    fn ts_try_get(
        &'lock self,
        handle: &Self::SLock<'_>,
    ) -> Result<Option<Self::Value>, Self::Error>;
    /// Attempts to set a value given its key.
    fn ts_try_set(
        &'lock self,
        handle: &mut Self::XLock,
        value: &Self::Value,
    ) -> Result<(), Self::Error>;
    /// Attempts to check if the cache key entry exists.
    fn ts_try_exists(&'lock self, handle: &Self::SLock<'_>) -> Result<bool, Self::Error> {
        self.ts_try_get(handle).map(|v| v.is_some())
    }

    /// Same as `ts_get` but it performs a one-time lock
    fn ts_one_try_get(
        &'lock self,
        key: &'lock Self::Key,
    ) -> Result<Option<Self::Value>, Self::Error> {
        let handle = self.ts_try_slock(key)?;
        self.ts_try_get(&handle)
    }
    /// Same as `ts_set` but it performs a one-time lock
    fn ts_one_try_set(
        &'lock self,
        key: &'lock Self::Key,
        value: &Self::Value,
    ) -> Result<(), Self::Error> {
        let mut handle = self.ts_try_xlock(key)?;
        self.ts_try_set(&mut handle, value)
    }
    /// Same as `ts_exists` but it performs a one-time lock
    fn ts_one_try_exists(&'lock self, key: &'lock Self::Key) -> Result<bool, Self::Error> {
        let handle = self.ts_try_slock(key)?;
        self.ts_try_exists(&handle)
    }

    /// Attempt to exclusively lock a key until the handle is dropped.
    fn ts_try_xlock(&'lock self, key: &'lock Self::Key) -> Result<Self::XLock, Self::Error>;
    /// Attempt to acquire a shared lock of a key until the handle is dropped.
    fn ts_try_slock(&'lock self, key: &'lock Self::Key) -> Result<Self::SLock<'lock>, Self::Error>;

    /// Attempt to exclusively lock a key until the handle is dropped. Non block.
    fn ts_try_xlock_nblock(&'lock self, key: &'lock Self::Key) -> Result<Self::XLock, Self::Error>;
    /// Attempt to acquire a shared lock of a key until the handle is dropped. Non block.
    fn ts_try_slock_nblock(
        &'lock self,
        key: &'lock Self::Key,
    ) -> Result<Self::SLock<'lock>, Self::Error>;
}

/// Blanket implementation to allow a [`ThreadSafeCacheStore`] to behave as a
/// [`ThreadSafeTryCacheStore`]
impl<
        'lock,
        K,
        V,
        SL: for<'b> From<&'b XL> + 'lock,
        XL: 'lock,
        T: ThreadSafeCacheStore<'lock, Key = K, Value = V, SLock<'lock> = SL, XLock = XL>,
    > ThreadSafeTryCacheStore<'lock> for T
{
    type Key = K;
    type Value = V;
    type SLock<'guard>
        = SL
    where
        'lock: 'guard;
    type XLock = XL;
    type Error = ();

    fn ts_try_get(
        &'lock self,
        handle: &Self::SLock<'lock>,
    ) -> Result<Option<Self::Value>, Self::Error> {
        Ok(self.ts_get(handle))
    }

    fn ts_try_set(
        &'lock self,
        handle: &mut Self::XLock,
        value: &Self::Value,
    ) -> Result<(), Self::Error> {
        #[allow(clippy::unit_arg)]
        Ok(self.ts_set(handle, value))
    }

    fn ts_try_exists(&'lock self, handle: &Self::SLock<'lock>) -> Result<bool, Self::Error> {
        Ok(self.ts_exists(handle))
    }

    fn ts_try_slock(&'lock self, key: &'lock Self::Key) -> Result<Self::SLock<'lock>, Self::Error> {
        Ok(self.ts_slock(key))
    }

    fn ts_try_xlock(&'lock self, key: &'lock Self::Key) -> Result<Self::XLock, Self::Error> {
        Ok(self.ts_xlock(key))
    }

    fn ts_try_slock_nblock(
        &'lock self,
        key: &'lock Self::Key,
    ) -> Result<Self::SLock<'lock>, Self::Error> {
        Ok(self.ts_slock_nblock(key))
    }

    fn ts_try_xlock_nblock(&'lock self, key: &'lock Self::Key) -> Result<Self::XLock, Self::Error> {
        Ok(self.ts_xlock_nblock(key))
    }
}

// /// Blanket implementation to allow a [`ThreadSafeCacheStore`] to behave as a [`CacheStore`]
// impl<K, V, T: ThreadSafeCacheStore<Key = K, Value = V>> CacheStore for T {
//     type Key = K;
//     type Value = V;

//     fn get(&self, key: &Self::Key) -> Option<Self::Value> {
//         self.ts_get(key)
//     }

//     fn set(&mut self, key: &Self::Key, value: &Self::Value) {
//         self.ts_set(key, value)
//     }

//     fn exists(&self, key: &Self::Key) -> bool {
//         self.ts_exists(key)
//     }
// }

/// Macro to automatically implement [`CacheStore`] on a struct that implements [`ThreadSafeCacheStore`]
#[macro_export]
macro_rules! implThreadUnsafe {
    ($for:expr) => {
        impl<K, V> CacheStore for $for {
            type Key = K;
            type Value = V;

            fn get(&self, key: &Self::Key) -> Option<Self::Value> {
                self.ts_one_get(key)
            }

            fn set(&mut self, key: &Self::Key, value: &Self::Value) {
                self.ts_one_set(key, value)
            }

            fn exists(&self, key: &Self::Key) -> bool {
                self.ts_one_exists(key)
            }
        }
    };
}
pub use implThreadUnsafe;

// /// Blanket implementation to allow a [`ThreadSafeTryCacheStore`] to behave as a [`TryCacheStore`]
// impl<
//         K,
//         V,
//         L,
//         E,
//         T: for<'a> ThreadSafeTryCacheStore<'a, Key = K, Value = V, LockedItem = L, Error = E>,
//     > TryCacheStore for T
// {
//     type Key = K;
//     type Value = V;
//     type Error = E;

//     fn try_get(&self, key: &Self::Key) -> Result<Option<Self::Value>, Self::Error> {
//         self.ts_try_get(key)
//     }

//     fn try_set(&mut self, key: &Self::Key, value: &Self::Value) -> Result<(), Self::Error> {
//         self.ts_try_set(key, value)
//     }

//     fn try_exists(&self, key: &Self::Key) -> Result<bool, Self::Error> {
//         self.ts_try_exists(key)
//     }
// }

/// Macro to automatically implement [`TryCacheStore`] on a struct that implements
/// [`ThreadSafeTryCacheStore`]
#[macro_export]
macro_rules! implTryThreadUnsafe {
    ($for:ty, $( $t:tt $( : $tb:ident)? ),*) => {
        impl<$($t $( : $tb)?),*> TryCacheStore for $for
            {
            type Key = K;
            type Value = V;
            type Error = E;

            fn try_get(&self, key: &Self::Key) -> Result<Option<Self::Value>, Self::Error> {
                self.ts_one_try_get(key)
            }

            fn try_set(&mut self, key: &Self::Key, value: &Self::Value) -> Result<(), Self::Error> {
                self.ts_one_try_set(key, value)
            }

            fn try_exists(&self, key: &Self::Key) -> Result<bool, Self::Error> {
                self.ts_one_try_exists(key)
            }
        }
    };
}
pub use implTryThreadUnsafe;

// wtf tho 😭
// pub fn lol<'b, L, E: for<'a> From<PoisonError<MutexGuard<'a, L>>>>(
//     that: E,
// ) -> PoisonError<MutexGuard<'b, L>>
// where
//     for<'a> PoisonError<MutexGuard<'a, L>>: From<E>,
// {
//     that.into()
// }

pub mod dumb_wrappers {
    use core::{convert::Infallible, marker::PhantomData};
    use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError};

    use super::*;

    #[derive(Debug)]
    /// Empty struct to represent [`PoisonErrors`][std::sync::PoisonError]s without actually
    /// holding a guard.
    pub enum EmptyDumbError {
        Poisoned,
        WouldBlock,
    }
    impl From<Infallible> for EmptyDumbError {
        fn from(_: Infallible) -> Self {
            unreachable!()
        }
    }
    impl<T> From<PoisonError<T>> for EmptyDumbError {
        fn from(_: PoisonError<T>) -> Self {
            Self::Poisoned
        }
    }
    impl<T> From<TryLockError<T>> for EmptyDumbError {
        fn from(value: TryLockError<T>) -> Self {
            match value {
                TryLockError::Poisoned(_) => Self::Poisoned,
                TryLockError::WouldBlock => Self::WouldBlock,
            }
        }
    }

    // pub fn aaaaaa<
    //     K,
    //     V,
    //     E: for<'a> From<PoisonError<RwLockReadGuard<'a, S>>>
    //         + for<'a> From<PoisonError<RwLockWriteGuard<'a, S>>>,
    //     S: TryCacheStore<Key = K, Value = V, Error = E> + 'static,
    // >(
    //     key: &K,
    //     dcs: DumbTryThreadSafeWrapper<K, V, E, S>,
    // ) -> Result<(), E> {
    //     let xlock = dcs.ts_try_xlock(key)?;
    //     drop(xlock);
    //     drop(dcs);

    //     Ok(())
    // }

    /// A thread safe wrapper around a normal non-thread safe [`TryCacheStore`]
    pub struct DumbTryThreadSafeWrapper<
        'a,
        K,
        V,
        E,
        S: TryCacheStore<Key = K, Value = V, Error = E>,
    > {
        pub store: RwLock<S>,
        __phantom: PhantomData<&'a ()>,
    }
    // implTryThreadUnsafe!(DumbTryThreadSafeWrapper<K, V, E, S>, K, V, E, S: TryCacheStore<>);
    // impl<K, V, E, S: TryCacheStore<Key = K, Value = V, Error = E>> crate::TryCacheStore
    //     for DumbTryThreadSafeWrapper<K, V, E, S>
    // {
    //     type Key = K;
    //     type Value = V;
    //     type Error = E;
    // }

    impl<K, V, E, S: TryCacheStore<Key = K, Value = V, Error = E>>
        DumbTryThreadSafeWrapper<'_, K, V, E, S>
    {
        pub fn new(store: S) -> Self {
            Self {
                store: RwLock::new(store),
                __phantom: PhantomData,
            }
        }
    }

    /// Generic enum for a shared key, can hold a [`RwLockWriteGuard`] or [`RwLockReadGuard`] as
    /// both should be possible to be used for shared access, along with the key accessed itself.
    /// Hacky solution for the [`DumbTryThreadSafeWrapper`].
    pub enum RwLockAnyGuardKey<'lock, 'guard, T, K> {
        Read((RwLockReadGuard<'lock, T>, &'lock K)),
        Write(&'guard (RwLockWriteGuard<'lock, T>, &'lock K)),
    }

    impl<'lock, T, K> RwLockAnyGuardKey<'lock, '_, T, K> {
        pub fn get_key(&self) -> &'lock K {
            match self {
                Self::Read((_, k)) => k,
                Self::Write((_, k)) => k,
            }
        }
    }

    impl<'lock, T, K> From<(RwLockReadGuard<'lock, T>, &'lock K)>
        for RwLockAnyGuardKey<'lock, '_, T, K>
    {
        fn from(value: (RwLockReadGuard<'lock, T>, &'lock K)) -> Self {
            Self::Read(value)
        }
    }

    impl<'lock, 'guard, T, K> From<&'guard (RwLockWriteGuard<'lock, T>, &'lock K)>
        for RwLockAnyGuardKey<'lock, 'guard, T, K>
    {
        fn from(value: &'guard (RwLockWriteGuard<'lock, T>, &'lock K)) -> Self {
            Self::Write(value)
        }
    }

    impl<T, K> Deref for RwLockAnyGuardKey<'_, '_, T, K> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            match self {
                Self::Read((l, _)) => l,
                Self::Write((l, _)) => l,
            }
        }
    }

    impl<'lock, K, V, E, S> ThreadSafeTryCacheStore<'lock>
        for DumbTryThreadSafeWrapper<'lock, K, V, E, S>
    where
        Self: 'lock,
        S: TryCacheStore<Key = K, Value = V, Error = E> + 'lock,
        E: From<PoisonError<RwLockReadGuard<'lock, S>>>
            + From<PoisonError<RwLockWriteGuard<'lock, S>>>
            + From<TryLockError<RwLockReadGuard<'lock, S>>>
            + From<TryLockError<RwLockWriteGuard<'lock, S>>>,
    {
        type Key = K;
        type Value = V;
        type SLock<'guard>
            = RwLockAnyGuardKey<'lock, 'guard, S, Self::Key>
        where
            'lock: 'guard;
        type XLock = (RwLockWriteGuard<'lock, S>, &'lock Self::Key);
        type Error = E;

        fn ts_try_get(&self, handle: &Self::SLock<'_>) -> Result<Option<Self::Value>, Self::Error> {
            handle.try_get(handle.get_key())
        }

        fn ts_try_set(
            &self,
            handle: &mut Self::XLock,
            value: &Self::Value,
        ) -> Result<(), Self::Error> {
            handle.0.try_set(handle.1, value)
        }

        fn ts_try_exists(&self, handle: &Self::SLock<'_>) -> Result<bool, Self::Error> {
            handle.try_exists(handle.get_key())
        }

        fn ts_try_slock(
            &'lock self,
            key: &'lock Self::Key,
        ) -> Result<Self::SLock<'lock>, Self::Error> {
            Ok((self.store.read()?, key).into())
        }

        fn ts_try_xlock(&'lock self, key: &'lock Self::Key) -> Result<Self::XLock, Self::Error> {
            Ok((self.store.write()?, key))
        }

        fn ts_try_slock_nblock(
            &'lock self,
            key: &'lock Self::Key,
        ) -> Result<Self::SLock<'lock>, Self::Error> {
            Ok((self.store.try_read()?, key).into())
        }

        fn ts_try_xlock_nblock(
            &'lock self,
            key: &'lock Self::Key,
        ) -> Result<Self::XLock, Self::Error> {
            Ok((self.store.try_write()?, key))
        }
    }
}
