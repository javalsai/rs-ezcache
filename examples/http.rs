#[path = "_common.rs"]
pub mod common;

use std::time::Instant;

use ezcache::{prelude::*, TryCacheStoreErrorMap};
use rand::Rng;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}
impl From<std::convert::Infallible> for Error {
    fn from(_: std::convert::Infallible) -> Self {
        unreachable!()
    }
}

fn main() {
    // Optionally get how many runs to do
    let args: Vec<_> = std::env::args().collect();
    let n: usize = args
        .get(1)
        .map_or((common::SOURCES.len() * 5).div_ceil(2), |a| {
            a.parse().expect("argument was not a valid number")
        });

    let store: TryCacheStoreErrorMap<_, _, _, Error, _> =
        TryCacheStoreErrorMap::from_store(MemoryStore::new());
    let mut store = TryGenCacheStoreWrapper::new(
        store,
        |k: &&str, (client,): (&reqwest::blocking::Client,)| -> Result<_, reqwest::Error> {
            Ok(client.get(*k).send()?.error_for_status()?.bytes()?.to_vec())
        },
    );

    let client = reqwest::blocking::Client::new();
    let mut rng = rand::thread_rng();

    for i in 0..n {
        let (name, url) = common::SOURCES[rng.gen::<usize>() % common::SOURCES.len()];
        println!(
            "\x1b[1;33m{}\x1b[0m: downloading \x1b[36m{name}\x1b[0m - \x1b[35m{url}\x1b[0m",
            i + 1
        );

        let a = Instant::now();
        let value = store
            .try_get_or_new(url, (&client,))
            .expect("unknown error downloading");
        let b = Instant::now();

        let hash = Sha256::new()
            .chain_update(&value)
            .finalize()
            .into_iter()
            .fold(String::new(), |acc, b| acc + &format!("{b:X}"));
        #[allow(clippy::cast_precision_loss)]
        let size = common::normalize_len(value.len() as f32);

        println!(
            "fetched \x1b[35m{size}\x1b[0m in \x1b[35m{:?}\x1b[0m (sha256 \x1b[1;4;30m{hash}\x1b[0m)\n",
            b - a
        );
    }
}
