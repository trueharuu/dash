use std::sync::Arc;
use std::time::Duration;

use dash_middle::compiler::StaticImportKind;
use dash_middle::util::ThreadSafeStorage;
use dash_rt::event::EventMessage;
use dash_rt::module::ModuleLoader;
use dash_rt::state::State;
use dash_vm::gc::persistent::Persistent;
use dash_vm::local::LocalScope;
use dash_vm::throw;
use dash_vm::value::function::native::CallContext;
use dash_vm::value::function::Function;
use dash_vm::value::function::FunctionKind;
use dash_vm::value::object::NamedObject;
use dash_vm::value::object::Object;
use dash_vm::value::object::PropertyValue;
use dash_vm::value::ops::abstractions::conversions::ValueConversion;
use dash_vm::value::Value;

#[derive(Debug)]
pub struct TimersModule;

impl ModuleLoader for TimersModule {
    fn import(&self, sc: &mut LocalScope, _import_ty: StaticImportKind, path: &str) -> Result<Option<Value>, Value> {
        if path == "@std/timers" {
            let obj = NamedObject::new(sc);

            let set_timeout = Function::new(sc, Some("setTimeout".into()), FunctionKind::Native(set_timeout));
            let set_timeout = Value::Object(sc.register(set_timeout));

            obj.set_property(sc, "setTimeout".into(), PropertyValue::static_default(set_timeout))?;

            Ok(Some(Value::Object(sc.register(obj))))
        } else {
            Ok(None)
        }
    }
}

fn set_timeout(cx: CallContext) -> Result<Value, Value> {
    let callback = match cx.args.first() {
        Some(Value::Object(cb)) => cb.clone(),
        _ => throw!(cx.scope, TypeError, "missing callback function argument"),
    };

    let callback = Arc::new(ThreadSafeStorage::new(Persistent::new(callback)));

    let delay = match cx.args.get(1) {
        Some(delay) => delay.to_int32(cx.scope)? as u64,
        None => throw!(cx.scope, TypeError, "Missing delay argument"),
    };

    let state = State::from_vm(cx.scope);
    let tx = state.event_sender();
    let tid = state.active_tasks().add();

    state.rt_handle().spawn(async move {
        let tx2 = tx.clone();
        tokio::time::sleep(Duration::from_millis(delay)).await;

        tx.send(EventMessage::ScheduleCallback(Box::new(move |rt| {
            let mut sc = LocalScope::new(rt.vm_mut());
            let callback = callback.get();

            if let Err(err) = callback.apply(&mut sc, Value::undefined(), Vec::new()) {
                eprintln!("Unhandled error in timer callback: {err:?}");
            }

            tx2.send(EventMessage::RemoveTask(tid));
        })));
    });

    Ok(Value::undefined())
}
