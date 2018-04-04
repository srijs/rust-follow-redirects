use bytes::Bytes;
use hyper::{self, Error, Headers, HttpVersion, Method, Request, Response, StatusCode, Uri};

use ::uri::UriExt;

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
        if let Some(location) = res.headers().get::<hyper::header::Location>() {
            self.uri = self.uri.compute_redirect(location)?;
            Ok(StateMachineDecision::Continue)
        } else {
            Ok(StateMachineDecision::Return)
        }
    }
}
