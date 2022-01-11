use std::{
    fs::File,
    io::Read
};
use oh::prelude::*;

fn main() {
    let mut args = std::env::args();
    args.next().unwrap();
    let path = args.next().unwrap();


    let program_input = Value::List(args.map(|arg| Value::from_string(arg)).collect());

    let mut runtime = Runtime::new(program_input, OutputHandler::default());

    runtime.init_default_functions();

    let mut code = String::new();
    File::open(&path).unwrap().read_to_string(&mut code).unwrap();
    let program = Program::parse(&code);

    runtime.run(&program);
}
