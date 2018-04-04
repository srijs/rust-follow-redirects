use std::io;

use bytes::Bytes;
use http;
use hyper::{self, Error, Headers, HttpVersion, Method, Request, Response, StatusCode, Uri};

pub(crate) struct StateMachine {
    method: Method,
    uri: Uri,
    version: HttpVersion,
    headers: Headers,
    body: Option<Bytes>,
    remaining_redirects: usize
}

pub(crate) enum StateMachineDecision {
    Continue,
    Return
}

impl StateMachine {
    pub fn new<B>(req: &mut Request<B>, max_redirects: usize) -> StateMachine {
        let mut state = StateMachine {
            method: req.method().clone(),
            uri: req.uri().clone(),
            version: req.version(),
            headers: Headers::new(),
            body: None,
            remaining_redirects: max_redirects
        };
        state.headers = ::std::mem::replace(req.headers_mut(), Headers::new());
        state
    }

    pub fn set_body(&mut self, body: Option<Bytes>) {
        self.body = body;
    }

    pub fn create_request<B: From<Bytes>>(&self) -> Request<B> {
        let mut req = Request::new(self.method.clone(), self.uri.clone());
        req.set_version(self.version);
        req.headers_mut().clone_from(&self.headers);
        if let Some(ref bytes) = self.body {
            req.set_body(bytes.clone());
        }
        req
    }

    pub fn handle_response(&mut self, res: &Response) -> Result<StateMachineDecision, Error> {
        match res.status() {
            StatusCode::MovedPermanently | StatusCode::PermanentRedirect => {
                self.follow_redirect(res)
            },
            StatusCode::Found | StatusCode::TemporaryRedirect => {
                self.follow_redirect(res)
            },
            StatusCode::SeeOther => {
                self.method = Method::Get;
                self.body = None;
                self.follow_redirect(res)
            },
            _ => {
                Ok(StateMachineDecision::Return)
            }
        }
    }

    fn follow_redirect(&mut self, res: &Response) -> Result<StateMachineDecision, Error> {
        if self.remaining_redirects == 0 {
            return Ok(StateMachineDecision::Return);
        }
        self.remaining_redirects -= 1;
        let opt_new_uri_res = res.headers().get::<hyper::header::Location>().map(|location| {
            println!("location: {}", location);
            location.parse::<http::Uri>()
                .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
        });
        if let Some(new_uri_res) = opt_new_uri_res {
            println!("current uri {}", self.uri);
            let new_uri = new_uri_res?;
            let old_uri = self.uri.as_ref().parse::<http::Uri>()
                .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
            let old_parts = http::uri::Parts::from(old_uri);
            let mut new_parts = http::uri::Parts::from(new_uri);
            if new_parts.scheme.is_none() {
                new_parts.scheme = old_parts.scheme;
            }
            if new_parts.authority.is_none() {
                new_parts.authority = old_parts.authority;
            }
            let absolute_new_uri = http::Uri::from_parts(new_parts)
                .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
            self.uri = absolute_new_uri.to_string().parse::<Uri>()?;
            Ok(StateMachineDecision::Continue)
        } else {
            Ok(StateMachineDecision::Return)
        }
    }
}
