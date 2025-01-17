//! Thread safe traits for generative cache stores.

use core::marker::PhantomData;

use crate::__internal_prelude::*;

use super::ThreadSafeCacheStore;

/// Infalible thread safe generative cache store. This trait is **HIGHLY** discouraged for the
/// reasons explained in [`thread_safe`][crate::thread_safe]
#[delegatable_trait]
pub trait ThreadSafeGenCacheStore<'lock>:
    super::ThreadSafeCacheStore<
    'lock,
    Key = <Self as ThreadSafeGenCacheStore<'lock>>::Key,
    Value = <Self as ThreadSafeGenCacheStore<'lock>>::Value,
>
where
    Self: 'lock,
{
    type Key;
    type Value;
    type Args;

    /// Generate a new value without checking cache or adding the value to it.
    fn ts_gen(
        &'lock self,
        key: &'lock <Self as ThreadSafeGenCacheStore<'lock>>::Key,
        args: Self::Args,
    ) -> <Self as ThreadSafeGenCacheStore<'lock>>::Value;
    /// Get the value from cache or generate a new one without adding it.
    fn ts_get_or_gen(
        &'lock self,
        key: &'lock <Self as ThreadSafeGenCacheStore<'lock>>::Key,
        args: Self::Args,
    ) -> <Self as ThreadSafeGenCacheStore<'lock>>::Value;
    /// Get the value from cache or generate a new one adding it.
    fn ts_get_or_new(
        &'lock self,
        key: &'lock <Self as ThreadSafeGenCacheStore<'lock>>::Key,
        args: Self::Args,
    ) -> <Self as ThreadSafeGenCacheStore<'lock>>::Value;
    /// Generate a new value without checking cache and add the value to it, possibly overwriting
    /// previous values.
    fn ts_gen_new(
        &'lock self,
        key: &'lock <Self as ThreadSafeGenCacheStore<'lock>>::Key,
        args: Self::Args,
    ) -> <Self as ThreadSafeGenCacheStore<'lock>>::Value;
}

/// Falible thread safe generative cache store.
#[delegatable_trait]
#[allow(clippy::missing_errors_doc)]
pub trait ThreadSafeTryGenCacheStore<'lock>:
    super::ThreadSafeTryCacheStore<
    'lock,
    Key = <Self as ThreadSafeTryGenCacheStore<'lock>>::Key,
    Value = <Self as ThreadSafeTryGenCacheStore<'lock>>::Value,
>
{
    type Key;
    type Value;
    type Error;
    type Args;

    /// Generate a new value without checking cache or adding the value to it.
    fn ts_try_gen(
        &'lock self,
        key: &'lock <Self as ThreadSafeTryGenCacheStore<'lock>>::Key,
        args: Self::Args,
    ) -> Result<
        <Self as ThreadSafeTryGenCacheStore<'lock>>::Value,
        <Self as ThreadSafeTryGenCacheStore<'lock>>::Error,
    >;
    /// Get the value from cache or generate a new one without adding it.
    fn ts_try_get_or_gen(
        &'lock self,
        key: &'lock <Self as ThreadSafeTryGenCacheStore<'lock>>::Key,
        args: Self::Args,
    ) -> Result<
        <Self as ThreadSafeTryGenCacheStore<'lock>>::Value,
        <Self as ThreadSafeTryGenCacheStore<'lock>>::Error,
    >;
    /// Get the value from cache or generate a new one adding it.
    fn ts_try_get_or_new(
        &'lock self,
        key: &'lock <Self as ThreadSafeTryGenCacheStore<'lock>>::Key,
        args: Self::Args,
    ) -> Result<
        <Self as ThreadSafeTryGenCacheStore<'lock>>::Value,
        <Self as ThreadSafeTryGenCacheStore<'lock>>::Error,
    >;
    /// Generate a new value without checking cache and add the value to it, possibly overwriting
    /// previous values.
    fn ts_try_gen_new(
        &'lock self,
        key: &'lock <Self as ThreadSafeTryGenCacheStore<'lock>>::Key,
        args: Self::Args,
    ) -> Result<
        <Self as ThreadSafeTryGenCacheStore<'lock>>::Value,
        <Self as ThreadSafeTryGenCacheStore<'lock>>::Error,
    >;
}

use super::ambassador_impl_ThreadSafeCacheStore;
#[derive(Delegate)]
#[delegate(ThreadSafeCacheStore<'lock>, target = "store")]
/// Infallible thread safe generative cache store wrapper around a [`ThreadSafeCacheStore`]
/// and a generator function.
///
/// One of the few unafallible thread safe wrappers.
///
/// Generics:
/// - `K`: Type of the key used for cache indexing.
/// - `V`: Type of the value stored in the cache store.
/// - `A`: Type of additional arguments of the generator function.
/// - `S`: [`ThreadSafeCacheStore`] which this wraps around.
/// - `F`: [`Fn<&K, A>`] with `V` return generator function.
pub struct ThreadSafeGenCacheStoreWrapper<
    'lock,
    K,
    V,
    A,
    S: super::ThreadSafeCacheStore<'lock, Key = K, Value = V>,
    F: Fn(&K, A) -> V + 'lock,
> {
    pub store: S,
    pub generator: F,
    phantom: PhantomData<&'lock (K, V, A)>,
}

/// Default implementation
impl<
        'lock,
        K,
        V,
        A,
        S: super::ThreadSafeCacheStore<'lock, Key = K, Value = V>,
        F: Fn(&K, A) -> V,
    > ThreadSafeGenCacheStoreWrapper<'lock, K, V, A, S, F>
{
    /// Make a new [`ThreadSafeGenCacheStoreWrapper`] from a
    /// [`ThreadSafeCacheStore`] and a generator function.
    pub fn new(store: S, generator: F) -> Self {
        Self {
            store,
            generator,
            phantom: PhantomData,
        }
    }
}

/// Implement [`ThreadSafeCacheStore`]
impl<
        'lock,
        K,
        V: Clone,
        A,
        S: super::ThreadSafeCacheStore<'lock, Key = K, Value = V>,
        F: Fn(&K, A) -> V,
    > ThreadSafeGenCacheStore<'lock> for ThreadSafeGenCacheStoreWrapper<'lock, K, V, A, S, F>
{
    type Key = K;
    type Value = V;
    type Args = A;

    fn ts_gen(
        &'lock self,
        key: &'lock <Self as ThreadSafeGenCacheStore<'lock>>::Key,
        args: Self::Args,
    ) -> <Self as ThreadSafeGenCacheStore<'lock>>::Value {
        (self.generator)(key, args)
    }

    fn ts_get_or_gen(
        &'lock self,
        key: &'lock <Self as ThreadSafeGenCacheStore<'lock>>::Key,
        args: Self::Args,
    ) -> <Self as ThreadSafeGenCacheStore<'lock>>::Value {
        self.store
            .ts_one_get(key)
            .unwrap_or_else(|| self.ts_gen(key, args))
    }

    fn ts_get_or_new(
        &'lock self,
        key: &'lock <Self as ThreadSafeGenCacheStore<'lock>>::Key,
        args: Self::Args,
    ) -> <Self as ThreadSafeGenCacheStore<'lock>>::Value {
        let mut handle = self.ts_xlock(key);
        let slock: Self::SLock<'_> = (&handle).into();
        let value = self
            .store
            .ts_get(&slock)
            .unwrap_or_else(|| self.ts_gen(key, args));
        drop(slock);
        self.store.ts_set(&mut handle, &value);
        drop(handle);
        value
    }

    fn ts_gen_new(
        &'lock self,
        key: &'lock <Self as ThreadSafeGenCacheStore<'lock>>::Key,
        args: Self::Args,
    ) -> <Self as ThreadSafeGenCacheStore<'lock>>::Value {
        let value = self.ts_gen(key, args);
        self.store.ts_one_set(key, &value);
        value
    }
}

use super::ambassador_impl_ThreadSafeTryCacheStore;
#[derive(Delegate)]
#[delegate(ThreadSafeTryCacheStore<'lock>, target = "store")]
/// Fallible thread safe generative cache store wrapper around a [`ThreadSafeTryCacheStore`]
/// and a generator function.
///
/// Generics:
/// - `K`: Type of the key used for cache indexing.
/// - `V`: Type of the value stored in the cache store.
/// - `E`: Error type.
/// - `A`: Type of additional arguments of the generator function.
/// - `StErr`: Error type of the store.
/// - `FnErr`: Error type of the function.
/// - `S`: [`ThreadSafeCacheStore`] which this wraps around.
/// - `F`: [`Fn<&K, A>`] with `V` return generator function.
pub struct ThreadSafeGenTryCacheStoreWrapper<
    'lock,
    K,
    V,
    E,
    A,
    StErr: Into<E> + 'lock,
    FnErr: Into<E> + 'lock,
    S: super::ThreadSafeTryCacheStore<'lock, Key = K, Value = V, Error = StErr>,
    F: Fn(&K, A) -> Result<V, FnErr> + 'lock,
> {
    pub store: S,
    pub generator: F,
    phantom: PhantomData<&'lock (K, V, A, E)>,
}

/// Default implementation
impl<
        'lock,
        K,
        V,
        E,
        A,
        StErr: Into<E> + 'lock,
        FnErr: Into<E> + 'lock,
        S: super::ThreadSafeTryCacheStore<'lock, Key = K, Value = V, Error = StErr>,
        F: Fn(&K, A) -> Result<V, FnErr>,
    > ThreadSafeGenTryCacheStoreWrapper<'lock, K, V, E, A, StErr, FnErr, S, F>
{
    /// Make a new [`ThreadSafeGenCacheStoreWrapper`] from a [`ThreadSafeCacheStore`] and a generator function.
    pub fn new(store: S, generator: F) -> Self {
        Self {
            store,
            generator,
            phantom: PhantomData,
        }
    }
}

/// Implement [`ThreadSafeCacheStore`]
impl<
        'lock,
        K,
        V: Clone,
        E,
        A,
        StErr: Into<E> + 'lock,
        FnErr: Into<E> + 'lock,
        S: super::ThreadSafeTryCacheStore<'lock, Key = K, Value = V, Error = StErr>,
        F: Fn(&K, A) -> Result<V, FnErr>,
    > ThreadSafeTryGenCacheStore<'lock>
    for ThreadSafeGenTryCacheStoreWrapper<'lock, K, V, E, A, StErr, FnErr, S, F>
{
    type Key = K;
    type Value = V;
    type Args = A;
    type Error = E;

    fn ts_try_gen(
        &self,
        key: &<Self as ThreadSafeTryGenCacheStore<'lock>>::Key,
        args: Self::Args,
    ) -> Result<
        <Self as ThreadSafeTryGenCacheStore<'lock>>::Value,
        <Self as ThreadSafeTryGenCacheStore<'lock>>::Error,
    > {
        (self.generator)(key, args).map_err(Into::into)
    }

    fn ts_try_get_or_gen(
        &'lock self,
        key: &'lock <Self as ThreadSafeTryGenCacheStore<'lock>>::Key,
        args: Self::Args,
    ) -> Result<
        <Self as ThreadSafeTryGenCacheStore<'lock>>::Value,
        <Self as ThreadSafeTryGenCacheStore<'lock>>::Error,
    > {
        self.store
            .ts_one_try_get(key)
            .map_err(Into::into)?
            .map_or_else(move || self.ts_try_gen(key, args), Ok)
    }

    fn ts_try_get_or_new(
        &'lock self,
        key: &'lock <Self as ThreadSafeTryGenCacheStore<'lock>>::Key,
        args: Self::Args,
    ) -> Result<
        <Self as ThreadSafeTryGenCacheStore<'lock>>::Value,
        <Self as ThreadSafeTryGenCacheStore<'lock>>::Error,
    > {
        let mut handle = self.ts_try_xlock(key).map_err(Into::into)?;
        let value = self
            .store
            .ts_try_get(&(&handle).into())
            .map_err(Into::into)?
            .map_or_else(|| self.ts_try_gen(key, args), Ok)?;
        self.store
            .ts_try_set(&mut handle, &value)
            .map_err(Into::into)?;
        drop(handle);
        Ok(value)
    }

    fn ts_try_gen_new(
        &'lock self,
        key: &'lock <Self as ThreadSafeTryGenCacheStore<'lock>>::Key,
        args: Self::Args,
    ) -> Result<
        <Self as ThreadSafeTryGenCacheStore<'lock>>::Value,
        <Self as ThreadSafeTryGenCacheStore<'lock>>::Error,
    > {
        let value = self.ts_try_gen(key, args)?;
        self.store.ts_one_try_set(key, &value).map_err(Into::into)?;
        Ok(value)
    }
}
