extern crate syscall; //add "redox_syscall": "*" to your cargo dependencies

use syscall::scheme::SchemeMut;
use syscall::{Packet, error::*};

use std::fs::File;
use std::thread::JoinHandle;
use std::sync::mpsc::TryRecvError;
use std::sync::{RwLock, Mutex};
use std::collections::btree_map::BTreeMap;
use core::sync::atomic::{AtomicUsize, Ordering};
use std::io::{Read, Write};

use brainfuck_redox_scheme::brainfuck::BrainfuckInterpreter;


#[cfg(test)]
mod test;


// based on https://doc.redox-os.org/book/example.html

fn main() {
    let mut scheme = BrainfuckScheme::new();

    let mut handler = File::create(":brainfuck")
        .expect("Failed to create the vec scheme");

    
    let mut packet = Packet::default();

    loop {
        let read_bytes = handler.read(&mut packet).expect("brainfuck: failed to read event from brainfuck scheme handler");

        if read_bytes == 0 {
            break;
        }

        scheme.handle(&mut packet);

        let _ = handler.write(&packet).expect("brainfuck: failed to write response to brainfuck scheme handler");
    }    
}

struct BrainfuckEntry {
    input_channel: Mutex<std::sync::mpsc::Sender<u8>>,
    output_channel: Mutex<std::sync::mpsc::Receiver<u8>>,
    thread: JoinHandle<()>,
}

struct BrainfuckScheme {
    next_id: AtomicUsize,
    handles: RwLock<BTreeMap<usize, BrainfuckEntry>>,
}


impl BrainfuckScheme {
    fn new() -> BrainfuckScheme {
        BrainfuckScheme { next_id: AtomicUsize::new(0), handles: RwLock::new(BTreeMap::new()) }
    }
}


impl SchemeMut for BrainfuckScheme {
    fn open(&mut self, path: &str, _flags: usize, _uid: u32, _gid: u32) -> Result<usize> {
        let program_code: Vec<char> = path.chars().collect();

        let (input_sender, input_receiver) = std::sync::mpsc::channel();
        let (output_sender, output_receiver) = std::sync::mpsc::channel();


        let thread = std::thread::spawn(move || 
            BrainfuckInterpreter::new(program_code).run_brain_fuck(input_receiver, output_sender)
        );

        let entry = BrainfuckEntry { input_channel: Mutex::new(input_sender), output_channel: Mutex::new(output_receiver), thread };
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        if let Ok(mut guard) = self.handles.write() {
            guard.insert(id, entry);
            Ok(id)
        } else {
            //Poisoned Lock is not recoverable
            Err(Error::new(syscall::error::ENOTRECOVERABLE))
        }
    }

    fn read(&mut self, id: usize, buf: &mut [u8]) -> Result<usize> {
        if let Ok(guard) = self.handles.read() {
            if let Some(entry) = guard.get(&id) {
                if let Ok(output_guard) = entry.output_channel.lock() {
                    let mut i = 0;
                    while i < buf.len() {
                        match output_guard.try_recv() {
                            Ok(msg) => {
                                buf[i] = msg;
                                i += 1;
                            }
                            Err(TryRecvError::Empty) => {
                                break;
                            }
                            Err(TryRecvError::Disconnected) => {
                                return if i == 0 {
                                    Err(Error::new(syscall::error::EBADFD))
                                } else {
                                    Ok(i)
                                };
                            }
                        }
                    }
                    Ok(i)
                } else {
                    Err(Error::new(syscall::error::EBADFD))
                }
            } else {
                Err(Error::new(syscall::error::EBADFD))
            }
        } else {
            Err(Error::new(syscall::error::ENOTRECOVERABLE))
        }
    }

    fn write(&mut self, id: usize, buf: &[u8]) -> Result<usize> {
        if let Ok(guard) = self.handles.read() {
            if let Some(entry) = guard.get(&id) {
                if let Ok(output_guard) = entry.input_channel.lock() {
                    let mut i = 0;
                    while i < buf.len() {
                        match output_guard.send(buf[i]) {
                            Ok(_) => {
                                i += 1;
                            }
                            Err(_) => {
                                return if i > 0 {
                                    Ok(i)
                                } else {
                                    Err(Error::new(syscall::error::EBADFD))
                                };
                            }
                        }
                    }
                    Ok(i)
                } else {
                    Err(Error::new(EBADFD))
                }
            } else {
                Err(Error::new(EBADFD))
            }
        } else {
            Err(Error::new(ENOTRECOVERABLE))
        }
    }

    fn close(&mut self, id: usize) -> Result<usize> {
        if let Ok(mut res) = self.handles.write() {
            if let Some(entry) = res.remove(&id) {
                if entry.thread.join().is_err() {
                    Err(Error::new(EBADF))
                }else{
                    Ok(0)
                }
            } else {
                Err(Error::new(syscall::error::EBADF))
            }
        } else {
            Err(Error::new(syscall::error::ENOTRECOVERABLE))
        }
    }
}
