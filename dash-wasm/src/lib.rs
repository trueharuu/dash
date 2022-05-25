use dash::compiler::StaticImportKind;
use dash::vm::frame::Frame;
use dash::vm::params::VmParams;
use dash::vm::Vm;
use dash::EvalError;
use dash_core as dash;

use dash::compiler::decompiler;
use dash::compiler::FunctionCompiler;
use dash::optimizer;
use dash::parser::parser::Parser;
use dash::vm::local::LocalScope;
use dash::vm::value::ops::abstractions::conversions::ValueConversion;
use dash::vm::value::Value;
use std::fmt::Write;
use wasm_bindgen::prelude::*;

use crate::externalvm::OptLevel;

mod externalfunction;
mod externalvm;
mod jsvalue;
mod util;

#[wasm_bindgen]
pub enum Emit {
    Bytecode,
    JavaScript,
}

#[wasm_bindgen]
pub fn eval(s: &str, opt: OptLevel, _context: Option<js_sys::Object>) -> Result<String, JsValue> {
    fn import_callback(_: &mut Vm, _: StaticImportKind, path: &str) -> Result<Value, Value> {
        Ok(Value::String(format!("Hello from module {path}").into()))
    }

    fn random_callback(_: &mut Vm) -> Result<f64, Value> {
        Ok(js_sys::Math::random())
    }

    let params = VmParams::new()
        .set_static_import_callback(import_callback)
        .set_math_random_callback(random_callback);

    let mut vm = Vm::new(params);

    let result = match vm.eval(s, opt.into()) {
        Ok(value) => {
            let mut scope = LocalScope::new(&mut vm);
            let inspect = compile_inspect(&mut scope);

            let value = inspect
                .apply(&mut scope, Value::undefined(), vec![value])
                .map(|x| match x {
                    Value::String(s) => String::from(s.as_ref()),
                    _ => unreachable!(),
                });

            match value {
                Ok(value) => value,
                Err(e) => fmt_value(e, &mut scope),
            }
        }
        Err(EvalError::VmError(val)) => fmt_value(val, &mut vm),
        Err(e) => e.to_string().into(),
    };

    Ok(result)
}

fn fmt_value(value: Value, vm: &mut Vm) -> String {
    let mut scope = LocalScope::new(vm);
    value
        .to_string(&mut scope)
        .map(|s| ToString::to_string(&s))
        .unwrap_or_else(|_| "<exception>".into())
}

#[wasm_bindgen]
pub fn decompile(s: &str, o: OptLevel, em: Emit) -> String {
    let parser = Parser::from_str(s).unwrap();
    let mut ast = parser.parse_all().unwrap();
    optimizer::optimize_ast(&mut ast, o.into());

    match em {
        Emit::Bytecode => {
            let cmp = FunctionCompiler::new().compile_ast(ast).unwrap();
            decompiler::decompile(cmp).unwrap_or_else(|e| match e {
                decompiler::DecompileError::AbruptEof => String::from("Error: Abrupt end of file"),
                decompiler::DecompileError::UnknownInstruction(u) => {
                    format!("Error: Unknown or unimplemented instruction 0x{:x}", u)
                }
            })
        }
        Emit::JavaScript => {
            let mut output = String::new();
            for node in ast {
                let _ = write!(output, "{node}; ");
            }
            output
        }
    }
}

fn compile_inspect(vm: &mut Vm) -> Value {
    let source = include_str!("../../dash-rt/js/inspect.js");
    let ast = Parser::from_str(source).unwrap().parse_all().unwrap();
    let re = FunctionCompiler::new().compile_ast(ast).unwrap();

    let f = Frame::from_compile_result(re);
    vm.execute_module(f).unwrap().default.unwrap()
}
