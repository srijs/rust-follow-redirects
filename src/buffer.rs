use std::error::Error as StdError;
use std::future::Future;
use std::pin::Pin;
use std::task::{self, Poll};

use bytes::{Bytes, BytesMut};
use hyper::body::{Buf, HttpBody};
use hyper::Request;

pub(crate) struct Buffer<B> {
    req: Request<B>,
    buf: BytesMut,
}

impl<B> From<Request<B>> for Buffer<B> {
    fn from(req: Request<B>) -> Buffer<B> {
        Buffer {
            req,
            buf: BytesMut::new(),
        }
    }
}

/// A macro for extracting the successful type of a `Poll<T, E>`.
///
/// This macro bakes in propagation of both errors and `Pending` signals by
/// returning early.
#[macro_export]
macro_rules! try_ready {
    ($e:expr) => {
        match $e {
            Poll::Ready(t) => t,
            Poll::Pending => return Poll::Pending,
        }
    };
}

impl<B> Future for Buffer<B>
where
    B: HttpBody + From<Bytes> + Send + Unpin + 'static,
    B::Data: Send,
    B::Error: Into<Box<dyn StdError + Send + Sync>>,
{
    type Output = Result<Bytes, B::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        let self_ref = self.get_mut();
        let body = self_ref.req.body_mut();
        loop {
            match try_ready!(HttpBody::poll_data(Pin::new(body), cx)) {
                Some(Ok(chunk)) => self_ref.buf.extend_from_slice(chunk.chunk()),
                Some(Err(e)) => return Poll::Ready(Err(e)),
                None => {
                    let buf = ::std::mem::replace(&mut self_ref.buf, BytesMut::new());
                    return Poll::Ready(Ok(buf.freeze()));
                }
            }
        }
    }
}
