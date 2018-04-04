//! Extension for `hyper` to follow HTTP redirects.
//!
//! # Behaviour and Limitations
//!
//! ## Supported Redirects
//!
//! This crate supports all 5 redirect styles that are defined by the HTTP/1.1 spec.
//! These include both "permanent" redirects (status codes `301` and `308`) as well as
//! "temporary" redirects (status codes `302`, `303` and `307`).
//!
//! Encountering a `303 See Other` response from the server will cause the client to change
//! the request method to `GET`, and discard the request body before following the redirect.
//!
//! ## Location Header
//!
//! The client uses the `Location` HTTP header to determine the next url to follow the redirect to.
//! Both relative (without a hostname) as well as absolute URLs (including schema and hostname) urls
//! are supported.
//!
//! When the `Location` header is missing in a response from the server, but the status code indicates
//! a redirect, the client will return the response to the caller. You can detect this situation by
//! checking the status code and headers of the returned response.
//!
//! ## Buffering
//!
//! In order to be able to replay the request body when following redirects other than `303 See Other`,
//! the client will buffer the request body in-memory before making the first request to the
//! server.
//!
//! ## Redirect Limit
//!
//! To avoid following an endless chain of redirects, the client has a limit for the maximum number
//! of redirects it will follow until giving up.
//!
//! When the maximum number of redirects is reached, the client will simply return the last response
//! to the caller. You can detect this situation by checking the status code of the returned response.
//!
//! ## Security Considerations
//!
//! In order to protect the confidentiality of authentication or session information passed along in
//! a request, the client will strip authentication and cookie headers when following a redirect to
//! a different host and port.
//!
//! Redirects to the same host and port, but different paths will retain session information.
//!
//! # Example
//!
//! ```rust
//! extern crate hyper;
//! extern crate follow_redirects;
//! # extern crate tokio_core;
//!
//! // 1. import the extension trait
//! use follow_redirects::ClientExt;
//!
//! # fn main() {
//! # let core = tokio_core::reactor::Core::new().unwrap();
//! # let handle = core.handle();
//! // ...
//! // 2. create a standard hyper client
//! let client = hyper::Client::new(&handle);
//!
//! // ...
//! // 3. make a request that will follow redirects
//! let url = "http://docs.rs/hyper".parse().unwrap();
//! let future = client.follow_redirects().get(url);
//! # drop(future);
//! # }
//! ```

#![deny(missing_docs)]
#![deny(warnings)]
#![deny(missing_debug_implementations)]

extern crate bytes;
#[macro_use] extern crate futures;
extern crate http;
extern crate hyper;

use std::fmt;

use bytes::Bytes;
use futures::{Future, Poll, Stream};
use hyper::{Error, Method, Request, Response, Uri};
use hyper::client::{Connect, Service};

mod uri;
mod buffer;
mod machine;
mod future;

use future::FutureInner;

/// Extension trait for adding follow-redirect features to `hyper::Client`.
pub trait ClientExt<C, B> {
    /// Wrap the `hyper::Client` in a new client that follows redirects.
    fn follow_redirects(&self) -> Client<C, B>;

    /// Wrap the `hyper::Client` in a new client that follows redirects,
    /// and set the maximum number of redirects to follow.
    fn follow_redirects_max(&self, max_redirects: usize) -> Client<C, B> {
        let mut client = self.follow_redirects();
        client.set_max_redirects(max_redirects);
        client
    }
}

impl<C, B> ClientExt<C, B> for hyper::Client<C, B> {
    fn follow_redirects(&self) -> Client<C, B> {
        Client {
            inner: self.clone(),
            max_redirects: 10
        }
    }
}

/// A client to make outgoing HTTP requests, and which follows redirects.
///
/// By default, the client will follow up to 10 redirects. This limit can be configured
/// via the `set_max_redirects` method.
pub struct Client<C, B> {
    inner: hyper::Client<C, B>,
    max_redirects: usize
}

impl<C, B> Clone for Client<C, B> {
    fn clone(&self) -> Client<C, B> {
        Client {
            inner: self.inner.clone(),
            max_redirects: self.max_redirects
        }
    }
}

impl<C, B> fmt::Debug for Client<C, B> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Client")
            .field("max_redirects", &self.max_redirects)
            .finish()
    }
}

impl<C, B> Client<C, B>
    where C: Connect,
          B: Stream<Error = Error> + From<Bytes> + 'static,
          B::Item: AsRef<[u8]>
{
    /// Send a GET Request using this client.
    pub fn get(&self, url: Uri) -> FutureResponse {
        self.request(Request::new(Method::Get, url))
    }

    /// Send a constructed Request using this client.
    pub fn request(&self, req: Request<B>) -> FutureResponse {
        FutureResponse(Box::new(FutureInner::new(self.inner.clone(), req, self.max_redirects)))
    }
}

impl<C, B> Client<C, B> {
    /// Set the maximum number of redirects the client will follow before giving up.
    ///
    /// By default, this limit is set to 10.
    pub fn set_max_redirects(&mut self, max_redirects: usize) {
        self.max_redirects = max_redirects;
    }
}

impl<C, B> Service for Client<C, B>
    where C: Connect,
          B: Stream<Error = Error> + From<Bytes> + 'static,
          B::Item: AsRef<[u8]>
{
    type Request = Request<B>;
    type Response = Response;
    type Error = Error;
    type Future = FutureResponse;

    fn call(&self, req: Request<B>) -> FutureResponse {
        self.request(req)
    }
}

/// A `Future` that will resolve to an HTTP Response.
pub struct FutureResponse(Box<Future<Item=Response, Error=Error>>);

impl fmt::Debug for FutureResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("Future<Response>")
    }
}

impl Future for FutureResponse {
    type Item = Response;
    type Error = Error;

    fn poll(&mut self) -> Poll<Response, Error> {
        self.0.poll()
    }
}
