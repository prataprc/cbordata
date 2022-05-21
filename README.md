Why yet another JSON package in Rust ?
======================================

[![Rustdoc](https://img.shields.io/badge/rustdoc-hosted-blue.svg)](https://docs.rs/cbordata)
[![Build Status](https://travis-ci.org/bnclabs/cbordata.svg?branch=master)](https://travis-ci.org/bnclabs/cbordata)

This crate makes several trade-offs that are tuned for big-data
and document database.

What is CBOR
============

* Concise Binary Object Representation, also called CBOR, RFC-7049link.
* Machine friendly, designed for IoT, inter-networking of light weight devices, and easy to implement in many languages.
* Can be used for more than data exchange, left to user imagination :) ...

----

* [x] Serialization from Rust native type to CBOR binary.
* [x] De-serialization from CBOR binary to Rust native type.
* [ ] Streaming CBOR parser.
* [ ] Sorted keys in property object.

Useful links
============

* **[API Documentation](https://docs.rs/jsondata)**
* [CBOR][https://cbor.io/] specification 
* [RFC specification][cbor-rfc] for CBOR.
* [Release notes](./RELEASE.md).

Contribution
------------

* Simple workflow. Fork - Modify - Pull request.
* Before creating a PR,
  * Run `make build` to confirm all versions of build is passing with
    0 warnings and 0 errors.
  * Run `check.sh` with 0 warnings, 0 errors and all testcases passing.
  * Run `perf.sh` with 0 warnings, 0 errors and all testcases passing.
  * [Install][spellcheck] and run `cargo spellcheck` to remove common spelling mistakes.
* [Developer certificate of origin][dco] is preferred.

[spellcheck]: https://github.com/drahnr/cargo-spellcheck
[dco]: https://developercertificate.org/
[cbor-rfc]: https://tools.ietf.org/html/rfc7049
