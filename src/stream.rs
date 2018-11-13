use futures::stream::{FuturesUnordered, StreamFuture};
use futures::{Async, Future, Poll, Stream};
use std::fmt;
use std::fmt::Debug;

pub fn run_forever<S: Stream<Item = (), Error = ()>>(
    stream: S,
) -> impl Future<Item = (), Error = ()> {
    stream
        .skip_while(|_| Ok(true))
        .into_future()
        .map(|_| ())
        .map_err(|_| ())
}

pub fn select_all<I>(streams: I) -> SelectAll<I::Item>
where
    I: IntoIterator,
    I::Item: Stream,
{
    let mut set = SelectAll::new();
    for stream in streams {
        set.push(stream);
    }
    return set;
}

pub struct SelectAll<S> {
    inner: FuturesUnordered<StreamFuture<S>>,
}

impl<T: Debug> Debug for SelectAll<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "SelectAll {{ ... }}")
    }
}
impl<S: Stream> SelectAll<S> {
    fn new() -> SelectAll<S> {
        SelectAll {
            inner: FuturesUnordered::new(),
        }
    }

    pub fn push(&mut self, stream: S) {
        self.inner.push(stream.into_future());
    }
}

impl<S: Stream> Stream for SelectAll<S> {
    type Item = S::Item;
    type Error = S::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            match self.inner.poll() {
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Ok(Async::Ready(Some((Some(item), remaining)))) => {
                    self.push(remaining);
                    return Ok(Async::Ready(Some(item)));
                }
                Err((err, remaining)) => {
                    self.push(remaining);
                    return Err(err);
                }
                Ok(Async::Ready(Some((None, _remaining)))) => {}
                Ok(Async::Ready(None)) => return Ok(Async::Ready(None)),
            }
        }
    }
}
