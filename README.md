# follow-redirects [![Build Status](https://travis-ci.org/srijs/rust-follow-redirects.svg?branch=master)](https://travis-ci.org/srijs/rust-follow-redirects)

Extension for `hyper` to follow HTTP redirects.

## Example

```rust
extern crate hyper;
extern crate follow_redirects;

// 1. import the extension trait
use follow_redirects::ClientExt;

// ...
// 2. create a standard hyper client
let client = hyper::Client::new(&handle);

// ...
// 3. make a request that will follow redirects
let url = "http://docs.rs/hyper".parse().unwrap();
let future = client.follow_redirects().get(url);
```
