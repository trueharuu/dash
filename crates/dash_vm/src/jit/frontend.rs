use std::collections::HashMap;
use std::rc::Rc;

use dash_llvm_jit_backend::backend::CompiledFunction;
use dash_llvm_jit_backend::backend::JitFunction;
use dash_llvm_jit_backend::error::Error;
use dash_llvm_jit_backend::init;
use dash_llvm_jit_backend::passes::infer::infer_types_and_labels;
use dash_llvm_jit_backend::Backend;
use dash_middle::compiler::constant::Function;

use crate::Vm;

use super::query::QueryProvider;
pub use dash_llvm_jit_backend::Trace;

pub struct Frontend {
    /// If we are currently recording a trace for a loop iteration,
    /// this will contain metadata such as the pc of the loop header and its end
    trace: Option<Trace>,
    /// The JIT backend
    backend: Backend,
    cache: HashMap<(*const (), usize), CompiledFunction>,
}

impl Frontend {
    pub fn new() -> Self {
        init();

        Self {
            trace: None,
            backend: Backend::new(),
            cache: HashMap::new(),
        }
    }

    pub fn recording_trace(&self) -> Option<&Trace> {
        self.trace.as_ref()
    }

    pub fn recording_trace_mut(&mut self) -> Option<&mut Trace> {
        self.trace.as_mut()
    }

    pub fn take_recording_trace(&mut self) -> Option<Trace> {
        self.trace.take()
    }

    pub fn set_recording_trace(&mut self, trace: Trace) {
        self.trace = Some(trace);
    }

    fn get_cached_function(&self, origin: *const Function, pc: usize) -> Option<&CompiledFunction> {
        self.cache.get(&(origin.cast(), pc))
    }
}

pub fn compile_current_trace(vm: &mut Vm) -> Result<(Rc<Trace>, JitFunction), Error> {
    let frame = vm.frames.last().unwrap();
    let trace = vm.jit.take_recording_trace().unwrap();
    let bytecode = &frame.function.buffer[trace.start()..trace.end()];
    let origin = trace.origin();

    if let Some(cached) = vm.jit.get_cached_function(origin, trace.start()) {
        return Ok((Rc::clone(cached.trace()), cached.compiled()));
    }

    let trace = Rc::new(trace);

    let types = infer_types_and_labels(bytecode, QueryProvider::new(vm, Rc::clone(&trace)))?;

    dbg!(&types.local_tys);
    panic!();

    let fun = vm.jit.backend.compile_trace(
        QueryProvider::new(vm, Rc::clone(&trace)),
        bytecode,
        types,
        Rc::clone(&trace),
    )?;

    let compiled = fun.compiled();

    vm.jit.cache.insert((origin.cast(), trace.start()), fun);

    Ok((trace, compiled))
}
