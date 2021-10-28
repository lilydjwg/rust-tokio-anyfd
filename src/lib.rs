use std::io::{Result, Error};
use std::os::unix::io::AsRawFd;

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::ready;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::io::unix::AsyncFd;
use tokio::io::ReadBuf;

pub struct Anyfd<T: AsRawFd> {
  afd: AsyncFd<T>,
}

/// Wrap any suitable file descriptor `fd` as
/// [`AsyncRead`] and [`AsyncWrite`].
///
/// You need to make sure the file descriptor is
/// non-blocking. Set it with [`set_nonblocking`] if not
/// already.
///
/// [`AsyncRead`]: ../tokio/io/trait.AsyncRead.html
/// [`AsyncWrite`]: ../tokio/io/trait.AsyncWrite.html
/// [`set_nonblocking`]: fn.set_nonblocking.html
pub fn anyfd<T: AsRawFd>(fd: T) -> Result<Anyfd<T>> {
  Ok(Anyfd { afd: AsyncFd::new(fd)? })
}

/// Set `fd` as non-blocking (the [`O_NONBLOCK`] flag).
///
/// [`O_NONBLOCK`]: ../libc/constant.O_NONBLOCK.html
pub fn set_nonblocking(fd: impl AsRawFd) -> Result<()> {
  let fd = fd.as_raw_fd();
  unsafe {
    let mut flags = libc::fcntl(fd, libc::F_GETFL);
    if flags < 0 {
      return Err(Error::last_os_error());
    }
    flags |= libc::O_NONBLOCK;
    let r = libc::fcntl(fd, libc::F_SETFL, flags);
    if r < 0 {
      return Err(Error::last_os_error());
    }
  }
  Ok(())
}

impl<T: AsRawFd> AsyncRead for Anyfd<T> {
  fn poll_read(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &mut ReadBuf<'_>
  ) -> Poll<Result<()>> {
    let fd = self.afd.as_raw_fd();
    loop {
      let mut guard = ready!(self.afd.poll_read_ready(cx))?;

      match guard.try_io(|_| {
        let r = unsafe {
          let unfilled = buf.unfilled_mut();
          libc::read(fd, unfilled.as_ptr() as *mut _, unfilled.len())
        };
        if r < 0 {
          let err = Error::last_os_error();
          Err(err)
        } else {
          unsafe { buf.assume_init(r as usize) };
          buf.advance(r as usize);
          Ok(())
        }
      }) {
        Ok(result) => return Poll::Ready(result),
        Err(_would_block) => continue,
      }
    }
  }
}

impl<T: AsRawFd> AsyncWrite for Anyfd<T> {
  fn poll_write(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &[u8]
  ) -> Poll<Result<usize>> {
    let fd = self.afd.as_raw_fd();
    loop {
      let mut guard = ready!(self.afd.poll_write_ready(cx))?;

      match guard.try_io(|_| {
        let r = unsafe {
          libc::write(fd, buf.as_ptr() as *const _, buf.len())
        };
        if r < 0 {
          let err = Error::last_os_error();
          Err(err)
        } else {
          Ok(r as usize)
        }
      }) {
        Ok(result) => return Poll::Ready(result),
        Err(_would_block) => continue,
      }
    }
  }

  fn poll_flush(
    self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
  ) -> Poll<Result<()>> {
    Poll::Ready(Ok(()))
  }

  fn poll_shutdown(
    self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
  ) -> Poll<Result<()>> {
    let fd = self.afd.as_raw_fd();
    let r = unsafe {
      libc::shutdown(fd, libc::SHUT_WR)
    };
    if r == 0 {
      Poll::Ready(Ok(()))
    } else {
      Poll::Ready(Err(Error::last_os_error()))
    }
  }
}
