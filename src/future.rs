use bytes::Bytes;
use futures::{Async, Future, Poll, Stream};
use hyper::{self, Error, Request, Response};
use hyper::client::Connect;

use ::buffer::Buffer;
use ::machine::{StateMachine, StateMachineDecision};

pub(crate) enum FutureInner<C, B> {
    Lazy(hyper::Client<C, B>, Request<B>, usize),
    Buffering(hyper::Client<C, B>, StateMachine, Buffer<B>),
    Requesting(hyper::Client<C, B>, StateMachine, hyper::client::FutureResponse),
    Swapping
}

impl<C, B> FutureInner<C, B> {
    pub fn new(client: hyper::Client<C, B>, req: Request<B>, max_redirects: usize) -> FutureInner<C, B> {
        FutureInner::Lazy(client, req, max_redirects)
    }
}

impl<C, B> Future for FutureInner<C, B>
    where C: Connect,
          B: Stream<Error = Error> + From<Bytes> + 'static,
          B::Item: AsRef<[u8]>
{
    type Item = Response;
    type Error = Error;

    fn poll(&mut self) -> Poll<Response, Error> {
        match ::std::mem::replace(self, FutureInner::Swapping) {
            FutureInner::Lazy(client, req, max_redirects) => {
                self.buffer(client, req, max_redirects)
            },
            FutureInner::Buffering(client, mut state, mut future) => {
                let async = future.poll()?;
                if let Async::Ready(body) = async {
                    state.set_body(body);
                    self.request(client, state)
                } else {
                    *self = FutureInner::Buffering(client, state, future);
                    Ok(Async::NotReady)
                }
            },
            FutureInner::Requesting(client, state, mut future) => {
                let async = future.poll()?;
                if let Async::Ready(response) = async {
                    self.redirect(client, state, response)
                } else {
                    *self = FutureInner::Requesting(client, state, future);
                    Ok(Async::NotReady)
                }
            }
            _ => unreachable!()
        }
    }
}

impl<C, B> FutureInner<C, B>
    where C: Connect,
          B: Stream<Error = Error> + From<Bytes> + 'static,
          B::Item: AsRef<[u8]>,
{
    fn buffer(&mut self, client: hyper::Client<C, B>, mut req: Request<B>, max_redirects: usize) -> Poll<Response, Error> {
        let state = StateMachine::new(&mut req, max_redirects);
        let buffering = Buffer::from(req);
        *self = FutureInner::Buffering(client, state, buffering);
        self.poll()
    }

    fn request(&mut self, client: hyper::Client<C, B>, state: StateMachine) -> Poll<Response, Error> {
        let future = client.request(state.create_request());
        *self = FutureInner::Requesting(client, state, future);
        self.poll()
    }

    fn redirect(&mut self, client: hyper::Client<C, B>, mut state: StateMachine, res: Response) -> Poll<Response, Error> {
        match state.handle_response(&res)? {
            StateMachineDecision::Continue =>
                self.request(client, state),
            StateMachineDecision::Return =>
                Ok(Async::Ready(res))
        }
    }
}
