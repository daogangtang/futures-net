//! # Async network TCP, UDP, UDS
//!
//! The types  are designed to closely follow the APIs of the
//! analogous types in `std::net` in `Asychronous` versions.
//!
//! # Examples
//! __TCP Server__
//! ```rust
//! use futures_net::{TcpListener, TcpStream};
//! use futures::prelude::*;
//!
//! async fn say_hello(mut stream: TcpStream) {
//!     stream.write_all(b"Shall I hear more, or shall I speak at this?").await;
//! }
//!
//! async fn listen() -> Result<(), Box<dyn std::error::Error + 'static>> {
//!     let socket_addr = "127.0.0.1:8080".parse()?;
//!     let mut listener = TcpListener::bind(&socket_addr)?;
//!     let mut incoming = listener.incoming();
//!
//!     // accept connections and process them serially
//!     while let Some(stream) = incoming.next().await {
//!         say_hello(stream?).await;
//!     }
//!     Ok(())
//! }
//! ```
//! __TCP Client__
//! ```rust,no_run
//! use std::error::Error;
//! use futures::prelude::*;
//! use futures_net::{TcpListener, TcpStream};
//!
//! async fn receive_sonnet() -> Result<(), Box<dyn Error + 'static>> {
//!     let socket_addr = "127.0.0.1:8080".parse()?;
//!     let mut buffer = vec![];
//!     let mut stream = TcpStream::connect(&socket_addr).await?;
//!
//!     stream.read(&mut buffer).await?;
//!     println!("{:?}", buffer);
//!     Ok(())
//! }
//! ```

#![warn(
    rust_2018_idioms,
    unreachable_pub,
    missing_debug_implementations,
    missing_docs,
)]
#![allow(
    warnings,
    missing_docs,
    type_alias_bounds,
    clippy::type_complexity,
    clippy::borrow_interior_mutable_const,
    clippy::needless_doctest_main,
    clippy::too_many_arguments,
    clippy::new_without_default
)]

#[cfg(feature = "macro")]
#[doc(inline)]
pub use futures_net_macro::{main, test};

pub mod runtime;
pub mod uds;
mod tcp;
mod udp;

pub(crate) mod driver;

#[doc(inline)]
pub use crate::tcp::{TcpListener, TcpStream};
#[doc(inline)]
pub use crate::udp::UdpSocket;
#[doc(inline)]
pub use crate::uds::{UnixDatagram, UnixListener, UnixStream};