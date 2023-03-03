use dash_middle::compiler::StaticImportKind;
use dash_rt::module::ModuleLoader;
use dash_vm::local::LocalScope;
use dash_vm::value::function::native::CallContext;
use dash_vm::value::function::Function;
use dash_vm::value::function::FunctionKind;
use dash_vm::value::object::NamedObject;
use dash_vm::value::object::Object;
use dash_vm::value::object::PropertyValue;
use dash_vm::value::ops::abstractions::conversions::ValueConversion;
use dash_vm::value::Value;
use dash_vm::value::ValueContext;
use db::Database;

mod db;
mod worker;

#[derive(Debug)]
pub struct SqlxModule;

impl ModuleLoader for SqlxModule {
    fn import(&self, sc: &mut LocalScope, _import_ty: StaticImportKind, path: &str) -> Result<Option<Value>, Value> {
        if path != "@std/sqlx" {
            return Ok(None);
        }

        let exports = NamedObject::new(sc);
        let connection_ctor = Function::new(sc, Some("Connection".into()), FunctionKind::Native(connection_ctor));
        let connection_ctor = sc.register(connection_ctor);
        exports.set_property(
            sc,
            "Connection".into(),
            PropertyValue::static_default(Value::Object(connection_ctor)),
        )?;

        let exports = sc.register(exports);

        Ok(Some(Value::Object(exports)))
    }
}

fn connection_ctor(cx: CallContext) -> Result<Value, Value> {
    let url = cx.args.first().unwrap_or_undefined().to_string(cx.scope)?;
    let db = Database::connect(&url, cx.scope);
    let db = cx.scope.register(db);
    Ok(Value::Object(db))
}
