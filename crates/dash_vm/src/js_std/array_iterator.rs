use crate::throw;
use crate::value::array::ArrayIterator;
use crate::value::function::native::CallContext;
use crate::value::object::NamedObject;
use crate::value::object::Object;
use crate::value::object::PropertyValue;
use crate::value::Value;
use crate::value::ValueContext;

pub fn next(cx: CallContext) -> Result<Value, Value> {
    let iterator = match cx.this.downcast_ref::<ArrayIterator>() {
        Some(it) => it,
        None => throw!(cx.scope, TypeError, "Incompatible receiver"),
    };

    let next = iterator.next(cx.scope)?;
    let done = next.is_none();

    let obj = NamedObject::new(cx.scope);
    obj.set_property(
        cx.scope,
        "value".into(),
        PropertyValue::static_default(next.unwrap_or_undefined()),
    )?;
    obj.set_property(
        cx.scope,
        "done".into(),
        PropertyValue::static_default(Value::Boolean(done)),
    )?;

    Ok(cx.scope.register(obj).into())
}
