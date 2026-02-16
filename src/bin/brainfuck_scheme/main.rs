
use std::collections::BTreeMap;
use std::num::Wrapping;
use std::sync::mpsc::TryRecvError;

use brainfuck_redox_scheme::brainfuck::BrainfuckInterpreter;

use daemon::Daemon;
use redox_scheme::scheme::SchemeSync;
use redox_scheme::{CallerCtx, OpenResult, SignalBehavior, Socket};
use syscall::error::*;
use syscall::schemev2::NewFdFlags;

#[cfg(test)]
mod test;

// based on
// - https://doc.redox-os.org/book/example.html and
// - https://gitlab.redox-os.org/redox-os/base/-/blob/4c8727ed40dfd54264c590e4d8a5d3407c5df650/randd/src/main.rs

fn main() {
    daemon::Daemon::new(daemon);
}

fn daemon(daemon: Daemon) -> ! {
    let mut scheme = BrainfuckScheme::new();
    let socket = Socket::create().expect("brainfuck: failed to create socket for brainfuck scheme");

    redox_scheme::scheme::register_sync_scheme(&socket, "brainfuck", &mut scheme)
        .expect("brainfuck: failed to register scheme namespace");

    daemon.ready();

    //TODO: should we enter the null namespace as randd does?

    while let Some(request) = socket
        .next_request(SignalBehavior::Restart)
        .expect("brainfuck: failed to read request")
    {
        match request.kind() {
            redox_scheme::RequestKind::Call(call_request) => {
                let response = call_request.handle_sync(&mut scheme);
                socket
                    .write_response(response, SignalBehavior::Restart)
                    .expect("brainfuck: failed to write response");
            }
            redox_scheme::RequestKind::OnClose { id } => scheme.on_close(id),
            _ => {}
        }
    }

    std::process::exit(0)
}

enum SchemeEntry {
    SchemeRoot,
    Brainfuck(BrainfuckEntry),
}
impl SchemeEntry {
    fn as_brainfuck(&self) -> Result<&BrainfuckEntry, Error> {
        match self {
            SchemeEntry::SchemeRoot => Err(Error::new(EBADFD)),
            SchemeEntry::Brainfuck(brainfuck_entry) => Ok(brainfuck_entry),
        }
    }

    fn as_brainfuck_mut(&mut self) -> Result<&mut BrainfuckEntry, Error> {
        match self {
            SchemeEntry::SchemeRoot => Err(Error::new(EBADFD)),
            SchemeEntry::Brainfuck(brainfuck_entry) => Ok(brainfuck_entry),
        }
    }
}

struct BrainfuckEntry {
    input_channel: std::sync::mpsc::Sender<u8>,
    output_channel: std::sync::mpsc::Receiver<u8>,
}

struct BrainfuckScheme {
    next_id: Wrapping<usize>,
    handles: BTreeMap<usize, SchemeEntry>,
}

impl BrainfuckScheme {
    fn new() -> BrainfuckScheme {
        BrainfuckScheme {
            next_id: Wrapping(0),
            handles: BTreeMap::new(),
        }
    }

    fn next_id(&mut self) -> usize {
        while self.handles.contains_key(&self.next_id.0) {
            self.next_id += 1;
        }

        let next = self.next_id.0;

        self.next_id += 1;

        next
    }
}

impl SchemeSync for BrainfuckScheme {
    fn scheme_root(&mut self) -> Result<usize> {
        let id = self.next_id();
        self.handles.insert(id, SchemeEntry::SchemeRoot);
        Ok(id)
    }

    fn openat(
        &mut self,
        dir_fd: usize,
        path: &str,
        _flags: usize,
        _fcntl_flags: u32,
        _ctx: &CallerCtx,
    ) -> Result<OpenResult> {
        if !matches!(
            self.handles.get(&dir_fd).ok_or(Error::new(EBADFD))?,
            SchemeEntry::SchemeRoot
        ) {
            return Err(Error::new(EACCES));
        }

        let program_code: Vec<char> = path.chars().collect();

        let (input_sender, input_receiver) = std::sync::mpsc::channel();
        let (output_sender, output_receiver) = std::sync::mpsc::channel();

        let _join_handle = std::thread::spawn(move || {
            BrainfuckInterpreter::new(program_code).run_brain_fuck(input_receiver, output_sender)
        });

        let entry = BrainfuckEntry {
            input_channel: input_sender,
            output_channel: output_receiver,
        };
        let id = self.next_id();

        self.handles.insert(id, SchemeEntry::Brainfuck(entry));
        Ok(OpenResult::ThisScheme {
            number: id,
            flags: NewFdFlags::empty(),
        })
    }

    fn read(
        &mut self,
        fd: usize,
        buf: &mut [u8],
        _offset: u64,
        _fcntl_flags: u32,
        _ctx: &CallerCtx,
    ) -> Result<usize> {
        let entry = self
            .handles
            .get_mut(&fd)
            .ok_or(Error::new(EBADFD))?
            .as_brainfuck_mut()?;

        let mut i = 0;
        while i < buf.len() {
            match entry.output_channel.try_recv() {
                Ok(msg) => {
                    buf[i] = msg;
                    i += 1;
                }
                Err(TryRecvError::Empty) => {
                    if i == 0 {
                        return Err(Error::new(EAGAIN));
                    } else {
                        break;
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    break;
                }
            }
        }
        Ok(i)
    }

    fn write(
        &mut self,
        fd: usize,
        buf: &[u8],
        _offset: u64,
        _fcntl_flags: u32,
        _ctx: &CallerCtx
    ) -> Result<usize> {
        let entry = self
            .handles
            .get_mut(&fd)
            .ok_or(Error::new(EBADFD))?
            .as_brainfuck()?;

        let mut i = 0;
        while i < buf.len() {
            match entry.input_channel.send(buf[i]) {
                Ok(_) => {
                    i += 1;
                }
                Err(_) => {
                    break;
                }
            }
        }
        Ok(i) 
    }

    fn on_close(&mut self, id: usize) {
        let _ = self.handles.remove(&id);
    }
}
