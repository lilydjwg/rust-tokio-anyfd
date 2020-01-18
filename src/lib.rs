use std::io;
use std::os::unix::io::RawFd;

use mio::{Ready, Poll, PollOpt, Token};
use mio::event::Evented;
use mio::unix::EventedFd;
use tokio::io::PollEvented;

pub struct Anyfd {
  fd: RawFd,
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
pub fn anyfd(fd: RawFd) -> io::Result<PollEvented<Anyfd>> {
  let io = Anyfd { fd };
  PollEvented::new(io)
}

/// Set `fd` as non-blocking (the [`O_NONBLOCK`] flag).
///
/// [`O_NONBLOCK`]: ../libc/constant.O_NONBLOCK.html
pub fn set_nonblocking(fd: RawFd) -> io::Result<()> {
  unsafe {
    let mut flags = libc::fcntl(fd, libc::F_GETFL);
    if flags < 0 {
      return Err(io::Error::last_os_error());
    }
    flags |= libc::O_NONBLOCK;
    let r = libc::fcntl(fd, libc::F_SETFL, flags);
    if r < 0 {
      return Err(io::Error::last_os_error());
    }
  }
  Ok(())
}

impl Evented for Anyfd {
  fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt)
    -> io::Result<()>
  {
    EventedFd(&self.fd).register(poll, token, interest, opts)
  }

  fn reregister(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt)
    -> io::Result<()>
  {
    EventedFd(&self.fd).reregister(poll, token, interest, opts)
  }

  fn deregister(&self, poll: &Poll) -> io::Result<()> {
    EventedFd(&self.fd).deregister(poll)
  }
}

impl io::Read for Anyfd {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    let r = unsafe {
      libc::read(self.fd, buf.as_mut_ptr() as *mut _, buf.len())
    };
    if r < 0 {
      Err(io::Error::last_os_error())
    } else {
      Ok(r as usize)
    }
  }
}

impl io::Write for Anyfd {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    let r = unsafe {
      libc::write(self.fd, buf.as_ptr() as *const _, buf.len())
    };
    if r < 0 {
      Err(io::Error::last_os_error())
    } else {
      Ok(r as usize)
    }
  }

  fn flush(&mut self) -> io::Result<()> {
    Ok(())
  }
}
