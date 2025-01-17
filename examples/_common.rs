use std::{fmt::Display, ops::DivAssign};

pub const SOURCES: &[(&str, &str)] = &[
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

pub static MAGNITUDE_PREFIX_BINARY: &[&str] = &["", "Ki", "Mi", "Gi", "Ti", "Pi", "Ei", "Zi"];
pub fn normalize_len<T: DivAssign + PartialOrd + Copy + From<u16> + Display>(
    mut amount: T,
) -> String {
    let max_idx = MAGNITUDE_PREFIX_BINARY.len() - 1;
    let mut unit_idx = 0;
    let radix = T::from(1024);

    while amount >= radix && unit_idx < max_idx {
        amount /= radix;
        unit_idx += 1;
    }

    format!("{amount:.2}{}B", MAGNITUDE_PREFIX_BINARY[unit_idx])
}

#[allow(dead_code)]
fn main() {}
