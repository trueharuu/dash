use std::collections::HashMap;
use std::rc::Rc;
use std::rc::Weak;

use dash_middle::compiler::constant::Constant;
use dash_middle::compiler::constant::Function;
use dash_middle::compiler::instruction as inst;
use indexmap::IndexMap;

#[derive(Debug)]
pub struct Trace {
    /// The "parent" trace
    ///
    /// This is `Some` if this trace records a side exit and will contain a
    /// strong reference to the predecessor trace
    pub(crate) parent: Option<Weak<Trace>>,
    /// The "successor" traces
    pub(crate) successors: HashMap<usize, Rc<Trace>>,
    pub(crate) origin: *const Function,
    pub(crate) start: usize,
    pub(crate) end: usize,
    /// A map that maps instruction pointer of conditional jumps to whether that jump was taken
    pub(crate) conditional_jumps: HashMap<usize, bool>,
}

impl Trace {
    pub fn new(origin: *const Function, start: usize, end: usize, parent: Option<Weak<Trace>>) -> Self {
        Self {
            parent,
            successors: HashMap::new(),
            origin,
            start,
            end,
            conditional_jumps: HashMap::new(),
        }
    }

    pub fn did_take_jump_at(&self, ip: usize) -> bool {
        self.conditional_jumps[&ip]
    }

    pub fn record_conditional_jump_at(&mut self, ip: usize, taken: bool) {
        self.conditional_jumps.insert(ip, taken);
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn origin(&self) -> *const Function {
        self.origin
    }

    pub fn parent(&self) -> Option<&Weak<Trace>> {
        self.parent.as_ref()
    }
}
