use base64::{prelude::BASE64_URL_SAFE, Engine};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};

use crate::{__internal_prelude::*, thread_safe::dumb_wrappers::RwLockAnyGuardKey};

use core::hash::Hash;
use std::vec;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    string::String,
    sync::{Mutex, PoisonError, RwLock, RwLockWriteGuard, TryLockError},
    vec::Vec,
};

/// Error Type used by the File Based cache store
#[derive(Debug)]
pub enum ThreadSafeFileStoreError {
    Io(std::io::Error),
    Bincode(bincode::Error),
    Poisoned,
    WouldBlock,
}
impl std::error::Error for ThreadSafeFileStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Bincode(err) => Some(err),
            _ => None,
        }
    }
}
impl std::fmt::Display for ThreadSafeFileStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(err) => writeln!(f, "io error: {err}"),
            Self::Bincode(err) => writeln!(f, "bincode error: {err}"),
            Self::Poisoned => writeln!(f, "poisoned lock"),
            Self::WouldBlock => writeln!(f, "locking would block"),
        }
    }
}

impl From<bincode::Error> for ThreadSafeFileStoreError {
    fn from(value: bincode::Error) -> Self {
        Self::Bincode(value)
    }
}
impl From<std::io::Error> for ThreadSafeFileStoreError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl<T> From<PoisonError<T>> for ThreadSafeFileStoreError {
    fn from(_: PoisonError<T>) -> Self {
        Self::Poisoned
    }
}
impl<T> From<TryLockError<T>> for ThreadSafeFileStoreError {
    fn from(value: TryLockError<T>) -> Self {
        match value {
            TryLockError::Poisoned(_) => Self::Poisoned,
            TryLockError::WouldBlock => Self::WouldBlock,
        }
    }
}

/// Custom trait used for filename hashing
pub trait CustomHash {
    fn hash(&self) -> String;
}
impl<T: AsRef<[u8]>> CustomHash for T {
    fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self);
        BASE64_URL_SAFE.encode(hasher.finalize().as_slice())
    }
}

// ---- Raw (No Serialization)

/// Thread safe store based on files
pub struct ThreadSafeFileStore<K, V> {
    path: PathBuf,
    cache: Mutex<HashMap<K, RwLock<()>>>,
    value_phantom: PhantomData<V>,
}

impl<K: CustomHash, V> ThreadSafeFileStore<K, V> {
    /// Makes a new instance from a directory path
    /// Doesn't perform any file lock, you must ensure this path isn't used by other processes
    /// or even this one itself.
    ///
    /// # Errors
    /// Fails when any underlying io call does.
    pub fn new_on(path: impl AsRef<Path> + TryInto<PathBuf>) -> std::io::Result<Self> {
        std::fs::create_dir_all(&path)?;
        Ok(Self {
            path: path.try_into().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::Other, "error converting from path")
            })?,
            cache: Mutex::new(HashMap::new()),
            value_phantom: PhantomData,
        })
    }

    fn get_path_of(&self, key: &K) -> PathBuf {
        self.path.join(key.hash())
    }
}

impl<'lock, K: Clone + Hash + Eq + CustomHash, V: Clone + AsRef<[u8]> + From<Vec<u8>>>
    ThreadSafeTryCacheStore<'lock> for ThreadSafeFileStore<K, V>
where
    Self: 'lock,
{
    type Key = K;
    type Value = V;
    type Error = ThreadSafeFileStoreError;
    type SLock<'guard>
        = RwLockAnyGuardKey<'lock, 'guard, (), K>
    where
        'lock: 'guard;
    type XLock = (RwLockWriteGuard<'lock, ()>, &'lock K);

    fn ts_try_get(
        &'lock self,
        handle: &Self::SLock<'_>,
    ) -> Result<Option<Self::Value>, Self::Error> {
        let path = self.get_path_of(handle.get_key());
        match File::open(path) {
            Ok(mut fil) => {
                let mut buf = vec![];
                fil.read_to_end(&mut buf)?;
                Ok(Some(buf.into()))
            }
            Err(ref error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn ts_try_set(
        &'lock self,
        handle: &mut Self::XLock,
        value: &Self::Value,
    ) -> Result<(), Self::Error> {
        let serialized = value.as_ref();

        let path = self.get_path_of(handle.1);
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        file.write_all(serialized)?;
        Ok(())
    }

    fn ts_try_exists(&'lock self, handle: &Self::SLock<'_>) -> Result<bool, Self::Error> {
        let path = self.get_path_of(handle.get_key());
        Ok(std::fs::metadata(path)?.is_file())
    }

    fn ts_try_xlock(&'lock self, key: &'lock Self::Key) -> Result<Self::XLock, Self::Error> {
        let mut cache_lock = self.cache.lock()?;
        let value = if let Some(thing) = cache_lock.get(key) {
            thing
        } else {
            cache_lock.insert(key.clone(), RwLock::default());
            cache_lock.get(key).unwrap()
        };

        // Detach the lock itself from the HashMap guard lifetime
        let value: *const _ = value;
        let lock: Self::XLock = unsafe { ((*value).write()?, key) };
        drop(cache_lock);

        Ok(lock)
    }

    fn ts_try_slock(&'lock self, key: &'lock Self::Key) -> Result<Self::SLock<'lock>, Self::Error> {
        let mut cache_lock = self.cache.lock()?;
        let value = if let Some(thing) = cache_lock.get(key) {
            thing
        } else {
            cache_lock.insert(key.clone(), RwLock::default());
            cache_lock.get(key).unwrap()
        };

        // Detach the lock itself from the HashMap guard lifetime
        let value: *const _ = value;
        let lock: Self::SLock<'_> = unsafe { ((*value).read()?, key).into() };
        drop(cache_lock);

        Ok(lock)
    }

    fn ts_try_xlock_nblock(&'lock self, key: &'lock Self::Key) -> Result<Self::XLock, Self::Error> {
        let mut cache_lock = self.cache.lock()?;
        let value = if let Some(thing) = cache_lock.get(key) {
            thing
        } else {
            cache_lock.insert(key.clone(), RwLock::default());
            cache_lock.get(key).unwrap()
        };

        // Detach the lock itself from the HashMap guard lifetime
        let value: *const _ = value;
        let lock: Self::XLock = unsafe { ((*value).try_write()?, key) };
        drop(cache_lock);

        Ok(lock)
    }

    fn ts_try_slock_nblock(
        &'lock self,
        key: &'lock Self::Key,
    ) -> Result<Self::SLock<'lock>, Self::Error> {
        let mut cache_lock = self.cache.lock()?;
        let value = if let Some(thing) = cache_lock.get(key) {
            thing
        } else {
            cache_lock.insert(key.clone(), RwLock::default());
            cache_lock.get(key).unwrap()
        };

        // Detach the lock itself from the HashMap guard lifetime
        let value: *const _ = value;
        let lock: Self::SLock<'_> = unsafe { ((*value).try_read()?, key).into() };
        drop(cache_lock);

        Ok(lock)
    }
}

// ---- With Serialization

/// Thread safe store based on files with serialization
pub struct ThreadSafeFileStoreSerializable<K, V> {
    path: PathBuf,
    cache: Mutex<HashMap<K, RwLock<()>>>,
    value_phantom: PhantomData<V>,
}

impl<K: CustomHash, V> ThreadSafeFileStoreSerializable<K, V> {
    /// Makes a new instance from a directory path
    /// Doesn't perform any file lock, you must ensure this path isn't used by other processes
    /// or even this one itself.
    ///
    /// # Errors
    /// Fails when any underlying io call does.
    pub fn new_on(path: impl AsRef<Path> + TryInto<PathBuf>) -> std::io::Result<Self> {
        std::fs::create_dir_all(&path)?;
        Ok(Self {
            path: path.try_into().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::Other, "error converting from path")
            })?,
            cache: Mutex::new(HashMap::new()),
            value_phantom: PhantomData,
        })
    }

    fn get_path_of(&self, key: &K) -> PathBuf {
        self.path.join(key.hash())
    }
}

impl<'lock, K: Clone + Hash + Eq + CustomHash, V: Clone + Serialize + DeserializeOwned>
    ThreadSafeTryCacheStore<'lock> for ThreadSafeFileStoreSerializable<K, V>
where
    Self: 'lock,
{
    type Key = K;
    type Value = V;
    type Error = ThreadSafeFileStoreError;
    type SLock<'guard>
        = RwLockAnyGuardKey<'lock, 'guard, (), K>
    where
        'lock: 'guard;
    type XLock = (RwLockWriteGuard<'lock, ()>, &'lock K);

    fn ts_try_get(
        &'lock self,
        handle: &Self::SLock<'_>,
    ) -> Result<Option<Self::Value>, Self::Error> {
        let path = self.get_path_of(handle.get_key());
        match File::open(path) {
            Ok(mut fil) => {
                let mut buf = vec![];
                fil.read_to_end(&mut buf)?;
                Ok(bincode::deserialize(buf.as_slice()).map(Some)?)
            }
            Err(ref error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn ts_try_set(
        &'lock self,
        handle: &mut Self::XLock,
        value: &Self::Value,
    ) -> Result<(), Self::Error> {
        let serialized = bincode::serialize(&value)?;

        let path = self.get_path_of(handle.1);
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        file.write_all(&serialized)?;
        Ok(())
    }

    fn ts_try_exists(&'lock self, handle: &Self::SLock<'_>) -> Result<bool, Self::Error> {
        let path = self.get_path_of(handle.get_key());
        Ok(std::fs::metadata(path)?.is_file())
    }

    fn ts_try_xlock(&'lock self, key: &'lock Self::Key) -> Result<Self::XLock, Self::Error> {
        let mut cache_lock = self.cache.lock()?;
        let value = if let Some(thing) = cache_lock.get(key) {
            thing
        } else {
            cache_lock.insert(key.clone(), RwLock::default());
            cache_lock.get(key).unwrap()
        };

        // Detach the lock itself from the HashMap guard lifetime
        let value: *const _ = value;
        let lock: Self::XLock = unsafe { ((*value).write()?, key) };
        drop(cache_lock);

        Ok(lock)
    }

    fn ts_try_slock(&'lock self, key: &'lock Self::Key) -> Result<Self::SLock<'lock>, Self::Error> {
        let mut cache_lock = self.cache.lock()?;
        let value = if let Some(thing) = cache_lock.get(key) {
            thing
        } else {
            cache_lock.insert(key.clone(), RwLock::default());
            cache_lock.get(key).unwrap()
        };

        // Detach the lock itself from the HashMap guard lifetime
        let value: *const _ = value;
        let lock: Self::SLock<'_> = unsafe { ((*value).read()?, key).into() };
        drop(cache_lock);

        Ok(lock)
    }

    fn ts_try_xlock_nblock(&'lock self, key: &'lock Self::Key) -> Result<Self::XLock, Self::Error> {
        let mut cache_lock = self.cache.lock()?;
        let value = if let Some(thing) = cache_lock.get(key) {
            thing
        } else {
            cache_lock.insert(key.clone(), RwLock::default());
            cache_lock.get(key).unwrap()
        };

        // Detach the lock itself from the HashMap guard lifetime
        let value: *const _ = value;
        let lock: Self::XLock = unsafe { ((*value).try_write()?, key) };
        drop(cache_lock);

        Ok(lock)
    }

    fn ts_try_slock_nblock(
        &'lock self,
        key: &'lock Self::Key,
    ) -> Result<Self::SLock<'lock>, Self::Error> {
        let mut cache_lock = self.cache.lock()?;
        let value = if let Some(thing) = cache_lock.get(key) {
            thing
        } else {
            cache_lock.insert(key.clone(), RwLock::default());
            cache_lock.get(key).unwrap()
        };

        // Detach the lock itself from the HashMap guard lifetime
        let value: *const _ = value;
        let lock: Self::SLock<'_> = unsafe { ((*value).try_read()?, key).into() };
        drop(cache_lock);

        Ok(lock)
    }
}

// ---- And some tests

#[cfg(test)]
mod tests {
    use std::println;

    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::tempdir;

    #[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
    struct MyValue {
        name: String,
        number: i32,
    }

    #[test]
    fn raw_set_get() {
        // Create a temporary directory for the store
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let store_path = temp_dir.path().to_path_buf();

        // Initialize the ThreadSafeFileStore
        let store = ThreadSafeFileStore::<String, Vec<u8>>::new_on(store_path)
            .expect("Failed to create ThreadSafeFileStore");

        // Define a key and value
        let key = String::from("test_key");
        let value = String::from("my value").into_bytes().as_slice().to_vec();

        println!("on {temp_dir:?}");

        // Write the value to the store
        {
            let mut xlock = store
                .ts_try_xlock_nblock(&key)
                .expect("Failed to acquire exclusive lock");
            store
                .ts_try_set(&mut xlock, &value)
                .expect("Failed to set value");
        }

        // Retrieve the value from the store
        {
            let slock = store
                .ts_try_slock_nblock(&key)
                .expect("Failed to acquire shared lock");
            let retrieved_value = store
                .ts_try_get(&slock)
                .expect("Failed to get value")
                .expect("Value not found");
            assert_eq!(
                retrieved_value, value,
                "Retrieved value does not match the original"
            );
        }
    }

    #[test]
    fn serialization_set_get() {
        // Create a temporary directory for the store
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let store_path = temp_dir.path().to_path_buf();

        // Initialize the ThreadSafeFileStore
        let store = ThreadSafeFileStoreSerializable::<String, MyValue>::new_on(store_path)
            .expect("Failed to create ThreadSafeFileStore");

        // Define a key and value
        let key = String::from("test_key");
        let value = MyValue {
            name: String::from("test_name"),
            number: 42,
        };

        println!("on {temp_dir:?}");

        // Write the value to the store
        {
            let mut xlock = store
                .ts_try_xlock_nblock(&key)
                .expect("Failed to acquire exclusive lock");
            store
                .ts_try_set(&mut xlock, &value)
                .expect("Failed to set value");
        }

        // Retrieve the value from the store
        {
            let slock = store
                .ts_try_slock_nblock(&key)
                .expect("Failed to acquire shared lock");
            let retrieved_value = store
                .ts_try_get(&slock)
                .expect("Failed to get value")
                .expect("Value not found");
            assert_eq!(
                retrieved_value, value,
                "Retrieved value does not match the original"
            );
        }
    }

    #[test]
    fn file_get_inexistent() {
        // Create a temporary directory for the store
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let store_path = temp_dir.path().to_path_buf();

        // Initialize the ThreadSafeFileStore
        let store = ThreadSafeFileStoreSerializable::<String, ()>::new_on(store_path)
            .expect("Failed to create ThreadSafeFileStore");

        assert_eq!(
            store
                .ts_one_try_get(&String::from("key that doesn't exist"))
                .expect("to not fail"),
            None
        );
    }
}
