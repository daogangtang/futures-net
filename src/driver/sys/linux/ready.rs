use crate::driver::sys::event::{ready_as_usize, ready_from_usize, Ready};

use std::fmt;
use std::ops;

/// Unix specific extensions to `Ready`
///
/// Provides additional readiness event kinds that are available on unix
/// platforms. Unix platforms are able to provide readiness events for
/// additional socket events, such as HUP and error.
///
/// HUP events occur when the remote end of a socket hangs up. In the TCP case,
/// this occurs when the remote end of a TCP socket shuts down writes.
///
/// Error events occur when the socket enters an error state. In this case, the
/// socket will also receive a readable or writable event. Reading or writing to
/// the socket will result in an error.
///
/// Conversion traits are implemented between `Ready` and `UnixReady`. See the
/// examples.
///
/// For high level documentation on polling and readiness, see [`Poll`].
///
/// # Examples
///
/// Most of the time, all that is needed is using bit operations
///
/// ```
/// use futures_net::driver::sys::event::Ready;
/// use futures_net::driver::sys::UnixReady;
///
/// let ready = Ready::readable() | UnixReady::hup();
///
/// assert!(ready.is_readable());
/// assert!(UnixReady::from(ready).is_hup());
/// ```
///
/// Basic conversion between ready types.
///
/// ```
/// use futures_net::driver::sys::event::Ready;
/// use futures_net::driver::sys::UnixReady;
///
/// // Start with a portable ready
/// let ready = Ready::readable();
///
/// // Convert to a unix ready, adding HUP
/// let mut unix_ready = UnixReady::from(ready) | UnixReady::hup();
///
/// unix_ready.insert(UnixReady::error());
///
/// // `unix_ready` maintains readable interest
/// assert!(unix_ready.is_readable());
/// assert!(unix_ready.is_hup());
/// assert!(unix_ready.is_error());
///
/// // Convert back to `Ready`
/// let ready = Ready::from(unix_ready);
///
/// // Readable is maintained
/// assert!(ready.is_readable());
/// ```
///
/// Registering readable and error interest on a socket
///
/// ```
/// # use std::error::Error;
/// # fn try_main() -> Result<(), Box<Error>> {
/// use futures_net::driver::sys::{Poll, Token};
/// use futures_net::driver::sys::event::{Ready, PollOpt};
/// use futures_net::driver::sys::net::TcpStream;
/// use futures_net::driver::sys::UnixReady;
///
/// let addr = "216.58.193.68:80".parse()?;
/// let socket = TcpStream::connect(&addr)?;
///
/// let poll = Poll::new()?;
///
/// poll.register(&socket,
///               Token(0),
///               Ready::readable() | UnixReady::error(),
///               PollOpt::edge())?;
/// #     Ok(())
/// # }
/// #
/// # fn main() {
/// #     try_main().unwrap();
/// # }
/// ```
///
/// [`Poll`]: ../struct.Poll.html
/// [readiness]: struct.Poll.html#readiness-operations
#[derive(Copy, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct UnixReady(Ready);

const ERROR: usize = 0b00_0100;
const HUP: usize = 0b00_1000;

const LIO: usize = 0b00_0000;

const PRI: usize = 0b100_0000;

// Export to support `Ready::all`
pub const READY_ALL: usize = ERROR | HUP  | LIO | PRI;

#[test]
fn test_ready_all() {
    let readable = Ready::readable().as_usize();
    let writable = Ready::writable().as_usize();

    assert_eq!(
        READY_ALL | readable | writable,
        ERROR + HUP + LIO + PRI + readable + writable
    );

    assert!(!Ready::from(UnixReady::priority()).is_writable());
}

impl UnixReady {

    /// Returns a `Ready` representing error readiness.
    ///
    /// **Note that only readable and writable readiness is guaranteed to be
    /// supported on all platforms**. This means that `error` readiness
    /// should be treated as a hint. For more details, see [readiness] in the
    /// poll documentation.
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::UnixReady;
    ///
    /// let ready = UnixReady::error();
    ///
    /// assert!(ready.is_error());
    /// ```
    ///
    /// [`Poll`]: ../struct.Poll.html
    /// [readiness]: ../struct.Poll.html#readiness-operations
    #[inline]
    pub fn error() -> UnixReady {
        UnixReady(ready_from_usize(ERROR))
    }

    /// Returns a `Ready` representing HUP readiness.
    ///
    /// A HUP (or hang-up) signifies that a stream socket **peer** closed the
    /// connection, or shut down the writing half of the connection.
    ///
    /// **Note that only readable and writable readiness is guaranteed to be
    /// supported on all platforms**. This means that `hup` readiness
    /// should be treated as a hint. For more details, see [readiness] in the
    /// poll documentation. It is also unclear if HUP readiness will remain in 0.7. See
    /// [here][issue-941].
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::UnixReady;
    ///
    /// let ready = UnixReady::hup();
    ///
    /// assert!(ready.is_hup());
    /// ```
    ///
    /// [`Poll`]: ../struct.Poll.html
    /// [readiness]: ../struct.Poll.html#readiness-operations
    #[inline]
    pub fn hup() -> UnixReady {
        UnixReady(ready_from_usize(HUP))
    }

    /// Returns a `Ready` representing priority (`EPOLLPRI`) readiness
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::UnixReady;
    ///
    /// let ready = UnixReady::priority();
    ///
    /// assert!(ready.is_priority());
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    pub fn priority() -> UnixReady {
        UnixReady(ready_from_usize(PRI))
    }

    /// Returns true if the value includes error readiness
    ///
    /// **Note that only readable and writable readiness is guaranteed to be
    /// supported on all platforms**. This means that `error` readiness should
    /// be treated as a hint. For more details, see [readiness] in the poll
    /// documentation.
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::UnixReady;
    ///
    /// let ready = UnixReady::error();
    ///
    /// assert!(ready.is_error());
    /// ```
    ///
    /// [`Poll`]: ../struct.Poll.html
    /// [readiness]: ../struct.Poll.html#readiness-operations
    #[inline]
    pub fn is_error(&self) -> bool {
        self.contains(ready_from_usize(ERROR))
    }

    /// Returns true if the value includes HUP readiness
    ///
    /// A HUP (or hang-up) signifies that a stream socket **peer** closed the
    /// connection, or shut down the writing half of the connection.
    ///
    /// **Note that only readable and writable readiness is guaranteed to be
    /// supported on all platforms**. This means that `hup` readiness
    /// should be treated as a hint. For more details, see [readiness] in the
    /// poll documentation.
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::UnixReady;
    ///
    /// let ready = UnixReady::hup();
    ///
    /// assert!(ready.is_hup());
    /// ```
    ///
    /// [`Poll`]: ../struct.Poll.html
    /// [readiness]: ../struct.Poll.html#readiness-operations
    #[inline]
    pub fn is_hup(&self) -> bool {
        self.contains(ready_from_usize(HUP))
    }

    /// Returns true if `Ready` contains priority (`EPOLLPRI`) readiness
    ///
    /// See [`Poll`] for more documentation on polling.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_net::driver::sys::UnixReady;
    ///
    /// let ready = UnixReady::priority();
    ///
    /// assert!(ready.is_priority());
    /// ```
    ///
    /// [`Poll`]: struct.Poll.html
    #[inline]
    pub fn is_priority(&self) -> bool {
        self.contains(ready_from_usize(PRI))
    }
}

impl From<Ready> for UnixReady {
    fn from(src: Ready) -> UnixReady {
        UnixReady(src)
    }
}

impl From<UnixReady> for Ready {
    fn from(src: UnixReady) -> Ready {
        src.0
    }
}

impl ops::Deref for UnixReady {
    type Target = Ready;

    fn deref(&self) -> &Ready {
        &self.0
    }
}

impl ops::DerefMut for UnixReady {
    fn deref_mut(&mut self) -> &mut Ready {
        &mut self.0
    }
}

impl ops::BitOr for UnixReady {
    type Output = UnixReady;

    #[inline]
    fn bitor(self, other: UnixReady) -> UnixReady {
        (self.0 | other.0).into()
    }
}

impl ops::BitXor for UnixReady {
    type Output = UnixReady;

    #[inline]
    fn bitxor(self, other: UnixReady) -> UnixReady {
        (self.0 ^ other.0).into()
    }
}

impl ops::BitAnd for UnixReady {
    type Output = UnixReady;

    #[inline]
    fn bitand(self, other: UnixReady) -> UnixReady {
        (self.0 & other.0).into()
    }
}

impl ops::Sub for UnixReady {
    type Output = UnixReady;

    #[inline]
    fn sub(self, other: UnixReady) -> UnixReady {
        ready_from_usize(ready_as_usize(self.0) & !ready_as_usize(other.0)).into()
    }
}

#[doc(hidden)]
impl ops::Not for UnixReady {
    type Output = UnixReady;

    #[inline]
    fn not(self) -> UnixReady {
        (!self.0).into()
    }
}

impl fmt::Debug for UnixReady {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut one = false;
        let flags = [
            (UnixReady(Ready::readable()), "Readable"),
            (UnixReady(Ready::writable()), "Writable"),
            (UnixReady::error(), "Error"),
            (UnixReady::hup(), "Hup"),
            (UnixReady::priority(), "Priority"),
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
