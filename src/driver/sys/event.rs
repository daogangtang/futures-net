//! Readiness event types and Utility

use std::os::unix::io::RawFd;
use std::{fmt, io, ops};

pub use super::poll::{Events, Iter};
use super::{linux, poll};
use crate::driver::sys::{Poll, Token};

/// A value that may be registered with `Poll`
///
/// Values that implement `Evented` can be registered with `Poll`. Users of linux
/// should not use the `Evented` trait functions directly. Instead, the
/// equivalent functions on `Poll` should be used.
///
/// See [`Poll`] for more details.
///
/// # Implementing `Evented`
///
/// There are two types of `Evented` values.
///
/// * **System** handles, which are backed by sockets or other system handles.
/// These `Evented` handles will be monitored by the system selector. In this
/// case, an implementation of `Evented` delegates to a lower level handle.
///
/// * **User** handles, which are driven entirely in user space using
/// [`Registration`] and [`SetReadiness`]. In this case, the implementer takes
/// responsibility for driving the readiness state changes.
///
/// [`Poll`]: ../struct.Poll.html
/// [`Registration`]: ../struct.Registration.html
/// [`SetReadiness`]: ../struct.SetReadiness.html
///
/// # Examples
///
/// Implementing `Evented` on a struct containing a socket:
///
/// ```
/// use futures_net::driver::sys::{Poll, Token};
/// use futures_net::driver::sys::event::{Ready, PollOpt};
/// use futures_net::driver::sys::event::Evented;
/// use futures_net::driver::sys::net::TcpStream;
///
/// use std::io;
///
/// pub struct MyEvented {
///     socket: TcpStream,
/// }
///
/// impl Evented for MyEvented {
///     fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt)
///         -> io::Result<()>
///     {
///         // Delegate the `register` call to `socket`
///         self.socket.register(poll, token, interest, opts)
///     }
///
///     fn reregister(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt)
///         -> io::Result<()>
///     {
///         // Delegate the `reregister` call to `socket`
///         self.socket.reregister(poll, token, interest, opts)
///     }
///
///     fn deregister(&self, poll: &Poll) -> io::Result<()> {
///         // Delegate the `deregister` call to `socket`
///         self.socket.deregister(poll)
///     }
/// }
/// ```
///
/// Implement `Evented` using [`Registration`] and [`SetReadiness`].
///
/// ```
/// use futures_net::driver::sys::{Poll, Registration, Token};
/// use futures_net::driver::sys::event::{Events, Ready, PollOpt};
/// use futures_net::driver::sys::event::Evented;
///
/// use std::io;
/// use std::time::Instant;
/// use std::thread;
///
/// pub struct Deadline {
///     when: Instant,
///     registration: Registration,
/// }
///
/// impl Deadline {
///     pub fn new(when: Instant) -> Deadline {
///         let (registration, set_readiness) = Registration::new2();
///
///         thread::spawn(move || {
///             let now = Instant::now();
///
///             if now < when {
///                 thread::sleep(when - now);
///             }
///
///             set_readiness.set_readiness(Ready::readable());
///         });
///
///         Deadline {
///             when: when,
///             registration: registration,
///         }
///     }
///
///     pub fn is_elapsed(&self) -> bool {
///         Instant::now() >= self.when
///     }
/// }
///
/// impl Evented for Deadline {
///     fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt)
///         -> io::Result<()>
///     {
///         self.registration.register(poll, token, interest, opts)
///     }
///
///     fn reregister(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt)
///         -> io::Result<()>
///     {
///         self.registration.reregister(poll, token, interest, opts)
///     }
///
///     fn deregister(&self, poll: &Poll) -> io::Result<()> {
///         self.registration.deregister(poll)
///     }
/// }
/// ```
pub trait Evented {
    /// Register `self` with the given `Poll` instance.
    ///
    /// This function should not be called directly. Use [`Poll::register`]
    /// instead. Implementors should handle registration by either delegating
    /// the call to another `Evented` type or creating a [`Registration`].
    ///
    /// [`Poll::register`]: ../struct.Poll.html#method.register
    /// [`Registration`]: ../struct.Registration.html
    fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt)
        -> io::Result<()>;

    /// Re-register `self` with the given `Poll` instance.
    ///
    /// This function should not be called directly. Use [`Poll::reregister`]
    /// instead. Implementors should handle re-registration by either delegating
    /// the call to another `Evented` type or calling
    /// [`SetReadiness::set_readiness`].
    ///
    /// [`Poll::reregister`]: ../struct.Poll.html#method.reregister
    /// [`SetReadiness::set_readiness`]: ../struct.SetReadiness.html#method.set_readiness
    fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()>;

    /// Deregister `self` from the given `Poll` instance
    ///
    /// This function should not be called directly. Use [`Poll::deregister`]
    /// instead. Implementors should handle deregistration by either delegating
    /// the call to another `Evented` type or by dropping the [`Registration`]
    /// associated with `self`.
    ///
    /// [`Poll::deregister`]: ../struct.Poll.html#method.deregister
    /// [`Registration`]: ../struct.Registration.html
    fn deregister(&self, poll: &Poll) -> io::Result<()>;
}

impl Evented for Box<dyn Evented> {
    fn register(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        self.as_ref().register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        self.as_ref().reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        self.as_ref().deregister(poll)
    }
}

impl<T: Evented> Evented for Box<T> {
    fn register(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        self.as_ref().register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        self.as_ref().reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        self.as_ref().deregister(poll)
    }
}

impl<T: Evented> Evented for ::std::sync::Arc<T> {
    fn register(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        self.as_ref().register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        self.as_ref().reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        self.as_ref().deregister(poll)
    }
}

/*
 *
 * ===== EventedFd =====
 *
 */

#[derive(Debug)]

/// Adapter for [`RawFd`] providing an [`Evented`] implementation.
///
/// `EventedFd` enables registering any type with an FD with [`Poll`].
///
/// While only implementations for TCP and UDP are provided, Mio supports
/// registering any FD that can be registered with the underlying OS selector.
/// `EventedFd` provides the necessary bridge.
///
/// Note that `EventedFd` takes a `&RawFd`. This is because `EventedFd` **does
/// not** take ownership of the FD. Specifically, it will not manage any
/// lifecycle related operations, such as closing the FD on drop. It is expected
/// that the `EventedFd` is constructed right before a call to
/// [`Poll::register`]. See the examples for more detail.
///
/// # Examples
///
/// Basic usage
///
/// ```
/// # use std::error::Error;
/// # fn try_main() -> Result<(), Box<Error>> {
/// use futures_net::driver::sys::{Poll, Token};
/// use futures_net::driver::sys::event::{Ready, PollOpt};
/// use futures_net::driver::sys::event::EventedFd;
///
/// use std::os::unix::io::AsRawFd;
/// use std::net::TcpListener;
///
/// // Bind a std listener
/// let listener = TcpListener::bind("127.0.0.1:0")?;
///
/// let poll = Poll::new()?;
///
/// // Register the listener
/// poll.register(&EventedFd(&listener.as_raw_fd()),
///              Token(0), Ready::readable(), PollOpt::edge())?;
/// #     Ok(())
/// # }
/// #
/// # fn main() {
/// #     try_main().unwrap();
/// # }
/// ```
///
/// Implementing [`Evented`] for a custom type backed by a [`RawFd`].
///
/// ```
/// use futures_net::driver::sys::{Poll, Token};
/// use futures_net::driver::sys::event::{Evented, Ready, PollOpt,EventedFd} ;
///
/// use std::os::unix::io::RawFd;
/// use std::io;
///
/// pub struct MyIo {
///     fd: RawFd,
/// }
///
/// impl Evented for MyIo {
///     fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt)
///         -> io::Result<()>
///     {
///         EventedFd(&self.fd).register(poll, token, interest, opts)
///     }
///
///     fn reregister(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt)
///         -> io::Result<()>
///     {
///         EventedFd(&self.fd).reregister(poll, token, interest, opts)
///     }
///
///     fn deregister(&self, poll: &Poll) -> io::Result<()> {
///         EventedFd(&self.fd).deregister(poll)
///     }
/// }
/// ```
///
/// [`RawFd`]: https://doc.rust-lang.org/std/os/unix/io/type.RawFd.html
/// [`Evented`]: ../event/trait.Evented.html
/// [`Poll`]: ../struct.Poll.html
/// [`Poll::register`]: ../struct.Poll.html#method.register
pub struct EventedFd<'a>(pub &'a RawFd);

impl<'a> Evented for EventedFd<'a> {
    fn register(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        poll::selector(poll).register(*self.0, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        poll::selector(poll).reregister(*self.0, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        poll::selector(poll).deregister(*self.0)
    }
}

/// Options supplied when registering an `Evented` handle with `Poll`
///
/// `PollOpt` values can be combined together using the various bitwise
/// operators.
///
/// For high level documentation on polling and poll options, see [`Poll`].
///
/// # Examples
///
/// ```
/// use futures_net::driver::sys::event::PollOpt;
///
/// let opts = PollOpt::edge() | PollOpt::oneshot();
///
/// assert!(opts.is_edge());
/// assert!(opts.is_oneshot());
/// assert!(!opts.is_level());
/// ```
///
/// [`Poll`]: struct.Poll.html
#[derive(Copy, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct PollOpt(usize);

impl PollOpt {
    /// Return a `PollOpt` representing no set options.
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::PollOpt;
    ///
    /// let opt = PollOpt::empty();
    ///
    /// assert!(!opt.is_level());
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    #[inline]
    pub fn empty() -> PollOpt {
        PollOpt(0)
    }

    /// Return a `PollOpt` representing edge-triggered notifications.
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::PollOpt;
    ///
    /// let opt = PollOpt::edge();
    ///
    /// assert!(opt.is_edge());
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    #[inline]
    pub fn edge() -> PollOpt {
        PollOpt(0b0001)
    }

    /// Return a `PollOpt` representing level-triggered notifications.
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::PollOpt;
    ///
    /// let opt = PollOpt::level();
    ///
    /// assert!(opt.is_level());
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    #[inline]
    pub fn level() -> PollOpt {
        PollOpt(0b0010)
    }

    /// Return a `PollOpt` representing oneshot notifications.
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::PollOpt;
    ///
    /// let opt = PollOpt::oneshot();
    ///
    /// assert!(opt.is_oneshot());
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    #[inline]
    pub fn oneshot() -> PollOpt {
        PollOpt(0b0100)
    }

    #[doc(hidden)]
    #[inline]
    pub fn urgent() -> PollOpt {
        PollOpt(0b1000)
    }

    #[doc(hidden)]
    #[inline]
    pub fn all() -> PollOpt {
        PollOpt::edge() | PollOpt::level() | PollOpt::oneshot()
    }

    /// Returns true if the options include edge-triggered notifications.
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::PollOpt;
    ///
    /// let opt = PollOpt::edge();
    ///
    /// assert!(opt.is_edge());
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    #[inline]
    pub fn is_edge(&self) -> bool {
        self.contains(PollOpt::edge())
    }

    /// Returns true if the options include level-triggered notifications.
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::PollOpt;
    ///
    /// let opt = PollOpt::level();
    ///
    /// assert!(opt.is_level());
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    #[inline]
    pub fn is_level(&self) -> bool {
        self.contains(PollOpt::level())
    }

    /// Returns true if the options includes oneshot.
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::PollOpt;
    ///
    /// let opt = PollOpt::oneshot();
    ///
    /// assert!(opt.is_oneshot());
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    #[inline]
    pub fn is_oneshot(&self) -> bool {
        self.contains(PollOpt::oneshot())
    }

    #[doc(hidden)]
    #[allow(deprecated)]
    #[inline]
    pub fn is_urgent(&self) -> bool {
        self.contains(PollOpt::urgent())
    }

    #[doc(hidden)]
    #[inline]
    pub fn bits(&self) -> usize {
        self.0
    }

    /// Returns true if `self` is a superset of `other`.
    ///
    /// `other` may represent more than one option, in which case the function
    /// only returns true if `self` contains all of the options specified in
    /// `other`.
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::PollOpt;
    ///
    /// let opt = PollOpt::oneshot();
    ///
    /// assert!(opt.contains(PollOpt::oneshot()));
    /// assert!(!opt.contains(PollOpt::edge()));
    /// ```
    ///
    /// ```
    /// use futures_net::driver::sys::event::PollOpt;
    ///
    /// let opt = PollOpt::oneshot() | PollOpt::edge();
    ///
    /// assert!(opt.contains(PollOpt::oneshot()));
    /// assert!(opt.contains(PollOpt::edge()));
    /// ```
    ///
    /// ```
    /// use futures_net::driver::sys::event::PollOpt;
    ///
    /// let opt = PollOpt::oneshot() | PollOpt::edge();
    ///
    /// assert!(!PollOpt::oneshot().contains(opt));
    /// assert!(opt.contains(opt));
    /// assert!((opt | PollOpt::level()).contains(opt));
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    #[inline]
    pub fn contains(&self, other: PollOpt) -> bool {
        (*self & other) == other
    }

    /// Adds all options represented by `other` into `self`.
    ///
    /// This is equivalent to `*self = *self | other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::PollOpt;
    ///
    /// let mut opt = PollOpt::empty();
    /// opt.insert(PollOpt::oneshot());
    ///
    /// assert!(opt.is_oneshot());
    /// ```
    #[inline]
    pub fn insert(&mut self, other: PollOpt) {
        self.0 |= other.0;
    }

    /// Removes all options represented by `other` from `self`.
    ///
    /// This is equivalent to `*self = *self & !other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::PollOpt;
    ///
    /// let mut opt = PollOpt::oneshot();
    /// opt.remove(PollOpt::oneshot());
    ///
    /// assert!(!opt.is_oneshot());
    /// ```
    #[inline]
    pub fn remove(&mut self, other: PollOpt) {
        self.0 &= !other.0;
    }
}

impl ops::BitOr for PollOpt {
    type Output = PollOpt;

    #[inline]
    fn bitor(self, other: PollOpt) -> PollOpt {
        PollOpt(self.0 | other.0)
    }
}

impl ops::BitXor for PollOpt {
    type Output = PollOpt;

    #[inline]
    fn bitxor(self, other: PollOpt) -> PollOpt {
        PollOpt(self.0 ^ other.0)
    }
}

impl ops::BitAnd for PollOpt {
    type Output = PollOpt;

    #[inline]
    fn bitand(self, other: PollOpt) -> PollOpt {
        PollOpt(self.0 & other.0)
    }
}

impl ops::Sub for PollOpt {
    type Output = PollOpt;

    #[inline]
    fn sub(self, other: PollOpt) -> PollOpt {
        PollOpt(self.0 & !other.0)
    }
}

#[doc(hidden)]
impl ops::Not for PollOpt {
    type Output = PollOpt;

    #[inline]
    fn not(self) -> PollOpt {
        PollOpt(!self.0)
    }
}

impl fmt::Debug for PollOpt {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut one = false;
        let flags = [
            (PollOpt::edge(), "Edge-Triggered"),
            (PollOpt::level(), "Level-Triggered"),
            (PollOpt::oneshot(), "OneShot"),
        ];

        for &(flag, msg) in &flags {
            if self.contains(flag) {
                if one {
                    write!(fmt, " | ")?
                }
                write!(fmt, "{}", msg)?;

                one = true
            }
        }

        if !one {
            fmt.write_str("(empty)")?;
        }

        Ok(())
    }
}

#[test]
fn test_debug_pollopt() {
    assert_eq!("(empty)", format!("{:?}", PollOpt::empty()));
    assert_eq!("Edge-Triggered", format!("{:?}", PollOpt::edge()));
    assert_eq!("Level-Triggered", format!("{:?}", PollOpt::level()));
    assert_eq!("OneShot", format!("{:?}", PollOpt::oneshot()));
}

/// A set of readiness event kinds
///
/// `Ready` is a set of operation descriptors indicating which kind of an
/// operation is ready to be performed. For example, `Ready::readable()`
/// indicates that the associated `Evented` handle is ready to perform a
/// `read` operation.
///
/// This struct only represents portable event kinds. Since only readable and
/// writable events are guaranteed to be raised on all systems, those are the
/// only ones available via the `Ready` struct. There are also platform specific
/// extensions to `Ready`, i.e. `UnixReady`, which provide additional readiness
/// event kinds only available on unix platforms.
///
/// `Ready` values can be combined together using the various bitwise operators.
///
/// For high level documentation on polling and readiness, see [`Poll`].
///
/// # Examples
///
/// ```
/// use futures_net::driver::sys::event::Ready;
///
/// let ready = Ready::readable() | Ready::writable();
///
/// assert!(ready.is_readable());
/// assert!(ready.is_writable());
/// ```
///
/// [`Poll`]: struct.Poll.html
/// [`readable`]: #method.readable
/// [`writable`]: #method.writable
/// [readiness]: struct.Poll.html#readiness-operations
#[derive(Copy, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct Ready(usize);

const READABLE: usize = 0b00001;
const WRITABLE: usize = 0b00010;

// These are deprecated and are moved into platform specific implementations.
const ERROR: usize = 0b00100;
const HUP: usize = 0b01000;

impl Ready {
    /// Returns the empty `Ready` set.
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::Ready;
    ///
    /// let ready = Ready::empty();
    ///
    /// assert!(!ready.is_readable());
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    pub fn empty() -> Ready {
        Ready(0)
    }

    /// Returns a `Ready` representing readable readiness.
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::Ready;
    ///
    /// let ready = Ready::readable();
    ///
    /// assert!(ready.is_readable());
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    #[inline]
    pub fn readable() -> Ready {
        Ready(READABLE)
    }

    /// Returns a `Ready` representing writable readiness.
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::Ready;
    ///
    /// let ready = Ready::writable();
    ///
    /// assert!(ready.is_writable());
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    #[inline]
    pub fn writable() -> Ready {
        Ready(WRITABLE)
    }

    /// Returns a `Ready` representing readiness for all operations.
    ///
    /// This includes platform specific operations as well (`hup`, `aio`,
    /// `error`, `lio`, `pri`).
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::Ready;
    ///
    /// let ready = Ready::all();
    ///
    /// assert!(ready.is_readable());
    /// assert!(ready.is_writable());
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    #[inline]
    pub fn all() -> Ready {
        Ready(READABLE | WRITABLE | linux::READY_ALL)
    }

    /// Returns true if `Ready` is the empty set
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::Ready;
    ///
    /// let ready = Ready::empty();
    /// assert!(ready.is_empty());
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    #[inline]
    pub fn is_empty(&self) -> bool {
        *self == Ready::empty()
    }

    /// Returns true if the value includes readable readiness
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::Ready;
    ///
    /// let ready = Ready::readable();
    ///
    /// assert!(ready.is_readable());
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    #[inline]
    pub fn is_readable(&self) -> bool {
        self.contains(Ready::readable())
    }

    /// Returns true if the value includes writable readiness
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::Ready;
    ///
    /// let ready = Ready::writable();
    ///
    /// assert!(ready.is_writable());
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    #[inline]
    pub fn is_writable(&self) -> bool {
        self.contains(Ready::writable())
    }

    /// Adds all readiness represented by `other` into `self`.
    ///
    /// This is equivalent to `*self = *self | other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::Ready;
    ///
    /// let mut readiness = Ready::empty();
    /// readiness.insert(Ready::readable());
    ///
    /// assert!(readiness.is_readable());
    /// ```
    #[inline]
    pub fn insert<T: Into<Self>>(&mut self, other: T) {
        let other = other.into();
        self.0 |= other.0;
    }

    /// Removes all options represented by `other` from `self`.
    ///
    /// This is equivalent to `*self = *self & !other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::Ready;
    ///
    /// let mut readiness = Ready::readable();
    /// readiness.remove(Ready::readable());
    ///
    /// assert!(!readiness.is_readable());
    /// ```
    #[inline]
    pub fn remove<T: Into<Self>>(&mut self, other: T) {
        let other = other.into();
        self.0 &= !other.0;
    }

    #[doc(hidden)]
    #[inline]
    pub fn bits(&self) -> usize {
        self.0
    }

    /// Returns true if `self` is a superset of `other`.
    ///
    /// `other` may represent more than one readiness operations, in which case
    /// the function only returns true if `self` contains all readiness
    /// specified in `other`.
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::Ready;
    ///
    /// let readiness = Ready::readable();
    ///
    /// assert!(readiness.contains(Ready::readable()));
    /// assert!(!readiness.contains(Ready::writable()));
    /// ```
    ///
    /// ```
    /// use futures_net::driver::sys::event::Ready;
    ///
    /// let readiness = Ready::readable() | Ready::writable();
    ///
    /// assert!(readiness.contains(Ready::readable()));
    /// assert!(readiness.contains(Ready::writable()));
    /// ```
    ///
    /// ```
    /// use futures_net::driver::sys::event::Ready;
    ///
    /// let readiness = Ready::readable() | Ready::writable();
    ///
    /// assert!(!Ready::readable().contains(readiness));
    /// assert!(readiness.contains(readiness));
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    #[inline]
    pub fn contains<T: Into<Self>>(&self, other: T) -> bool {
        let other = other.into();
        (*self & other) == other
    }

    /// Create a `Ready` instance using the given `usize` representation.
    ///
    /// The `usize` representation must have been obtained from a call to
    /// `Ready::as_usize`.
    ///
    /// The `usize` representation must be treated as opaque. There is no
    /// guaranteed correlation between the returned value and platform defined
    /// constants. Also, there is no guarantee that the `usize` representation
    /// will remain constant across patch releases of Mio.
    ///
    /// This function is mainly provided to allow the caller to loa a
    /// readiness value from an `AtomicUsize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::Ready;
    ///
    /// let ready = Ready::readable();
    /// let ready_usize = ready.as_usize();
    /// let ready2 = Ready::from_usize(ready_usize);
    ///
    /// assert_eq!(ready, ready2);
    /// ```
    pub fn from_usize(val: usize) -> Ready {
        Ready(val)
    }

    /// Returns a `usize` representation of the `Ready` value.
    ///
    /// This `usize` representation must be treated as opaque. There is no
    /// guaranteed correlation between the returned value and platform defined
    /// constants. Also, there is no guarantee that the `usize` representation
    /// will remain constant across patch releases of Mio.
    ///
    /// This function is mainly provided to allow the caller to store a
    /// readiness value in an `AtomicUsize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::event::Ready;
    ///
    /// let ready = Ready::readable();
    /// let ready_usize = ready.as_usize();
    /// let ready2 = Ready::from_usize(ready_usize);
    ///
    /// assert_eq!(ready, ready2);
    /// ```
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

impl<T: Into<Ready>> ops::BitOr<T> for Ready {
    type Output = Ready;

    #[inline]
    fn bitor(self, other: T) -> Ready {
        Ready(self.0 | other.into().0)
    }
}

impl<T: Into<Ready>> ops::BitOrAssign<T> for Ready {
    #[inline]
    fn bitor_assign(&mut self, other: T) {
        self.0 |= other.into().0;
    }
}

impl<T: Into<Ready>> ops::BitXor<T> for Ready {
    type Output = Ready;

    #[inline]
    fn bitxor(self, other: T) -> Ready {
        Ready(self.0 ^ other.into().0)
    }
}

impl<T: Into<Ready>> ops::BitXorAssign<T> for Ready {
    #[inline]
    fn bitxor_assign(&mut self, other: T) {
        self.0 ^= other.into().0;
    }
}

impl<T: Into<Ready>> ops::BitAnd<T> for Ready {
    type Output = Ready;

    #[inline]
    fn bitand(self, other: T) -> Ready {
        Ready(self.0 & other.into().0)
    }
}

impl<T: Into<Ready>> ops::BitAndAssign<T> for Ready {
    #[inline]
    fn bitand_assign(&mut self, other: T) {
        self.0 &= other.into().0
    }
}

impl<T: Into<Ready>> ops::Sub<T> for Ready {
    type Output = Ready;

    #[inline]
    fn sub(self, other: T) -> Ready {
        Ready(self.0 & !other.into().0)
    }
}

impl<T: Into<Ready>> ops::SubAssign<T> for Ready {
    #[inline]
    fn sub_assign(&mut self, other: T) {
        self.0 &= !other.into().0;
    }
}

#[doc(hidden)]
impl ops::Not for Ready {
    type Output = Ready;

    #[inline]
    fn not(self) -> Ready {
        Ready(!self.0)
    }
}

impl fmt::Debug for Ready {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut one = false;
        let flags = [
            (Ready::readable(), "Readable"),
            (Ready::writable(), "Writable"),
            (Ready(ERROR), "Error"),
            (Ready(HUP), "Hup"),
        ];

        for &(flag, msg) in &flags {
            if self.contains(flag) {
                if one {
                    write!(fmt, " | ")?
                }
                write!(fmt, "{}", msg)?;

                one = true
            }
        }

        if !one {
            fmt.write_str("(empty)")?;
        }

        Ok(())
    }
}

#[test]
fn test_debug_ready() {
    assert_eq!("(empty)", format!("{:?}", Ready::empty()));
    assert_eq!("Readable", format!("{:?}", Ready::readable()));
    assert_eq!("Writable", format!("{:?}", Ready::writable()));
}

/// An readiness event returned by [`Poll::poll`].
///
/// `Event` is a [readiness state] paired with a [`Token`]. It is returned by
/// [`Poll::poll`].
///
/// For more documentation on polling and events, see [`Poll`].
///
/// # Examples
///
/// ```
/// use futures_net::driver::sys::Token;
/// use futures_net::driver::sys::event::{Ready, Event};
///
/// let event = Event::new(Ready::readable() | Ready::writable(), Token(0));
///
/// assert_eq!(event.readiness(), Ready::readable() | Ready::writable());
/// assert_eq!(event.token(), Token(0));
/// ```
///
/// [`Poll::poll`]: ../struct.Poll.html#method.poll
/// [`Poll`]: ../struct.Poll.html
/// [readiness state]: ../struct.Ready.html
/// [`Token`]: ../struct.Token.html
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Event {
    kind: Ready,
    token: Token,
}

impl Event {
    /// Creates a new `Event` containing `readiness` and `token`
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::Token;
    /// use futures_net::driver::sys::event::{Ready, Event};
    ///
    /// let event = Event::new(Ready::readable() | Ready::writable(), Token(0));
    ///
    /// assert_eq!(event.readiness(), Ready::readable() | Ready::writable());
    /// assert_eq!(event.token(), Token(0));
    /// ```
    pub fn new(readiness: Ready, token: Token) -> Event {
        Event {
            kind: readiness,
            token,
        }
    }

    /// Returns the event's readiness.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::Token;
    /// use futures_net::driver::sys::event::{Event, Ready};
    ///
    /// let event = Event::new(Ready::readable() | Ready::writable(), Token(0));
    ///
    /// assert_eq!(event.readiness(), Ready::readable() | Ready::writable());
    /// ```
    pub fn readiness(&self) -> Ready {
        self.kind
    }

    /// Returns the event's token.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::Token;
    /// use futures_net::driver::sys::event::{Ready, Event};
    ///
    /// let event = Event::new(Ready::readable() | Ready::writable(), Token(0));
    ///
    /// assert_eq!(event.token(), Token(0));
    /// ```
    pub fn token(&self) -> Token {
        self.token
    }
}

/*
 *
 * ===== Mio internal helpers =====
 *
 */

pub fn ready_as_usize(events: Ready) -> usize {
    events.0
}

pub fn opt_as_usize(opt: PollOpt) -> usize {
    opt.0
}

pub fn ready_from_usize(events: usize) -> Ready {
    Ready(events)
}

pub fn opt_from_usize(opt: usize) -> PollOpt {
    PollOpt(opt)
}

// Used internally to mutate an `Event` in place
#[allow(dead_code)]
pub fn kind_mut(event: &mut Event) -> &mut Ready {
    &mut event.kind
}
