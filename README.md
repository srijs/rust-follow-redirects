# follow-redirects

Extension for `hyper` to follow HTTP redirects.

[Documentation](https://docs.rs/follow-redirects)

[![Build Status](https://travis-ci.org/srijs/rust-follow-redirects.svg?branch=master)](https://travis-ci.org/srijs/rust-follow-redirects)
[![crates.io](https://img.shields.io/crates/v/follow-redirects.svg)](https://crates.io/crates/follow-redirects)

## Example

```rust
// 1. import the extension trait
use follow_redirects::ClientExt;

// ...
// 2. create a standard hyper client
let client = hyper::Client::new();

// ...
// 3. make a request that will follow redirects
let url = "http://docs.rs/hyper".parse().unwrap();
let future = client.follow_redirects().get(url);
```

## MSRV

This crate supports Rust version 1.49.0 and higher.
