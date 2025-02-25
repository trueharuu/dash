use std::mem;

use crate::dispatch::HandleResult;
use crate::frame::Frame;
use crate::local::LocalScope;
use crate::throw;
use crate::value::function::generator::as_generator;
use crate::value::function::generator::GeneratorState;
use crate::value::function::native::CallContext;
use crate::value::function::Function;
use crate::value::function::FunctionKind;
use crate::value::object::NamedObject;
use crate::value::object::Object;
use crate::value::object::PropertyValue;
use crate::value::Value;
use crate::value::ValueContext;

pub fn next(cx: CallContext) -> Result<Value, Value> {
    let arg = cx.args.first().unwrap_or_undefined();
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
            .map(|fun| fun.kind())
        {
            Some(FunctionKind::Generator(gen)) => gen.function(),
            Some(FunctionKind::Async(fun)) => fun.inner().function(),
            _ => throw!(cx.scope, TypeError, "Incompatible generator function"),
        };

        let current_sp = cx.scope.stack_size();
        cx.scope.try_extend_stack(old_stack)?;

        let mut frame = Frame::from_function(None, function, false, false);
        frame.set_ip(ip);
        frame.set_sp(current_sp);

        if !generator.did_run() {
            // If it hasn't run before, do the stack space management initially (push undefined values for locals)
            // We only want to do this if the generator hasn't run already, because the locals are already in `old_stack`
            cx.scope.pad_stack_for_frame(&frame);
        } else {
            // Generator did run before. Push the yielded value onto the stack, which will be what the yield expression
            // evaluates to.
            cx.scope.stack.push(arg);
        }

        frame
    };

    // Generators work a bit different from normal functions, so we do the stack padding management ourselves here
    let result = cx.scope.execute_frame_raw(frame)?;
    let generator = as_generator(cx.scope, &cx.this)?;

    match result {
        HandleResult::Return(value) => {
            generator.state().replace(GeneratorState::Finished);

            create_generator_value(cx.scope, true, Some(value))
        }
        HandleResult::Yield(value) | HandleResult::Await(value) => {
            // Async functions are desugared to generators, so `await` is treated equivalent to `yield`, for now...
            let frame = cx.scope.pop_frame().expect("Generator frame is missing");
            let stack = cx.scope.drain_stack(frame.sp..).collect::<Vec<_>>();

            generator
                .state()
                .replace(GeneratorState::Running { ip: frame.ip, stack });

            create_generator_value(cx.scope, false, Some(value))
        }
    }
}

fn create_generator_value(scope: &mut LocalScope, done: bool, value: Option<Value>) -> Result<Value, Value> {
    let obj = NamedObject::new(scope);
    obj.set_property(scope, "done".into(), PropertyValue::static_default(done.into()))?;
    obj.set_property(
        scope,
        "value".into(),
        PropertyValue::static_default(value.unwrap_or_undefined()),
    )?;
    Ok(scope.register(obj).into())
}
