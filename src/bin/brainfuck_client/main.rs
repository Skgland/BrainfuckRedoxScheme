use std::fs::File;
use std::process::ExitCode;

fn main() -> ExitCode {

    let Some(program) = std::env::args().nth(1) else {
        eprintln!("First argument must be a brainfuck program!");
        return ExitCode::FAILURE;
    };

    let mut brainfuck_file_input = File::open(format!("/scheme/brainfuck/{program}"))
        .expect("Failed to open vec file");

    let mut brainfuck_file_output = brainfuck_file_input.try_clone().unwrap();

    std::thread::scope(|env| {
        env.spawn(|| {
            let mut stdout = std::io::stdout().lock();
            if let Err(err) = std::io::copy(&mut brainfuck_file_output, &mut stdout) {
                eprintln!("Failed to pipe output: {err}")
            };
        });
        
        let mut stdin = std::io::stdin().lock();
        if let Err(err) = std::io::copy(&mut stdin,  &mut brainfuck_file_input) {
                eprintln!("Failed to pipe input: {err}")
        };
    });

    ExitCode::SUCCESS
}