use std::{
    collections::{BTreeMap, HashSet},
    fmt::{Debug, Display, Formatter},
};
use std::str::FromStr;
use enum_iterator::IntoEnumIterator;
use rand::prelude::*;
use itertools::Itertools;

pub mod prelude {
    pub use crate::{Program, Runtime, Value, OutputHandler};
}

fn input(runtime: &mut Runtime, variables: &mut Variables, line: &Line) {
    variables.set_val(&line.left[0], runtime.input.clone());
}

fn output(runtime: &mut Runtime, variables: &mut Variables, line: &Line) {
    runtime.fresh_output = true;
    (runtime.output_handler.new_output)(format!("{}", line.right[0].value(&variables)));
}

fn get_2_nums(variables: &mut Variables, line: &Line) -> (i64, i64) {
    if let (Value::Number(i0), Value::Number(i1)) = (line.right[0].value(&variables), line.right[1].value(&variables)) {
        (*i0,*i1)
    } else {
        unreachable!("These weren't two numbers!")
    }
}

fn add(_: &mut Runtime, variables: &mut Variables, line: &Line) {
    let (a, b) = get_2_nums(variables, line);
    variables.set_val(&line.left[0],Value::Number(a+b));
}

fn sub(_: &mut Runtime, variables: &mut Variables, line: &Line) {
    let (a, b) = get_2_nums(variables, line);
    variables.set_val(&line.left[0],Value::Number(a-b));
}

fn mul(_: &mut Runtime, variables: &mut Variables, line: &Line) {
    let (a, b) = get_2_nums(variables, line);
    variables.set_val(&line.left[0],Value::Number(a*b));
}

fn div(runtime: &mut Runtime, variables: &mut Variables, line: &Line) {
    let (a, b) = get_2_nums(variables, line);
    if let Some(result) = a.checked_div(b) {
        variables.set_val(&line.left[0],Value::Number(result)); //TODO: checked div -> ok_state = false
    } else {
        runtime.ok_state = false;
    }
}

fn assign(_: &mut Runtime, variables: &mut Variables, line: &Line) {
    variables.set_val(&line.left[0],line.right[0].value(variables).clone());
}

fn num_to_str(_: &mut Runtime, variables: &mut Variables, line: &Line) {
    let num = *if let Value::Number(index) = line.right[0].value(variables) {index} else {unreachable!()};
    variables.set_val(&line.left[0],Value::from_string(num.to_string()));
}

fn str_to_num(runtime: &mut Runtime, variables: &mut Variables, line: &Line) {
    if let Some(string) = line.right[0].value(variables).to_string(){
        if let Ok(num) = i64::from_str(&string) {
            variables.set_val(&line.left[0], Value::Number(num));
            return;
        }
    }
    runtime.ok_state = false;
}

fn jump(runtime: &mut Runtime, variables: &mut Variables, line: &Line) {
    if let Value::Number(target) = line.right[0].value(variables) {
        let line_nr = *target - 1;
        if line_nr > 0 && line_nr <= runtime.program_length as i64 {
            runtime.next_line = line_nr as usize;
        } else {
            runtime.ok_state = false;
        }
    } else {
        unreachable!();
    }
}

fn get(runtime: &mut Runtime, variables: &mut Variables, line: &Line) {
    let list = if let Value::List(list) = line.right[0].value(variables) {list} else {unreachable!()};
    let index = if let Value::Number(index) = line.right[1].value(variables) {index} else {unreachable!()};
    if *index >= 0 {
        let index = *index as usize;
        if let Some(element) = list.get(index) {
            if element.ty() == *variables.types.get(&line.left[0]).unwrap() {
                let item = element.clone();
                variables.set_val(&line.left[0], item);
                return;
            }
        }
    }
    runtime.ok_state = false;
}

fn put(runtime: &mut Runtime, variables: &mut Variables, line: &Line) {
    let list = if let Value::List(list) = line.right[0].value(variables) {list} else {unreachable!()};
    let index = if let Value::Number(index) = line.right[1].value(variables) {index} else {unreachable!()};
    let element = line.right[2].value(variables);
    if *index >= 0 {
        let index = *index as usize;
        if index < list.len() {
            let mut list = list.clone();
            list[index] = element.clone();
            variables.set_val(&line.left[0], Value::List(list));
            return;
        }
    }
    runtime.ok_state = false;
}

fn push(_: &mut Runtime, variables: &mut Variables, line: &Line) {
    let element = line.right[0].value(variables);
    let list = if let Value::List(list) = line.right[1].value(variables) {list} else {unreachable!()};

    let mut list = list.clone();
    list.push(element.clone());
    variables.set_val(&line.left[0], Value::List(list));
}

fn pop(runtime: &mut Runtime, variables: &mut Variables, line: &Line) {
    let list = if let Value::List(list) = line.right[0].value(variables) {list} else {unreachable!()};

    let mut list = list.clone();
    if let Some(element) = list.pop() {
        if element.ty() == *variables.types.get(&line.left[0]).unwrap() {
            let element = element.clone();
            variables.set_val(&line.left[0], element);
            variables.set_val(&line.left[1], Value::List(list));
            return;
        }
    }
    runtime.ok_state = false;
}

fn length(_: &mut Runtime, variables: &mut Variables, line: &Line) {
    let list = if let Value::List(list) = line.right[0].value(variables) {list} else {unreachable!()};
    let length = list.len();
    variables.set_val(&line.left[0],Value::Number(length as i64));
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct FunctionType {
    in_ty : Vec<Type>,
    op: Option<Op>,
    out_ty : Vec<Type>,
}

impl FunctionType {
    pub fn new(in_ty: Vec<Type>, op: Option<Op>, out_ty: Vec<Type>) -> Self {
        FunctionType { in_ty, op, out_ty }
    }
}

pub struct Runtime {
    pub functions: BTreeMap<FunctionType, Vec<fn(&mut Runtime, &mut Variables, &Line)>>,
    input: Value,
    past_states: HashSet<(Variables, usize)>,
    ok_state: bool,
    next_line: usize,
    output_handler: OutputHandler,
    fresh_output: bool,
    program_length: usize,
}

pub struct OutputHandler {
    new_output: Box<dyn FnMut(String)>,
    revert_output: Box<dyn FnMut()>,
}

impl OutputHandler {
    pub fn new(new_output: Box<dyn FnMut(String)>, revert_output: Box<dyn FnMut()>) -> Self {
        OutputHandler { new_output, revert_output }
    }
}

impl Default for OutputHandler {
    fn default() -> Self {
        Self {
            new_output: Box::new(|out| println!("{}", out)),
            revert_output: Box::new(|| print!("\x1b[A\x1b[2K")),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Variables {
    values: BTreeMap<VariableName, Value>,
    types: BTreeMap<VariableName, Type>,
}

impl Variables {
    pub fn new() -> Self {
        Self {
            values: BTreeMap::new(),
            types: BTreeMap::new(),
        }
    }

    pub fn set_val(&mut self, name: &VariableName, value: Value) {
        if let Some(var) = self.values.get_mut(name) {
            *var = value;
        } else {
            self.values.insert(name.to_owned(), value);
        }
    }

    pub fn get_val(&self, name: &VariableName) -> &Value {
        self.values.get(name).unwrap()
    }

    pub fn get_mut_val(&mut self, name: &VariableName) -> &mut Value {
        self.values.get_mut(name).unwrap()
    }
}

impl Runtime {
    pub fn new(input: Value, output_handler: OutputHandler) -> Self {
        Runtime { functions: BTreeMap::new(), input, past_states: HashSet::new(), ok_state: true, next_line: 0, output_handler, fresh_output: false, program_length: 0 }
    }

    pub fn init_default_functions(&mut self) {
        self.functions.insert(FunctionType::new(vec![], Some(Op::Equals), vec![Type::List]), vec![input]);
        self.functions.insert(FunctionType::new(vec![Type::Number], Some(Op::Equals), vec![Type::List]), vec![num_to_str]);
        self.functions.insert(FunctionType::new(vec![Type::List], Some(Op::Equals), vec![Type::Number]), vec![str_to_num]);
        self.functions.insert(FunctionType::new(vec![Type::Number, Type::Number], Some(Op::Equals), vec![Type::Number]), vec![add, sub, mul, div]);
        self.functions.insert(FunctionType::new(vec![Type::Number], None, vec![]), vec![jump]);
        self.functions.insert(FunctionType::new(vec![Type::List], None, vec![Type::Number]), vec![length]);
        for ty in Type::into_enum_iter() {
            self.functions.insert(FunctionType::new(vec![ty], Some(Op::Equals), vec![]), vec![output]);
            self.functions.insert(FunctionType::new(vec![ty], Some(Op::Equals), vec![ty]), vec![assign]);
            self.functions.insert(FunctionType::new(vec![ty, Type::List], Some(Op::Equals), vec![Type::List]), vec![push]);
            self.functions.insert(FunctionType::new(vec![Type::List], Some(Op::Equals), vec![ty, Type::List]), vec![pop]);
            self.functions.insert(FunctionType::new(vec![Type::List, Type::Number], Some(Op::Equals), vec![ty]), vec![get]);
            self.functions.insert(FunctionType::new(vec![Type::List, Type::Number, ty], Some(Op::Equals), vec![Type::List]), vec![put]);
        }
    }

    pub fn run(&mut self, program: &Program) {
        self.program_length = program.lines.len();
        let mut variables = Variables::new();
        let oh = VariableName(String::from("oh"));
        variables.types.insert(oh.clone(), Type::List);
        variables.values.insert(oh, Value::Char('🥚'));
        if !self.run_bt(variables, program, 0) {
            (self.output_handler.new_output)(String::from("This program was presented by 𝔘𝔫𝔡𝔢𝔣𝔦𝔫𝔢𝔡𝔅𝔢𝔥𝔞𝔳𝔦𝔬𝔲𝔯™️."));
        } else {
            (self.output_handler.new_output)(String::from("."));
        }
    }

    fn run_bt(&mut self, mut variables: Variables, program: &Program, line_nr: usize) -> bool {
        if line_nr == program.lines.len() {
            return true;
        }

        let mut rng = thread_rng();

        let line = &program.lines[line_nr];
        if !line.left.is_empty() {
            let mut types = Type::into_enum_iter().collect::<Vec<_>>();
            types.shuffle(&mut rng);
            for tys in itertools::repeat_n(types.into_iter(), line.left.len()).multi_cartesian_product() {
                for (i,ty) in tys.into_iter().enumerate() {
                    variables.types.insert(line.left[i].clone(), ty);
                }
                let fun_ty = line.fun_ty(&variables.types);
                if let Some(mut functions) = self.functions.get(&fun_ty).cloned() {
                    functions.shuffle(&mut rng);
                    for fun in functions {
                        self.next_line = line_nr + 1;
                        self.fresh_output = false;
                        fun(self, &mut variables, line);
                        let fresh_output = self.fresh_output;
                        if self.is_state_ok(variables.clone(), line_nr) && self.run_bt(variables.clone(), program, self.next_line) {
                            return true;
                        } else if fresh_output {
                            (self.output_handler.revert_output)();
                        }
                    }
                }
            }
        } else {
            let fun_ty = line.fun_ty(&variables.types);
            if let Some(mut functions) = self.functions.get(&fun_ty).cloned() {
                functions.shuffle(&mut rng);
                for fun in functions {
                    self.next_line = line_nr + 1;
                    self.fresh_output = false;
                    fun(self, &mut variables, line);
                    let fresh_output = self.fresh_output;
                    if self.is_state_ok(variables.clone(), line_nr) && self.run_bt(variables.clone(), program, self.next_line) {
                        return true;
                    } else if fresh_output {
                        (self.output_handler.revert_output)();
                    }
                }
            }
        }

        return false;
    }

    fn is_state_ok(&mut self, variables: Variables, line_nr: usize) -> bool {
        let ok = self.ok_state && self.past_states.insert((variables.clone(), line_nr));
        self.ok_state = true;
        ok
    }
}

#[derive(Debug)]
pub struct Program {
    lines: Vec<Line>
}

impl Program {
    pub fn parse(input: &str) -> Self {
        let lines = input.lines().map(|line| Line::parse(line)).collect();
        Self { lines }
    }
}

#[derive(Debug)]
pub struct Line {
    left: Vec<VariableName>,
    op: Option<Op>,
    right: Vec<Item>,
}

impl Line {
    pub fn parse(input: &str) -> Line {
        if let Some((left, right)) = input.split_once('=') {
            let op = Some(Op::Equals);
            let left = left.split(' ').filter(|s| !s.is_empty()).map(|s| VariableName(s.to_owned())).collect();
            let right = right.split(' ').filter(|s| !s.is_empty()).map(Item::parse).collect();
            Line {
                left,
                op,
                right,
            }
        } else {
            let right = input.split(' ').map(Item::parse).collect();
            Line {
                left: Vec::new(),
                op: None,
                right,
            }
        }
    }

    pub fn fun_ty(&self, variable_types: &BTreeMap<VariableName, Type>) -> FunctionType {
        let out_ty = self.left.iter().map(|name| *variable_types.get(name).unwrap()).collect();
        let in_ty = self.right.iter().map(|item|
            match item {
                Item::Variable(var) =>
                    *variable_types.get(var).unwrap(),
                Item::Literal(val) => val.ty(),
            }).collect();
        FunctionType { in_ty, op: self.op, out_ty }
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Op {
    Equals
}

#[derive(Debug)]
pub enum Item {
    Variable(VariableName),
    Literal(Value),
}

impl Item {
    pub fn parse(input: &str) -> Item {
        let mut chars = input.chars();
        let first =  chars.next().unwrap();

        if first.is_alphabetic() {
            Item::Variable(VariableName(input.to_owned()))
        } else if first.is_digit(10) {
            Item::Literal(Value::Number(input.parse().unwrap()))
        } else if first == '"' {
            Item::Literal(Value::List(chars.take_while(|&c| c != '"').map(Value::Char).collect()))
        } else if first == '\'' {
            Item::Literal(Value::Char(chars.next().unwrap()))
        } else {
            panic!("Bad input: '{}'", first)
        }
    }

    pub fn value<'a>(&'a self, variables: &'a Variables) -> &'a Value {
        match self {
            Item::Variable(var) => {variables.get_val(var)}
            Item::Literal(lit) => {lit}
        }
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct VariableName(String);

#[derive(Debug, Clone, Copy, IntoEnumIterator, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Type {
    Number,
    Char,
    List,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Value {
    Number(i64),
    Char(char),
    List(Vec<Value>),
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(i) => {Display::fmt(i, f)}
            Value::Char(c) => {Display::fmt(c, f)}
            Value::List(l) => {
                let mut s = String::new();
                for c in l.iter() {
                    if let Value::Char(c) = c {
                        s.push(*c);
                    } else {
                        return write!(f, "[{}]", comma_seperated(l))
                    }
                }
                Display::fmt(&s, f)
            }
        }
    }
}

impl Value {
    pub fn ty(&self) -> Type {
        match self {
            Value::Number(_) => Type::Number,
            Value::Char(_) => Type::Char,
            Value::List(_) => Type::List,
        }
    }

    pub fn from_string(s: String) -> Value {
        Value::List(s.chars().map(|c| Value::Char(c)).collect())
    }

    pub fn to_string(&self) -> Option<String> {
        let vec = if let Value::List(vec) = self {vec} else {unreachable!()};
        let mut s = String::new();
        for c in vec {
            if let Value::Char(c) = c {
                s.push(*c);
            } else {
                return None;
            }
        }
        Some(s)
    }
}

fn comma_seperated<T: ToString>(vec: &Vec<T>) -> impl Display {
    let mut s = vec.iter().map(|it| {let mut s = it.to_string(); s.push_str(", "); s}).collect::<String>();
    s.truncate(s.len() - 2);
    s
}
