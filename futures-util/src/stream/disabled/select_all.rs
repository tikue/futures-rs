//! An unbounded set of streams

use std::fmt::{self, Debug};
use std::pin::PinMut;

use futures_core::{Poll, Stream};
use futures_core::task;

use stream::{StreamExt, StreamFuture, FuturesUnordered};

/// An unbounded set of streams
///
/// This "combinator" provides the ability to maintain a set of streams
/// and drive them all to completion.
///
/// Streams are pushed into this set and their realized values are
/// yielded as they become ready. Streams will only be polled when they
/// generate notifications. This allows to coordinate a large number of streams.
///
/// Note that you can create a ready-made `SelectAll` via the
/// `select_all` function in the `stream` module, or you can start with an
/// empty set with the `SelectAll::new` constructor.
#[must_use = "streams do nothing unless polled"]
pub struct SelectAll<St> {
    inner: FuturesUnordered<StreamFuture<St>>,
}

impl<St: Debug> Debug for SelectAll<St> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "SelectAll {{ ... }}")
    }
}

impl<St: Stream> SelectAll<St> {
    /// Constructs a new, empty `SelectAll`
    ///
    /// The returned `SelectAll` does not contain any streams and, in this
    /// state, `SelectAll::poll` will return `Ok(Async::Ready(None))`.
    pub fn new() -> SelectAll<St> {
        SelectAll { inner: FuturesUnordered::new() }
    }

    /// Returns the number of streams contained in the set.
    ///
    /// This represents the total number of in-flight streams.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the set contains no streams
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Push a stream into the set.
    ///
    /// This function submits the given stream to the set for managing. This
    /// function will not call `poll` on the submitted stream. The caller must
    /// ensure that `SelectAll::poll` is called in order to receive task
    /// notifications.
    pub fn push(&mut self, stream: S) {
        self.inner.push(stream.next());
    }
}

impl<St: Stream> Stream for SelectAll<St> {
    type Item = S::Item;
    type Error = S::Error;

    fn poll_next(
        &mut self,
        cx: &mut task::Context,
    ) -> Poll<Option<Self::Item>, Self::Error> {
        match self.inner.poll_next(cx).map_err(|(err, _)| err)? {
            Async::Pending => Ok(Async::Pending),
            Async::Ready(Some((Some(item), remaining))) => {
                self.push(remaining);
                Ok(Async::Ready(Some(item)))
            }
            Async::Ready(_) => Ok(Async::Ready(None)),
        }
    }
}

/// Convert a list of streams into a `Stream` of results from the streams.
///
/// This essentially takes a list of streams (e.g. a vector, an iterator, etc.)
/// and bundles them together into a single stream.
/// The stream will yield items as they become available on the underlying
/// streams internally, in the order they become available.
///
/// Note that the returned set can also be used to dynamically push more
/// futures into the set as they become available.
pub fn select_all<I>(streams: I) -> SelectAll<I::Item>
    where I: IntoIterator,
          I::Item: Stream
{
    let mut set = SelectAll::new();

    for stream in streams {
        set.push(stream);
    }

    return set
}
