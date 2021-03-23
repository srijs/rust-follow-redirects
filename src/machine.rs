
use bytes::Bytes;
use hyper::body::Body;
use hyper::header::HeaderMap;
use hyper::{self, Method, Request, Response, StatusCode, Uri, Version};

use crate::error::Error;
use crate::uri::UriExt;

pub(crate) struct StateMachine {
    method: Method,
    uri: Uri,
    version: Version,
    headers: HeaderMap,
    // This is None while we are still receiving the request body.
    pub(crate) request_body: Option<Bytes>,
    remaining_redirects: usize,
}

pub(crate) enum StateMachineDecision {
    Continue,
    Return,
}

impl StateMachine {
    pub fn new<B>(req: &mut Request<B>, max_redirects: usize) -> StateMachine {
        let mut state = StateMachine {
            method: req.method().clone(),
            uri: req.uri().clone(),
            version: req.version(),
            headers: HeaderMap::new(),
            request_body: None,
            remaining_redirects: max_redirects,
        };
        state.headers = ::std::mem::replace(req.headers_mut(), HeaderMap::new());
        state
    }

    pub fn set_body(&mut self, body: Bytes) {
        self.request_body = Some(body);
    }

    pub fn create_request<B: From<Bytes>>(&self) -> Result<Request<B>, Error> {
        Ok(Request::builder()
            .method(self.method.clone())
            .uri(self.uri.clone())
            .version(self.version)
            .body(self.request_body.clone().unwrap_or_default().into())?)
    }

    pub fn handle_response(&mut self, res: &Response<Body>) -> Result<StateMachineDecision, Error> {
        match res.status() {
            StatusCode::MOVED_PERMANENTLY | StatusCode::PERMANENT_REDIRECT => {
                self.follow_redirect(res)
            }
            StatusCode::FOUND | StatusCode::TEMPORARY_REDIRECT => self.follow_redirect(res),
            StatusCode::SEE_OTHER => {
                self.method = Method::GET;
                self.request_body = None;
                self.follow_redirect(res)
            }
            _ => Ok(StateMachineDecision::Return),
        }
    }

    fn follow_redirect(&mut self, res: &Response<Body>) -> Result<StateMachineDecision, Error> {
        if self.remaining_redirects == 0 {
            return Ok(StateMachineDecision::Return);
        }
        self.remaining_redirects -= 1;
        if let Some(location) = res.headers().get(hyper::header::LOCATION) {
            let next = self.uri.compute_redirect(location)?;
            remove_sensitive_headers(&mut self.headers, &next, &self.uri);
            self.uri = next;
            Ok(StateMachineDecision::Continue)
        } else {
            Ok(StateMachineDecision::Return)
        }
    }
}

pub fn remove_sensitive_headers(headers: &mut HeaderMap, next: &Uri, previous: &Uri) {
    if !next.is_same_host(previous) {
        headers.remove("authorization");
        headers.remove("cookie");
        headers.remove("cookie2");
        headers.remove("www-authenticate");
    }
}
