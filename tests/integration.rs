extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate follow_redirects;

use futures::{Future, Stream, future};
use hyper::{Client, Error, Request, Response, StatusCode};
use hyper::header::Location;
use hyper::server::{Http, Service};
use tokio_core::reactor::Core;
use follow_redirects::ClientExt;

#[test]
fn redirect_chain() {
    let mut core = Core::new().unwrap();

    let http = Http::new();
    let addr = "127.0.0.1:0".parse().unwrap();
    let serve = http.serve_addr_handle(&addr, &core.handle(), || Ok(Svc)).unwrap();
    let local_addr = serve.incoming_ref().local_addr().clone();
    let spawn_handle = core.handle();
    let serve_all = serve.for_each(move |conn| Ok(spawn_handle.spawn(conn.map_err(|err| panic!("err {}", err)))));
    core.handle().spawn(serve_all.map_err(|err| panic!("err {}", err)));

    struct Svc;
    impl Service for Svc {
        type Request = Request;
        type Response = Response;
        type Error = Error;
        type Future = Box<Future<Item=Response, Error=Error>>;

        fn call(&self, req: Request) -> Self::Future {
            if req.path() == "/" {
                let res = Response::new()
                    .with_status(StatusCode::MovedPermanently)
                    .with_header(Location::new("/foo"));
                Box::new(future::ok(res))
            } else if req.path() == "/foo" {
                let res = Response::new()
                    .with_status(StatusCode::PermanentRedirect)
                    .with_header(Location::new("/bar"));
                Box::new(future::ok(res))
            } else if req.path() == "/bar" {
                let res = Response::new()
                    .with_status(StatusCode::Found)
                    .with_header(Location::new("/baz"));
                Box::new(future::ok(res))
            } else if req.path() == "/baz" {
                let res = Response::new()
                    .with_status(StatusCode::TemporaryRedirect)
                    .with_header(Location::new("/quux"));
                Box::new(future::ok(res))
            } else if req.path() == "/quux" {
                let res = Response::new()
                    .with_status(StatusCode::SeeOther)
                    .with_header(Location::new("/other"));
                Box::new(future::ok(res))
            } else if req.path() == "/other" {
                let res = Response::new()
                    .with_status(StatusCode::Accepted);
                Box::new(future::ok(res))
            } else {
                let res = Response::new()
                    .with_status(StatusCode::BadRequest);
                Box::new(future::ok(res))
            }
        }
    }

    let client = Client::new(&core.handle());
    let url = format!("http://{}", local_addr).parse().unwrap();
    let future_response = client.follow_redirects().get(url);
    let response = core.run(future_response).unwrap();
    assert_eq!(StatusCode::Accepted, response.status());
}
