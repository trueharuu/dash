use std::{cell::RefCell, rc::Rc};

use crate::vm::value::{function::CallContext, object::Object, Value, ValueKind};

use super::error::{self, MaybeRc};

pub fn array_constructor(_args: CallContext) -> Result<Rc<RefCell<Value>>, Rc<RefCell<Value>>> {
    Ok(Value::new(ValueKind::Undefined).into())
}

pub fn push(value: CallContext) -> Result<Rc<RefCell<Value>>, Rc<RefCell<Value>>> {
    let this_cell = value.receiver.unwrap();

    let mut this = this_cell.borrow_mut();
    let this_arr = match this.as_object_mut() {
        Some(Object::Array(a)) => a,
        _ => {
            return Err(error::create_error(
                MaybeRc::Owned("Array.prototype.push called on non-array"),
                value.vm,
            ))
        }
    };

    for value in value.args.into_iter().rev() {
        this_arr.elements.push(value);
    }

    Ok(value
        .vm
        .create_js_value(this_arr.elements.len() as f64)
        .into())
}
