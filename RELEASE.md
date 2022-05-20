0.6.0
=====

* **Breaking Change**
  * Rename `bytes_into_cbor()` to `from_bytes()`, confirms to rust convention.
* new API `get_cborize_id()` confirms with Cborize.
* To run on rust stable
  * Fix compilation issue with missing Arc support in CBOR
  * Use a stub for `total_cmp` until it is stabalized in rust
* clippy fixes
* rust doc

0.5.4
=====

* Remove edition2021 dependency.

0.5.1 & 0.5.2 & 0.5.3
=====================

* Clippy fixes.
* Add TagNum for clean handling of cbor-tags.
* TagNum::Any uses 65535 as the discriminant value.
* pretty-printing
* cargo: enable extra-traits for `syn` package.
* make: add build/test/bench/doc/clippy for cbordata-derive in Makefile.
* make: clippy fixes for cbordata-derive.
* cbordata-derive: bugfixes.

0.5.0
=====

* **Breaking change**: This package is moved out of [mkit][mkit] _ver:0.4.0_
* refactor IntoCbor and FromCbor implementations into `types.rs`.
* Key: convertion trait implementation for Key type.
* add benchmark suite.
* clippy fixes.

[mkit]: https://github.com/bnclabs/mkit
