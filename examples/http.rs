use std::{fmt::Display, ops::DivAssign, time::Instant};

use ezcache::{prelude::*, TryCacheStoreErrorMap};
use rand::Rng;
use sha2::{Digest, Sha256};

const SOURCES: &[(&str, &str)] = &[
    (
        "javalsai/lidm latest zip",
        "https://github.com/javalsai/lidm/archive/refs/heads/master.zip",
    ),
    (
        "wikipedia article on rust",
        "https://en.wikipedia.org/wiki/Rust_(programming_language)",
    ),
    (
        "rust-lang/rust latest zip",
        "https://github.com/rust-lang/rust/archive/refs/heads/master.zip",
    ),
];

#[derive(Debug)]
pub struct E(pub reqwest::Error);
impl From<std::convert::Infallible> for E {
    fn from(_: std::convert::Infallible) -> Self {
        unreachable!()
    }
}
impl From<reqwest::Error> for E {
    fn from(value: reqwest::Error) -> Self {
        Self(value)
    }
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let n: usize = args.get(1).map_or((SOURCES.len() * 5).div_ceil(2), |a| {
        a.parse().expect("argument was not a valid number")
    });

    let store = MemoryStore::new();
    let store: TryCacheStoreErrorMap<_, _, _, E, _> = TryCacheStoreErrorMap::from_store(store);
    let mut store = TryGenCacheStoreWrapper::new(
        store,
        |k: &&str, (client,): (&reqwest::blocking::Client,)| {
            Ok(client.get(*k).send()?.error_for_status()?.bytes()?.to_vec())
        },
    );

    let client = reqwest::blocking::Client::new();
    let mut rng = rand::thread_rng();

    for i in 0..n {
        let (name, url) = SOURCES[rng.gen::<usize>() % SOURCES.len()];
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
        let size = normalize_len(value.len() as f32);

        println!(
            "fetched \x1b[35m{size}\x1b[0m in \x1b[35m{:?}\x1b[0m (sha256 \x1b[1;4;30m{hash}\x1b[0m)\n",
            b - a
        );
    }
}

pub static MAGNITUDE_PREFIX_BINARY: &[&str] = &["", "Ki", "Mi", "Gi", "Ti", "Pi", "Ei", "Zi"];
fn normalize_len<T: DivAssign + PartialOrd + Copy + From<u16> + Display>(mut amount: T) -> String {
    let max_idx = MAGNITUDE_PREFIX_BINARY.len() - 1;
    let mut unit_idx = 0;
    let radix = T::from(1024);

    while amount >= radix && unit_idx < max_idx {
        amount /= radix;
        unit_idx += 1;
    }

    format!("{amount:.2}{}B", MAGNITUDE_PREFIX_BINARY[unit_idx])
}
