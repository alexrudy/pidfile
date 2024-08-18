# pidfile: PID file locking and management with RAII semantics

[![crate][crate-image]][crate-link]
[![Docs][docs-image]][docs-link]
[![Build Status][build-image]][build-link]
![MIT licensed][license-image]

PID files are a crude form of locking which uses the filesystem to ensure that only one instance of a
program is running at a time. This crate provides a simple API for creating and managing PID files
in a way that is safe and easy to use. PID Files will be cleaned up on drop, and can be checked for
existence and validity.

There is already a [pidfile](https://crates.io/crates/pidfile) crate, but it is not updated. This one
is a more modern approach to the same problem, and on crates.io as `pidfile2`.

[crate-image]: https://buildstats.info/crate/pidfile
[crate-link]: https://crates.io/crates/pidfile2
[docs-image]: https://docs.rs/pidfile2/badge.svg
[docs-link]: https://docs.rs/pidfile2/
[build-image]: https://github.com/alexrudy/pidfile/actions/workflows/ci.yml/badge.svg
[build-link]: https://github.com/alexrudy/pidfile/actions/workflows/ci.yml
[license-image]: https://img.shields.io/badge/license-MIT-blue.svg
