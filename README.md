[![License](https://badgen.net/github/license/javalsai/rs-ezcache)](https://github.com/javalsai/rs-ezcache/blob/master/LICENSE)
[![Branches](https://badgen.net/github/branches/javalsai/rs-ezcache)](https://github.com/javalsai/rs-ezcache)
[![Latest Release](https://badgen.net/github/release/javalsai/rs-ezcache)](https://github.com/javalsai/rs-ezcache/releases)
[![CI Runs](https://badgen.net/github/checks/javalsai/rs-ezcache)](https://github.com/javalsai/rs-ezcache/actions)

# ezcache

Rust library that provides building blocks to create useful and flexible cache stores. With memory and file based stores by default. Available at:
* [GitHub](https://github.com/javalsai/rs-ezcache)
* [crates.io](https://crates.io/crates/ezcache).

# Features
- Traits to implement cache stores. Features faillible and infallible variants.
- Cache stores with default generators that activate by default when needed.
- Thread safe variants of everything possible under the "thread-safe" feature.
- Default cache stores implemented for filesystem, memory, etc.

# Documentation

The library is intended to be mainly documented through native cargo docs. These are deployed automatically by github actions to <https://javalsai.github.io/rs-ezcache/>. Those are guaranteed to have the latest information, so consider checking those first as they are the most reliable source.

# Examples

There are several examples on the [documentation](#documentation), some more complete examples on the [examples directory](examples/) and tests scattered around can also serve as examples.

# Contributing

Feel free to open any issue, fork, contribute, open a discussion... for anything. Guidelines on how to be organized with this will be created when it gets some more use, for now you're free to do it however you want.

# Features

The library aims to be as flexible as possible, potentially sacrificing some performance by default (as little as possible). For this reason there are a few features that you can use:
* `std*`: Enables std features, provides most of the default stuff, without it you are quite limited, but you might even be able to use this in embedded (I don't see why though).
* `thread-safe*`: Adds all the thread safe traits and wrappers.
* `file-stores*`: Enables file stores, depends on a few other crates.
* `nightly`: Enables nightly features, this library is completely std at the current moment however.

> Features marked with `*` are enabled by default
