use std::{fmt::Display, io::Read, ops::DivAssign, path::PathBuf, sync::Arc, time::Instant};

use ezcache::{
    prelude::*,
    stores::file_stores::{ThreadSafeFileStore, ThreadSafeFileStoreError},
};
use indicatif::{MultiProgress, ProgressBar};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
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

const BS: usize = 2048;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let req_to_std_io = |e| std::io::Error::new(std::io::ErrorKind::Other, e);

    let args: Vec<_> = std::env::args().collect();
    let dpath: PathBuf = args
        .get(1)
        .expect("pass a download directory as argument")
        .parse()
        .expect("argument was not a valid number");
    println!("\x1b[1;3;4;31mif cache'd stuff is too slow, it's probably computing a hash\x1b[0m\n");

    let store: ThreadSafeFileStore<&'static str, Vec<u8>> = ThreadSafeFileStore::new_on(&dpath)?;
    let store = ThreadSafeGenTryCacheStoreWrapper::new(
        store,
        |k: &&str, (client, pb): (&reqwest::blocking::Client, ProgressBar)| {
            let mut res = client
                .get(*k)
                .send()
                .map_err(req_to_std_io)?
                .error_for_status()
                .map_err(req_to_std_io)?;

            if let Some(len) = res.content_length() {
                pb.set_position(0);
                pb.set_length(len);
                #[allow(clippy::cast_possible_truncation)]
                let mut buf: Vec<u8> = vec![0; len as usize];
                buf.chunks_mut(BS).try_for_each(|ref mut chunk| {
                    res.read_exact(chunk)?;
                    pb.inc(chunk.len() as u64);
                    Ok::<(), std::io::Error>(())
                })?;
                Ok(buf)
            } else {
                let bytes = res.bytes().map_err(req_to_std_io)?.to_vec();
                pb.set_position(u64::MAX);

                Ok(bytes)
            }
        },
    );
    let arc_store = Arc::new(store);
    let arc_clone = Arc::clone(&arc_store);

    let mpb = MultiProgress::new();
    let client = reqwest::blocking::Client::new();

    let ipad = (SOURCES.len().ilog10() + 1) as usize;
    SOURCES
        .par_iter()
        .enumerate()
        .try_for_each(|(i, (name, url))| -> Result<(), ThreadSafeFileStoreError> {
            let this_bar = mpb.insert(i, ProgressBar::new(8));
            this_bar.set_style(
                // ._. or º-º
                indicatif::ProgressStyle::with_template(&format!(
                    "[{{bar}}] \x1b[33m{i:ipad$} \x1b[35m'{name}'\x1b[0m - \x1b[36m{url} \x1b[0m{{msg}}\x1b[0m",
                    i = i + 1,
                ))
                .unwrap()
                .progress_chars("##-"),
            );
            this_bar.set_message("\x1b[33m- downloading...");

            let store = &arc_clone;

            let a = Instant::now();
            let value = store.ts_try_get_or_new(url, (&client,this_bar.clone()))?;
            let b = Instant::now();
            let size = normalize_len(value.len() as f32);
            let pre_hash_msg = format!(
                "- downloaded \x1b[35m{size}\x1b[0m in \x1b[35m{:?}\x1b[0m",
                b - a,
            );
            this_bar.set_message(pre_hash_msg.clone());

            let hash_a = Instant::now();
            let hash = Sha256::new()
                .chain_update(&value)
                .finalize()
                .into_iter()
                .fold(String::new(), |acc, b| acc + &format!("{b:X}"));
            #[allow(clippy::cast_precision_loss)]
            let hash_b = Instant::now();

            this_bar.set_message(format!(
                "{pre_hash_msg}\n\x1b[30mhash {hash} {:?}",
                hash_b - hash_a,
            ));
            this_bar.abandon();

            Ok(())
        })
        .expect("some thread failed");

    Ok(())
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
