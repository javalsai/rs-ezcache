#[path = "_common.rs"]
pub mod common;

use std::{io::Read, path::PathBuf, sync::Arc, time::Instant};

use ezcache::{
    prelude::*,
    stores::file_stores::{ThreadSafeFileStore, ThreadSafeFileStoreError},
};
use indicatif::{MultiProgress, ProgressBar};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use sha2::{Digest, Sha256};
use thiserror::Error;

// Our main error type
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    CacheStore(#[from] ThreadSafeFileStoreError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

const BS: usize = 2048;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // We get a cache dir path
    let args: Vec<_> = std::env::args().collect();
    let dpath: PathBuf = args
        .get(1)
        .expect("pass a download directory as argument")
        .parse()
        .expect("argument was not a valid number");
    println!("\x1b[1;3;4;31mif cache'd stuff is too slow, it's probably computing a hash\x1b[0m\n");

    // Aaand, we make the generative cache store
    let store: ThreadSafeGenTryCacheStoreWrapper<'_, _, _, Error, _, _, _, _, _> =
        ThreadSafeGenTryCacheStoreWrapper::new(
            ThreadSafeFileStore::new_on(&dpath)?,
            // With a fancy generator function
            |k: &&str,
             (client, pb): (&reqwest::blocking::Client, ProgressBar)|
             -> Result<Vec<u8>, Error> {
                let mut res = client.get(*k).send()?.error_for_status()?;

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
                    let bytes = res.bytes()?.to_vec();
                    pb.set_position(u64::MAX);

                    Ok(bytes)
                }
            },
        );

    // Thread safety
    let arc_store = Arc::new(store);
    let arc_clone = Arc::clone(&arc_store);

    // Some globals
    let mpb = MultiProgress::new();
    let client = reqwest::blocking::Client::new();

    // And the multithreaded part
    let ipad = (common::SOURCES.len().ilog10() + 1) as usize;
    common::SOURCES
        .par_iter()
        .enumerate()
        .try_for_each(|(i, (name, url))| -> Result<(), Error> {
            let store = &arc_clone;

            // Bar printing stuff
            let this_bar = mpb.insert(i, ProgressBar::new(8));
            this_bar.set_style(
                indicatif::ProgressStyle::with_template(&format!(
                    "[{{bar}}] \x1b[33m{i:ipad$} \x1b[35m'{name}'\x1b[0m - \x1b[36m{url} \x1b[0m{{msg}}\x1b[0m",
                    i = i + 1,
                ))
                .unwrap()
                .progress_chars("##-"),
            );
            this_bar.set_message("\x1b[33m- downloading...");

            // We call the store
            let a = Instant::now();
            let value = store.ts_try_get_or_new(url, (&client,this_bar.clone()))?;
            let b = Instant::now();

            // More printing stuff
            #[allow(clippy::cast_precision_loss)]
            let pre_hash_msg = format!(
                "- downloaded \x1b[35m{size}\x1b[0m in \x1b[35m{time:?}\x1b[0m",
                size = common::normalize_len(value.len() as f32),
                time = b - a,
            );
            this_bar.set_message(pre_hash_msg.clone());

            // And hash just to make sure
            let hash_a = Instant::now();
            let hash = Sha256::new()
                .chain_update(&value)
                .finalize()
                .into_iter()
                .fold(String::new(), |acc, b| acc + &format!("{b:X}"));
            let hash_b = Instant::now();

            this_bar.set_message(format!(
                "{pre_hash_msg}\n\x1b[30mhash {hash} {time:?}",
                time = hash_b - hash_a,
            ));
            this_bar.abandon();

            Ok(())
        })
        .expect("some thread failed");

    Ok(())
}
