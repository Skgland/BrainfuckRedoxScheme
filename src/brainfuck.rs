use std::{cell::RefCell, rc::{Rc, Weak}};

pub type CellSize = i32;

struct MemoryCell {
    left: Weak<RefCell<MemoryCell>>,
    right: Weak<RefCell<MemoryCell>>,

    value: CellSize,
}

impl MemoryCell {
    fn new() -> MemoryCell {
        MemoryCell {
            left: Weak::new(),
            right: Weak::new(),
            value: 0,
        }
    }
}

pub struct BrainfuckInterpreter {
    code: Vec<char>,
    program_pointer: usize,
    memory_cells: Vec<Rc<RefCell<MemoryCell>>>,
    memory_pointer: Rc<RefCell<MemoryCell>>,
}

impl BrainfuckInterpreter {
    pub fn new(program: Vec<char>) -> BrainfuckInterpreter {
        let mem = Rc::new(RefCell::new(MemoryCell::new()));

        BrainfuckInterpreter {
            code: program,
            program_pointer: 0,
            memory_cells: vec![mem.clone()],
            memory_pointer: mem,
        }
    }

    pub fn run_brain_fuck(
        mut self,
        input: std::sync::mpsc::Receiver<u8>,
        output: std::sync::mpsc::Sender<u8>,
    ) {
        while self.program_pointer < self.code.len() {
            match self.code[self.program_pointer] {
                '+' => {
                    let v = self.memory_pointer.borrow().value;
                    self.memory_pointer.borrow_mut().value = CellSize::wrapping_add(v, 1)
                }
                '-' => {
                    let v = self.memory_pointer.borrow().value;
                    self.memory_pointer.borrow_mut().value = CellSize::wrapping_sub(v, 1);
                }
                '.' => {
                    if output.send(self.memory_pointer.borrow().value as u8).is_err() {
                        break;
                    }
                }
                ',' => {
                    if let Ok(v) = input.recv() {
                        self.memory_pointer.borrow_mut().value = v as CellSize;
                    } else {
                        break;
                    }
                }
                '>' => {
                    let next_cell = self.memory_pointer.borrow().right.upgrade();
                    if let Some(right) = next_cell {
                        self.memory_pointer = right;
                    } else {
                        //create new cell
                        let mem = Rc::new(RefCell::new(MemoryCell::new()));
                        self.memory_cells.push(mem.clone());
                        //link neighbours
                        mem.borrow_mut().left = Rc::downgrade(&self.memory_pointer);
                        self.memory_pointer.borrow_mut().right = Rc::downgrade(&mem);
                        //set pointer
                        self.memory_pointer = mem;
                    }
                }
                '<' => {
                    let next_cell = self.memory_pointer.borrow().left.upgrade();
                    if let Some(left) = next_cell {
                        self.memory_pointer = left;
                    } else {
                        //create new cell
                        let mem = Rc::new(RefCell::new(MemoryCell::new()));
                        self.memory_cells.push(mem.clone());
                        //link neighbours
                        mem.borrow_mut().right = Rc::downgrade(&self.memory_pointer);
                        self.memory_pointer.borrow_mut().left = Rc::downgrade(&mem);
                        //set pointer
                        self.memory_pointer = mem;
                    }
                }
                '[' => {
                    if self.memory_pointer.borrow().value == 0 {
                        self.program_pointer += 1;
                        let mut indent = 0u64;
                        while indent > 0 || self.code[self.program_pointer] != ']'
                        {
                            match self.code[self.program_pointer] {
                                '[' => indent += 1,
                                ']' => indent -= 1,
                                _ => {}
                            }
                            self.program_pointer += 1;
                        }
                    }
                }
                ']' => {
                    if self.memory_pointer.borrow().value != 0 {
                        self.program_pointer -= 1;
                        let mut indent = 0u64;
                        while indent > 0 || self.code[self.program_pointer] != '['
                        {
                            match self.code[self.program_pointer] {
                                '[' => indent -= 1,
                                ']' => indent += 1,
                                _ => {}
                            }
                            self.program_pointer -= 1;
                        }
                    }
                }
                _ => {}
            }
            self.program_pointer += 1;
        }
    }
}
