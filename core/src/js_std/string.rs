use crate::unwrap_call_result;
use crate::vm::abstractions;
use crate::vm::value::function::CallResult;
use crate::vm::value::{function::CallContext, Value, ValueKind};
use std::cell::RefCell;
use std::rc::Rc;

pub fn string_constructor(_args: CallContext) -> Result<CallResult, Rc<RefCell<Value>>> {
    Ok(CallResult::Ready(Value::new(ValueKind::Undefined).into()))
}

macro_rules! to_generic_string {
    ($ctx:expr) => {
        if let Some(this) = &$ctx.function_call_response {
            Rc::clone(this)
        } else {
            let this = $ctx.receiver.as_ref();
            unwrap_call_result!(abstractions::conversions::to_string($ctx.vm, this))
        }
    };
}

pub fn char_at(ctx: CallContext) -> Result<CallResult, Rc<RefCell<Value>>> {
    // 2. Let S be ? ToString(O).
    let this = to_generic_string!(ctx);
    let this_ref = this.borrow();
    let this_s = this_ref.as_string().unwrap();

    // Let position be ? ToIntegerOrInfinity(pos).
    let position = {
        let maybe_pos = ctx.args.first().map(|x| x.borrow());
        abstractions::object::to_integer_or_infinity(maybe_pos.as_deref())?
    };

    // Let size be the length of S.
    let size = this_s.len();

    // If position < 0 or position ≥ size, return the empty String.
    if position < 0f64 || position >= size as f64 {
        return Ok(CallResult::Ready(
            ctx.vm.create_js_value(String::new()).into(),
        ));
    }

    // 6. Return the String value of length 1, containing one code unit from S, namely the code unit at index position.
    let bytes = this_s.as_bytes();
    // TODO: This is not correct. This only works if chars up to `position` are in the range 0-255.
    let ret = String::from(bytes[position as usize] as char);
    Ok(CallResult::Ready(ctx.vm.create_js_value(ret).into()))
}

pub fn char_code_at(ctx: CallContext) -> Result<CallResult, Rc<RefCell<Value>>> {
    // 2. Let S be ? ToString(O).
    let this = to_generic_string!(ctx);
    let this_ref = this.borrow();
    let this_s = this_ref.as_string().unwrap();

    // Let position be ? ToIntegerOrInfinity(pos).
    let position = {
        let maybe_pos = ctx.args.first().map(|x| x.borrow());
        abstractions::object::to_integer_or_infinity(maybe_pos.as_deref())?
    };

    // Let size be the length of S.
    let size = this_s.len();

    // If position < 0 or position ≥ size, return the empty String.
    if position < 0f64 || position >= size as f64 {
        return Ok(CallResult::Ready(
            ctx.vm.create_js_value(String::new()).into(),
        ));
    }

    // 6. Return the Number value for the numeric value of the code unit at index position within the String S.
    let bytes = this_s.as_bytes();
    let ret = bytes[position as usize] as f64;
    Ok(CallResult::Ready(ctx.vm.create_js_value(ret).into()))
}

#[derive(Default)]
pub struct EndsWith {
    pub this: Option<Rc<RefCell<Value>>>,
}

pub fn ends_with(ctx: CallContext) -> Result<CallResult, Rc<RefCell<Value>>> {
    let this = {
        let state = ctx.state.get_or_insert_as(EndsWith::default).unwrap();
        match state.this.clone() {
            Some(this) => this,
            None => {
                let this = to_generic_string!(ctx);
                state.this = Some(Rc::clone(&this));
                this
            }
        }
    };
    let this_ref = this.borrow();
    let this_s = this_ref.as_string().unwrap();

    let (search_str_cell, _) = {
        let mut arguments = ctx.arguments();

        let search_str = arguments.next();
        let end_position_ref = arguments.next().map(|x| x.borrow());
        let end_position =
            abstractions::object::to_integer_or_infinity(end_position_ref.as_deref())?;

        (search_str.cloned(), end_position)
    };

    let search_str_cell = {
        // No need to update state here
        if let Some(search_string) = ctx.function_call_response {
            search_string
        } else {
            unwrap_call_result!(abstractions::conversions::to_string(
                ctx.vm,
                search_str_cell.as_ref()
            ))
        }
    };
    let search_str_ref = search_str_cell.borrow();
    let search_str = search_str_ref.as_string().unwrap();

    let ret = this_s.ends_with(search_str);
    Ok(CallResult::Ready(ctx.vm.create_js_value(ret).into()))
}

pub fn index_of(_args: CallContext) -> Result<CallResult, Rc<RefCell<Value>>> {
    Ok(CallResult::Ready(Value::new(ValueKind::Undefined).into()))
}
