extern crate syscall; //add "redox_syscall": "*" to your cargo dependencies

use syscall::scheme::SchemeMut;
use syscall::error::{Error, Result, ENOENT, EBADF, EINVAL};

use std::cmp::min;
use std::rc::{Rc, Weak};
use std::thread::{Thread, JoinHandle};
use std::collections::vec_deque::VecDeque;
use std::sync::mpsc::Sender;
use std::cell::{RefCell, RefMut};

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

struct BrainfuckScheme {
    input_channel: Option<std::sync::mpsc::Sender<char>>,
    output_channel: Option<std::sync::mpsc::Receiver<char>>,
    thread: Option<JoinHandle<()>>,
}

#[test]
fn run_calc_cell_size(){
    let mut scheme = BrainfuckScheme::new();
    scheme.open("
Calculate the value 256 and test if it's zero
If the interpreter errors on overflow this is where it'll happen
++++++++[>++++++++<-]>[<++++>-]
+<[>-<
    Not zero so multiply by 256 again to get 65536
    [>++++<-]>[<++++++++>-]<[>++++++++<-]
    +>[>
        # Print '32'
        ++++++++++[>+++++<-]>+.-.[-]<
    <[-]<->] <[>>
        # Print '16'
        +++++++[>+++++++<-]>.+++++.[-]<
<<-]] >[>
    # Print '8'
    ++++++++[>+++++++<-]>.[-]<
<-]<
# Print  bit cells\n
+++++++++++[>+++>+++++++++>+++++++++>+<<<<-]>-.>-.+++++++.+++++++++++.<.
>>.++.+++++++..<-.>>-
Clean up used cells.
[[-]<]
    ".as_bytes(), 0, 0, 0);

    while let Ok(val) = scheme.output_channel.as_mut().unwrap().recv() {
        print!("{}", val);
    }
}

#[test]
fn run_hello_world() {
    let mut scheme = BrainfuckScheme::new();
    scheme.open("
    ++++++++++
 [
  >+++++++>++++++++++>+++>+<<<<-
 ]                       Schleife zur Vorbereitung der Textausgabe
 >++.                    Ausgabe von 'H'
 >+.                     Ausgabe von 'e'
 +++++++.                'l'
 .                       'l'
 +++.                    'o'
 >++.                    Leerzeichen
 <<+++++++++++++++.      'W'
 >.                      'o'
 +++.                    'r'
 ------.                 'l'
 --------.               'd'
 >+.                     '!'
 >.                      Zeilenvorschub
 +++.                    Wagenrücklauf
    ".as_bytes(), 0, 0, 0);

    while let Ok(val) = scheme.output_channel.as_mut().unwrap().recv() {
        print!("{}", val);
    }
}

impl BrainfuckScheme {
    fn new() -> BrainfuckScheme {
        BrainfuckScheme {
            input_channel: None,
            output_channel: None,
            thread: None,
        }
    }

    fn start(&mut self, program: Vec<char>) -> () {
        let (input_sender, input_receiver) = std::sync::mpsc::channel();
        let (output_sender, output_receiver) = std::sync::mpsc::channel();

        self.input_channel = Some(input_sender);
        self.output_channel = Some(output_receiver);


        self.thread = Some(std::thread::spawn(move||
            run_brain_fuck(BrainfuckProgramm::new(program), input_receiver, output_sender)
        ));
    }
}

fn run_brain_fuck(mut program_state: BrainfuckProgramm, input: std::sync::mpsc::Receiver<char>, output: std::sync::mpsc::Sender<char>) {
    while program_state.programmPointer >= 0 && program_state.programmPointer < program_state.code.len() {
        match program_state.code[program_state.programmPointer] {
            '+' => {
                let v = program_state.memoryPointer.borrow().value;
                program_state.memoryPointer.borrow_mut().value = CellSize::wrapping_add(v,1)
            }
            '-' => {
                let v = program_state.memoryPointer.borrow().value;
                program_state.memoryPointer.borrow_mut().value = CellSize::wrapping_sub(v,1);
            }
            '.' => {
                output.send(program_state.memoryPointer.borrow().value as u8 as char);
            }
            ',' => {
                program_state.memoryPointer.borrow_mut().value = input.recv().unwrap() as CellSize
            }
            '>' => {
                let next_cell = program_state.memoryPointer.borrow().right.upgrade();
                if let Some(right) = next_cell {
                    program_state.memoryPointer = right;
                } else {
                    //create new cell
                    let mem = Rc::new(RefCell::new(MemoryCell::new()));
                    program_state.memoryCells.push(mem.clone());
                    //link neighbours
                    mem.borrow_mut().left = Rc::downgrade(&program_state.memoryPointer);
                    program_state.memoryPointer.borrow_mut().right = Rc::downgrade(&mem);
                    //set pointer
                    program_state.memoryPointer = mem;
                }
            }
            '<' => {
                let next_cell = program_state.memoryPointer.borrow().left.upgrade();
                if let Some(left) = next_cell {
                    program_state.memoryPointer = left;
                } else {
                    //create new cell
                    let mem = Rc::new(RefCell::new(MemoryCell::new()));
                    program_state.memoryCells.push(mem.clone());
                    //link neighbours
                    mem.borrow_mut().right = Rc::downgrade(&program_state.memoryPointer);
                    program_state.memoryPointer.borrow_mut().left = Rc::downgrade(&mem);
                    //set pointer
                    program_state.memoryPointer = mem;
                }
            }
            '[' => {
                if program_state.memoryPointer.borrow().value == 0 {
                    program_state.programmPointer += 1;
                    let mut indent = 0i128;
                    while indent > 0 || (program_state.code[program_state.programmPointer] != ']' && indent == 0){
                        match program_state.code[program_state.programmPointer] {
                            '[' => indent += 1,
                            ']' => indent -= 1,
                            _ => {}
                        }
                        program_state.programmPointer += 1;
                    }
                    if indent < 0 {
                        //TODO error
                    }
                }
            }
            ']' => {
                if program_state.memoryPointer.borrow().value != 0 {
                    program_state.programmPointer -= 1;
                    let mut indent = 0i128;
                    while indent > 0 || (program_state.code[program_state.programmPointer] != '[' && indent == 0) {
                        match program_state.code[program_state.programmPointer] {
                            '[' => indent -= 1,
                            ']' => indent += 1,
                            _ => {}
                        }
                        program_state.programmPointer -= 1;
                    }
                    if indent < 0 {
                        //TODO error
                    }
                }
            }
            _ => {}
        }
        program_state.programmPointer += 1;
    }
}

struct BrainfuckProgramm {
    code: Vec<char>,
    programmPointer: usize,
    memoryCells: Vec<Rc<RefCell<MemoryCell>>>,
    memoryPointer: Rc<RefCell<MemoryCell>>,
}


impl BrainfuckProgramm {
    fn new(programm:Vec<char>) -> BrainfuckProgramm {
        let mem = Rc::new(RefCell::new(MemoryCell::new()));

        BrainfuckProgramm {
            code: programm,
            programmPointer: 0,
            memoryCells: vec![mem.clone()],
            memoryPointer: mem,
        }
    }
}

impl SchemeMut for BrainfuckScheme {
    fn open(&mut self, path: &[u8], _flags: usize, _uid: u32, _gid: u32) -> Result<usize> {
        let program_code:Vec<char> = String::from_utf8_lossy(path).chars().collect();
        let result = program_code.len();
        self.start(program_code);
        Ok(result)
    }
}