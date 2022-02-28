use std::{any::Any, cell::RefCell, collections::HashMap, fmt::Debug};

use crate::{gc::trace::Trace, vm::Vm};

use super::Value;

// only here for the time being, will be removed later
fn __assert_trait_object_safety(_: Box<dyn Object>) {}

pub trait Object: Debug + Trace {
    fn get_property(&self, vm: &mut Vm, key: &str) -> Result<Value, Value>;
    fn set_property(&self, vm: &mut Vm, key: &str, value: Value) -> Result<Value, Value>;
    fn apply(&self, vm: &mut Vm, this: Value, args: Vec<Value>) -> Result<Value, Value>;
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug)]
pub struct AnonymousObject {
    values: RefCell<HashMap<String, Value>>,
}

impl AnonymousObject {
    pub fn new() -> Self {
        Self {
            values: RefCell::new(HashMap::new()),
        }
    }
}

unsafe impl Trace for AnonymousObject {
    fn trace(&self) {
        let values = self.values.borrow();
        for value in values.values() {
            value.trace();
        }
    }
}

impl Object for AnonymousObject {
    fn get_property(&self, vm: &mut Vm, key: &str) -> Result<Value, Value> {
        let map = self.values.borrow();
        map.get(key).cloned().ok_or(Value::Undefined)
    }

    fn set_property(&self, vm: &mut Vm, key: &str, value: Value) -> Result<Value, Value> {
        let mut map = self.values.borrow_mut();
        map.insert(key.into(), value);
        Ok(Value::Undefined)
    }

    fn apply(&self, vm: &mut Vm, this: Value, args: Vec<Value>) -> Result<Value, Value> {
        Ok(Value::Undefined)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
