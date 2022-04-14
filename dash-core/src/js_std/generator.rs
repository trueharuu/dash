use std::mem;

use crate::throw;
use crate::vm::dispatch::HandleResult;
use crate::vm::frame::Frame;
use crate::vm::local::LocalScope;
use crate::vm::value::function::generator::GeneratorIterator;
use crate::vm::value::function::generator::GeneratorState;
use crate::vm::value::function::native::CallContext;
use crate::vm::value::function::Function;
use crate::vm::value::object::NamedObject;
use crate::vm::value::object::Object;
use crate::vm::value::Value;
use crate::vm::value::ValueContext;

fn as_generator<'a>(scope: &mut LocalScope, value: &'a Value) -> Result<&'a GeneratorIterator, Value> {
    let generator = match value {
        Value::Object(o) => o.as_any().downcast_ref::<GeneratorIterator>(),
        _ => None,
    };

    let generator = match generator {
        Some(it) => it,
        None => throw!(scope, "Incompatible receiver"),
    };

    Ok(generator)
}

pub fn next(cx: CallContext) -> Result<Value, Value> {
    let frame = {
        let generator = as_generator(cx.scope, &cx.this)?;

        let (ip, old_stack) = match &mut *generator.state().borrow_mut() {
            GeneratorState::Finished => return create_generator_value(cx.scope, true, None),
            GeneratorState::Running { ip, stack } => (*ip, mem::take(stack)),
        };

        let function = match generator
            .function()
            .as_any()
            .downcast_ref::<Function>()
            .and_then(|fun| fun.kind().as_generator())
        {
            Some(uf) => uf.function(),
            _ => throw!(cx.scope, "Incompatible generator function"),
        };

        let current_sp = cx.scope.stack_size();
        cx.scope.try_extend_stack(old_stack)?;

        let mut frame = Frame::from_function(function, cx.scope);
        frame.ip = ip;
        frame.sp = current_sp;

        frame
    };

    let result = cx.scope.vm.execute_frame(frame)?;
    let generator = as_generator(cx.scope, &cx.this)?;

    match result {
        HandleResult::Return(value) => {
            generator.state().replace(GeneratorState::Finished);

            create_generator_value(cx.scope, true, Some(value))
        }
        HandleResult::Yield(value) => {
            let frame = cx.scope.pop_frame().expect("Generator frame is missing");
            let stack = cx.scope.drain_stack(frame.sp..).collect();

            generator.state().replace(GeneratorState::Running {
                ip: frame.ip + 1,
                stack,
            });

            create_generator_value(cx.scope, false, Some(value))
        }
    }
}

fn create_generator_value(scope: &mut LocalScope, done: bool, value: Option<Value>) -> Result<Value, Value> {
    let obj = NamedObject::new(scope);
    obj.set_property(scope, "done".into(), done.into())?;
    obj.set_property(scope, "value".into(), value.unwrap_or_undefined())?;
    Ok(scope.register(obj).into())
}
