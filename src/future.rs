use std::error::Error as StdError;
use std::future::Future;
use std::pin::Pin;
use std::task::{self, Poll};

use bytes::Bytes;
use hyper::body::{Body, HttpBody};
use hyper::client::connect::Connect;
use hyper::{self, Request, Response};

use crate::buffer::Buffer;
use crate::error::Error;
use crate::machine::{StateMachine, StateMachineDecision};

pub(crate) enum FutureInner<C, B> {
    Lazy(hyper::Client<C, B>, Request<B>, usize),
    Buffering(hyper::Client<C, B>, StateMachine, Buffer<B>),
    Requesting(hyper::Client<C, B>, StateMachine, hyper::client::ResponseFuture),
    Swapping,
}

impl<C, B> FutureInner<C, B> {
    pub fn new(
        client: hyper::Client<C, B>,
        req: Request<B>,
        max_redirects: usize,
    ) -> FutureInner<C, B> {
        FutureInner::Lazy(client, req, max_redirects)
    }
}

impl<C, B> Future for FutureInner<C, B>
where
    C: Connect + Clone + Send + Sync + Unpin + 'static,
    B: HttpBody + From<Bytes> + Send + Unpin + 'static,
    B::Data: Send,
    B::Error: Into<Box<dyn StdError + Send + Sync>>,
{
    type Output = Result<Response<Body>, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        let self_ref = self.get_mut();
        match ::std::mem::replace(self_ref, FutureInner::Swapping) {
            FutureInner::Lazy(client, req, max_redirects) => {
                self_ref.buffer(client, req, max_redirects, cx)
            }
            FutureInner::Buffering(client, mut state, mut buffer) => {
                match Future::poll(Pin::new(&mut buffer), cx) {
                    Poll::Ready(Ok(body)) => {
                        state.set_body(body);
                        self_ref.request(client, state, cx)
                    }
                    Poll::Ready(Err(e)) => {
                        return Poll::Ready(Err(Error::request(e)));
                    }
                    Poll::Pending => {
                        *self_ref = FutureInner::Buffering(client, state, buffer);
                        Poll::Pending
                    }
                }
            }
            FutureInner::Requesting(client, state, mut future) => {
                match Future::poll(Pin::new(&mut future), cx) {
                    Poll::Ready(Ok(response)) => self_ref.redirect(client, state, response, cx),
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e.into())),
                    Poll::Pending => {
                        *self_ref = FutureInner::Requesting(client, state, future);
                        Poll::Pending
                    }
                }
            }
            _ => unreachable!("one of the clauses above is not setting the state"),
        }
    }
}

impl<C, B> FutureInner<C, B>
where
    C: Connect + Clone + Send + Sync + Unpin + 'static,
    B: HttpBody + From<Bytes> + Send + Unpin + 'static,
    B::Data: Send,
    B::Error: Into<Box<dyn StdError + Send + Sync>>,
{
    fn buffer(
        &mut self,
        client: hyper::Client<C, B>,
        mut req: Request<B>,
        max_redirects: usize,
        cx: &mut task::Context<'_>,
    ) -> Poll<Result<Response<Body>, Error>> {
        let state = StateMachine::new(&mut req, max_redirects);
        let buffer = Buffer::from(req);
        *self = FutureInner::Buffering(client, state, buffer);
        Future::poll(Pin::new(self), cx)
    }

    fn request(
        &mut self,
        client: hyper::Client<C, B>,
        state: StateMachine,
        cx: &mut task::Context<'_>,
    ) -> Poll<Result<Response<Body>, Error>> {
        match state.create_request() {
            Ok(req) => {
                let future = client.request(req);
                *self = FutureInner::Requesting(client, state, future);
                Future::poll(Pin::new(self), cx)
            }
            Err(e) => Poll::Ready(Err(Error::request(e))),
        }
    }

    fn redirect(
        &mut self,
        client: hyper::Client<C, B>,
        mut state: StateMachine,
        res: Response<Body>,
        cx: &mut task::Context<'_>,
    ) -> Poll<Result<Response<Body>, Error>> {
        match state.handle_response(&res)? {
            StateMachineDecision::Continue => self.request(client, state, cx),
            StateMachineDecision::Return => Poll::Ready(Ok(res)),
        }
    }
}
