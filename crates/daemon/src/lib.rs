// Src: https://gitlab.redox-os.org/redox-os/base/-/blob/4c8727ed40dfd54264c590e4d8a5d3407c5df650/daemon/src/lib.rs
//
// MIT License
//
// Copyright (c) 2017 Redox OS
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

// Modifications:
// replace use of unstable feature `never_type`

use std::io::{self, PipeWriter, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::process::Command;

#[must_use = "Daemon::ready must be called"]
pub struct Daemon {
    write_pipe: PipeWriter,
}

mod never {
    use std::marker::PhantomData;

    pub type Never = <PhantomData<fn()->!> as Function>::Output;

    pub trait Function {
        type Output;
    }

    impl<T> Function for PhantomData<fn() -> T> {
        type Output = T;
    }

}

pub use never::Never;

impl Daemon {
    pub fn new<F: FnOnce(Daemon) -> Never>(f: F) -> ! {
        let write_pipe = unsafe {
            io::PipeWriter::from_raw_fd(std::env::var("INIT_NOTIFY").unwrap().parse().unwrap())
        };

        f(Daemon { write_pipe })
    }

    pub fn ready(mut self) {
        self.write_pipe.write_all(&[0]).unwrap();
    }

    pub fn spawn(mut cmd: Command) {
        let (mut read_pipe, write_pipe) = io::pipe().unwrap();

        // Pass pipe to child
        if unsafe { libc::fcntl(write_pipe.as_raw_fd(), libc::F_SETFD, 0) } == -1 {
            eprintln!(
                "daemon: failed to unset CLOEXEC flag for pipe: {}",
                io::Error::last_os_error()
            );
            return;
        }
        cmd.env("INIT_NOTIFY", format!("{}", write_pipe.as_raw_fd()));

        if let Err(err) = cmd.spawn() {
            eprintln!("daemon: failed to execute {cmd:?}: {err}");
            return;
        }
        drop(write_pipe);

        let mut data = [0];
        match read_pipe.read_exact(&mut data) {
            Ok(()) => {
                if data[0] != 0 {
                    eprintln!("daemon: {cmd:?} failed with {}", data[0]);
                }
            }
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                eprintln!("daemon: {cmd:?} exited without notifying readiness");
            }
            Err(err) => {
                eprintln!("daemon: failed to wait for {cmd:?}: {err}");
            }
        }
    }
}
