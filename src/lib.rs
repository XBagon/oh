use wasm_bindgen::prelude::*;
use oh::prelude::*;

#[wasm_bindgen]
pub fn interpret(input: String, args: js_sys::Array) {
    let args = args.to_vec().into_iter().map(|val| val.as_string().unwrap());
    let program_input = Value::List(args.map(|arg| Value::List(arg.chars().map(|c| Value::Char(c)).collect())).collect());

    let output_handler = OutputHandler::new(Box::new(|out| new_output(out.clone())), Box::new(|| revert_output()));
    let mut runtime = Runtime::new(program_input, output_handler);

    runtime.init_default_functions();

    let program = Program::parse(&input);

    clear_output();

    runtime.run(&program);
}

#[wasm_bindgen(module = "/output_handler.js")]
extern "C" {
    fn new_output(output: String);
    fn revert_output();
    fn clear_output();
}