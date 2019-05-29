extern crate syscall; //add "redox_syscall": "*" to your cargo dependencies

use syscall::scheme::SchemeMut;
use syscall::error::*;

use std::rc::{Rc, Weak};
use std::thread::JoinHandle;
use std::sync::mpsc::TryRecvError;
use std::cell::RefCell;
use std::sync::{RwLock, Arc, Mutex};
use std::collections::btree_map::BTreeMap;
use core::sync::atomic::{AtomicUsize, Ordering};
use std::io::Read;

mod examples;

type CellSize = isize;

struct MemoryCell {
    left: Weak<RefCell<MemoryCell>>,
    right: Weak<RefCell<MemoryCell>>,

    value: CellSize,
}

impl MemoryCell {
    fn new() -> MemoryCell {
        MemoryCell { left: Weak::new(), right: Weak::new(), value: 0 }
    }
}


struct BrainfuckProgramm {
    code: Vec<char>,
    program_pointer: usize,
    memory_cells: Vec<Rc<RefCell<MemoryCell>>>,
    memory_pointer: Rc<RefCell<MemoryCell>>,
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

fn run_program(program_code: &str) -> Result<()> {
    let scheme = Arc::new(Mutex::new(BrainfuckScheme::new()));
    let fd = scheme.lock().unwrap().open(program_code.as_bytes(), 0, 0, 0)?;

    let mut buf = [0u8; 1024];

    let scheme_copy = scheme.clone();

    std::thread::spawn(move || {
        let mut buf = [0u8; 16];
        loop {
            if let Ok(amount) = std::io::stdin().read(&mut buf) {
                let mut tmp = &buf[0..amount];
                while tmp.len() > 0 {
                    if let Ok(length) = scheme_copy.lock().unwrap().write(fd, &tmp){
                        tmp = &tmp[length..]
                    }else {
                        //broken channel is not recoverable
                        return ();
                    }
                }
            }
        }
    });

    while let Ok(count) = scheme.lock().unwrap().read(fd, &mut buf) {
        for i in 0..count {
            print!("{}", buf[i] as char)
        }
    }

    scheme.lock().unwrap().close(fd)?;

    Ok(())
}

#[test]
#[ignore]
fn run_cat() -> Result<()> {
    run_program(examples::CAT)
}

#[test]
fn run_calc_cell_size() -> Result<()> {
    run_program(examples::CELL_SIZE)
}

#[test]
fn run_hello_world() -> Result<()> {
    run_program(examples::HELLO_WORLD)
}

impl BrainfuckScheme {
    fn new() -> BrainfuckScheme {
        BrainfuckScheme { next_id: AtomicUsize::new(0), handles: RwLock::new(BTreeMap::new()) }
    }
}


fn run_brain_fuck(mut program_state: BrainfuckProgramm, input: std::sync::mpsc::Receiver<u8>, output: std::sync::mpsc::Sender<u8>) {
    while program_state.program_pointer < program_state.code.len() {
        match program_state.code[program_state.program_pointer] {
            '+' => {
                let v = program_state.memory_pointer.borrow().value;
                program_state.memory_pointer.borrow_mut().value = CellSize::wrapping_add(v, 1)
            }
            '-' => {
                let v = program_state.memory_pointer.borrow().value;
                program_state.memory_pointer.borrow_mut().value = CellSize::wrapping_sub(v, 1);
            }
            '.' => {
                if let Err(_) = output.send(program_state.memory_pointer.borrow().value as u8){
                    //ignore error
                }
            }
            ',' => {
                program_state.memory_pointer.borrow_mut().value = input.recv().unwrap() as CellSize
            }
            '>' => {
                let next_cell = program_state.memory_pointer.borrow().right.upgrade();
                if let Some(right) = next_cell {
                    program_state.memory_pointer = right;
                } else {
                    //create new cell
                    let mem = Rc::new(RefCell::new(MemoryCell::new()));
                    program_state.memory_cells.push(mem.clone());
                    //link neighbours
                    mem.borrow_mut().left = Rc::downgrade(&program_state.memory_pointer);
                    program_state.memory_pointer.borrow_mut().right = Rc::downgrade(&mem);
                    //set pointer
                    program_state.memory_pointer = mem;
                }
            }
            '<' => {
                let next_cell = program_state.memory_pointer.borrow().left.upgrade();
                if let Some(left) = next_cell {
                    program_state.memory_pointer = left;
                } else {
                    //create new cell
                    let mem = Rc::new(RefCell::new(MemoryCell::new()));
                    program_state.memory_cells.push(mem.clone());
                    //link neighbours
                    mem.borrow_mut().right = Rc::downgrade(&program_state.memory_pointer);
                    program_state.memory_pointer.borrow_mut().left = Rc::downgrade(&mem);
                    //set pointer
                    program_state.memory_pointer = mem;
                }
            }
            '[' => {
                if program_state.memory_pointer.borrow().value == 0 {
                    program_state.program_pointer += 1;
                    let mut indent = 0i128;
                    while indent > 0 || (program_state.code[program_state.program_pointer] != ']' && indent == 0) {
                        match program_state.code[program_state.program_pointer] {
                            '[' => indent += 1,
                            ']' => indent -= 1,
                            _ => {}
                        }
                        program_state.program_pointer += 1;
                    }
                    if indent < 0 {
                        //TODO error
                    }
                }
            }
            ']' => {
                if program_state.memory_pointer.borrow().value != 0 {
                    program_state.program_pointer -= 1;
                    let mut indent = 0i128;
                    while indent > 0 || (program_state.code[program_state.program_pointer] != '[' && indent == 0) {
                        match program_state.code[program_state.program_pointer] {
                            '[' => indent -= 1,
                            ']' => indent += 1,
                            _ => {}
                        }
                        program_state.program_pointer -= 1;
                    }
                    if indent < 0 {
                        //TODO error
                    }
                }
            }
            _ => {}
        }
        program_state.program_pointer += 1;
    }
}


impl BrainfuckProgramm {
    fn new(programm: Vec<char>) -> BrainfuckProgramm {
        let mem = Rc::new(RefCell::new(MemoryCell::new()));

        BrainfuckProgramm {
            code: programm,
            program_pointer: 0,
            memory_cells: vec![mem.clone()],
            memory_pointer: mem,
        }
    }
}

impl SchemeMut for BrainfuckScheme {
    fn open(&mut self, path: &[u8], _flags: usize, _uid: u32, _gid: u32) -> Result<usize> {
        let program_code: Vec<char> = String::from_utf8_lossy(path).chars().collect();

        let (input_sender, input_receiver) = std::sync::mpsc::channel();
        let (output_sender, output_receiver) = std::sync::mpsc::channel();


        let thread = std::thread::spawn(move ||
            run_brain_fuck(BrainfuckProgramm::new(program_code), input_receiver, output_sender)
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
                    return Ok(i);
                } else {
                    return Err(Error::new(syscall::error::EBADFD));
                }
            } else {
                return Err(Error::new(syscall::error::EBADFD));
            }
        } else {
            return Err(Error::new(syscall::error::ENOTRECOVERABLE));
        };
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
                    return Ok(i);
                } else {
                    return Err(Error::new(EBADFD));
                }
            } else {
                return Err(Error::new(EBADFD));
            }
        } else {
            return Err(Error::new(ENOTRECOVERABLE));
        };
    }

    fn close(&mut self, id: usize) -> Result<usize> {
        if let Ok(mut res) = self.handles.write() {
            if let Some(entry) = res.remove(&id) {
                if let Err(_) = entry.thread.join(){
                    Err(Error::new(EBADFD))
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
