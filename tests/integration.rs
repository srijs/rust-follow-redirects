use hyper::body::Body;
use hyper::header::LOCATION;
use hyper::server::Server;
use hyper::{Client, Error, Request, Response, StatusCode};

use follow_redirects::ClientExt;

#[test]
fn redirect_chain() {
    let make_svc = hyper::service::make_service_fn(|_| async {
        Ok::<_, Error>(hyper::service::service_fn(|req: Request<Body>| async move {
            Ok::<_, Error>(if req.uri().path() == "/" {
                Response::builder()
                    .status(StatusCode::MOVED_PERMANENTLY)
                    .header(LOCATION, "/foo")
                    .body(Body::empty())
                    .unwrap()
            } else if req.uri().path() == "/foo" {
                Response::builder()
                    .status(StatusCode::PERMANENT_REDIRECT)
                    .header(LOCATION, "/bar")
                    .body(Body::empty())
                    .unwrap()
            } else if req.uri().path() == "/bar" {
                Response::builder()
                    .status(StatusCode::FOUND)
                    .header(LOCATION, "/baz")
                    .body(Body::empty())
                    .unwrap()
            } else if req.uri().path() == "/baz" {
                Response::builder()
                    .status(StatusCode::TEMPORARY_REDIRECT)
                    .header(LOCATION, "/quux")
                    .body(Body::empty())
                    .unwrap()
            } else if req.uri().path() == "/quux" {
                Response::builder()
                    .status(StatusCode::SEE_OTHER)
                    .header(LOCATION, "/other")
                    .body(Body::empty())
                    .unwrap()
            } else if req.uri().path() == "/other" {
                Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap()
            } else {
                Response::builder().status(StatusCode::BAD_REQUEST).body(Body::empty()).unwrap()
            })
        }))
    });

    let rt =
        tokio::runtime::Builder::new_current_thread().enable_all().build().expect("build runtime");

    rt.block_on(async {
        let addr = "127.0.0.1:0".parse().unwrap();
        let server = Server::bind(&addr).serve::<_, Body>(make_svc);
        let local_addr = server.local_addr().clone();
        tokio::spawn(server);

        let client = Client::new();
        let url = format!("http://{}", local_addr).parse().unwrap();
        let response = client.follow_redirects().get(url).await.unwrap();
        assert_eq!(StatusCode::ACCEPTED, response.status());
    });
}
