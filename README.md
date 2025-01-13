[![License](https://badgen.net/github/license/javalsai/rs-ezcache)](https://github.com/javalsai/rs-ezcache/blob/master/LICENSE)
[![Branches](https://badgen.net/github/branches/javalsai/rs-ezcache)](https://github.com/javalsai/rs-ezcache)
[![Latest Release](https://badgen.net/github/release/javalsai/rs-ezcache)](https://github.com/javalsai/rs-ezcache/releases)
[![CI Runs](https://badgen.net/github/checks/javalsai/rs-ezcache)](https://github.com/javalsai/rs-ezcache/actions)

# ezcache

Rust library that provides building blocks to create useful and flexible cache stores. With memory and file based stores by default. It's only available through this GitHub repository, tho I'll consider adding it to [crates.io](https://crates.io).

# Features
- Traits to implement cache stores. Feature faillible and infallible variants.
- Cache stores with default generators that activate by default when needed.
- Thread safe variants of everything possible under the "thread-safe" feature.
- Default cache stores implemented for filesystem, memory, etc.

# Documentation

The library is intended to be mainly documented through native cargo docs. These are deployed automatically by github actions to https://javalsai.github.io/rs-ezcache/doc/ezcache/. That is guaranteed to have the most updated information, so consider checking it out before relying on anything said elsewhere, it's likely outdated.

# Contributing

Feel free to open any issue, fork, contribute, open a discussion... for anything. Guidelines on how to be organized with this will be created when it gets some more use, for now you're free to do it however you want.

# Features

The library aims to be as flexible as possible, potentially sacrificing some performance by default. For this reason there are a few features that you can use:
* `std*`: Enables std features, provides most of the default stuff, without it you are quite limited. Without it, you might even be able to use this in embedded.
* `thread-safe*`: Adds all the thread safe traits and wrappers.
* `nightly`: Enables nightly features.

> Features marked with `*` are enabled by default
