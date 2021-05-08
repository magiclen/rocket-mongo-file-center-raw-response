use std::io::{self, ErrorKind, Read};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::rocket::tokio::io::{AsyncRead, ReadBuf};

pub(crate) struct AsyncReader<R>(R);

impl<R: Read + Unpin> AsyncReader<R> {
    #[inline]
    pub(crate) fn from(reader: R) -> Self {
        Self(reader)
    }
}

impl<R: Read + Unpin> AsyncRead for AsyncReader<R> {
    #[inline]
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<(), io::Error>> {
        match self.0.read(buf.initialize_unfilled()) {
            Ok(c) => {
                buf.advance(c);

                Poll::Ready(Ok(()))
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}
