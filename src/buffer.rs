use bytes::{Bytes, BytesMut};
use futures::{Async, Future, Poll, Stream};
use hyper::Request;

pub(crate) struct Buffer<B> {
    req: Request<B>,
    buf: BytesMut
}

impl<B> From<Request<B>> for Buffer<B> {
    fn from(req: Request<B>) -> Buffer<B> {
        Buffer { req, buf: BytesMut::new() }
    }
}

impl<B> Future for Buffer<B>
    where B: Stream,
          B::Item: AsRef<[u8]>
{
    type Item = Option<Bytes>;
    type Error = B::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let &mut Some(ref mut body) = self.req.body_mut() {
            loop {
                if let Some(chunk) = try_ready!(body.poll()) {
                    self.buf.extend_from_slice(chunk.as_ref());
                } else {
                    let buf = ::std::mem::replace(&mut self.buf, BytesMut::new());
                    return Ok(Async::Ready(Some(buf.freeze())));
                }
            }
        } else {
            Ok(Async::Ready(None))
        }
    }
}
