use std::{fmt::Write, io::Read as _, sync::{Arc, Mutex}};

use brainfuck_redox_scheme::brainfuck::{BrainfuckInterpreter, CellSize};
use syscall::error::Result;
use redox_scheme::{CallerCtx, Id, scheme::SchemeSync};

use crate::{BrainfuckScheme};


#[derive(PartialEq, Eq)]
struct DisplacAscii<'a> {
    data: &'a [u8]
}

impl std::fmt::Debug for DisplacAscii<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for &b in self.data {
            f.write_char(b.into())?
        }
        Ok(())
    }
}

fn run_program(program_code: &str, expected_output: &[u8]) -> Result<()> {
    let scheme = BrainfuckScheme::new();
    let root = scheme.scheme_root().unwrap();
    let caller_ctx = CallerCtx{
        pid: 0,
        uid: 0,
        gid: 0,
        id: Id(0),
    };

    let fd = scheme.openat(root, program_code, 0, 0, &caller_ctx)?;

    let mut buf = [0u8; 1024];

    let scheme_copy = scheme.clone();

    std::thread::spawn(move || {
        let mut buf = [0u8; 16];
        loop {
            if let Ok(amount) = std::io::stdin().read(&mut buf) {
                let mut tmp = &buf[0..amount];
                while !tmp.is_empty() {
                    if let Ok(length) = scheme_copy.lock().unwrap().write(fd, tmp){
                        tmp = &tmp[length..]
                    }else {
                        //broken channel is not recoverable
                        return;
                    }
                }
            }
        }
    });

    let mut output = Vec::new();

    while let Ok(count) = scheme.lock().unwrap().read(fd, &mut buf) {
        output.extend_from_slice(&buf[..count]);
    }

    scheme.lock().unwrap().on_close(fd);

    assert_eq!(DisplacAscii{data: output.as_slice()}, DisplacAscii{data: expected_output});

    Ok(())
}

#[test]
fn run_cat() {
    let (input_sender, input_receiver) = std::sync::mpsc::channel();
    let (output_sender, output_receiver) = std::sync::mpsc::channel();

    let join_handle = std::thread::spawn(|| {
        BrainfuckInterpreter::new(brainfuck_redox_scheme::examples::CAT.chars().collect()).run_brain_fuck(input_receiver, output_sender);
    });

    let input = "Abracedabra!";

    for &b in input.as_bytes() {
        input_sender.send(b).unwrap();
    }

    drop(input_sender);

    join_handle.join().unwrap();

    let output: Vec<_> = output_receiver.into_iter().collect();

    assert_eq!(DisplacAscii{data: output.as_slice()}, DisplacAscii{data: input.as_bytes()});
}

#[test]
fn run_calc_cell_size() -> Result<()> {
    // test program only tests for 8/16/32 bits
    let expected = format!("{} bit cells\n", (std::mem::size_of::<CellSize>() * 8).min(32));

    run_program(brainfuck_redox_scheme::examples::CELL_SIZE,expected.as_bytes())
}

#[test]
fn run_hello_world() -> Result<()> {
    run_program(brainfuck_redox_scheme::examples::HELLO_WORLD, b"Hello World!\n\r") // why LFCR instead of CRLF ?
}