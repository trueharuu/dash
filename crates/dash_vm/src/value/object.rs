use std::{any::Any, borrow::Cow, cell::RefCell, collections::HashMap, fmt::Debug, ptr::addr_of};

use crate::{
    gc::{handle::Handle, trace::Trace},
    local::LocalScope,
    throw, Vm,
};

use super::{
    ops::abstractions::conversions::ValueConversion,
    primitive::{BuiltinCapabilities, Symbol},
    Typeof, Value, ValueContext,
};

// only here for the time being, will be removed later
fn __assert_trait_object_safety(_: Box<dyn Object>) {}

pub trait Object: Debug + Trace {
    fn get_property(&self, sc: &mut LocalScope, key: PropertyKey) -> Result<Value, Value>;

    fn set_property(&self, sc: &mut LocalScope, key: PropertyKey<'static>, value: PropertyValue) -> Result<(), Value>;

    fn delete_property(&self, sc: &mut LocalScope, key: PropertyKey) -> Result<Value, Value>;

    fn set_prototype(&self, sc: &mut LocalScope, value: Value) -> Result<(), Value>;

    fn get_prototype(&self, sc: &mut LocalScope) -> Result<Value, Value>;

    fn apply(
        &self,
        scope: &mut LocalScope,
        callee: Handle<dyn Object>,
        this: Value,
        args: Vec<Value>,
    ) -> Result<Value, Value>;

    fn construct(
        &self,
        scope: &mut LocalScope,
        callee: Handle<dyn Object>,
        this: Value,
        args: Vec<Value>,
    ) -> Result<Value, Value> {
        self.apply(scope, callee, this, args)
    }

    fn as_any(&self) -> &dyn Any;

    /**
     * Returns the own value as a trait object implementing the `BuiltinCapabilities` trait.
     * See docs on the mentioned trait for more details.
     *
     * Outside types should not (and usually cannot) implement this method and return Some.
     */
    fn as_builtin_capable(&self) -> Option<&dyn BuiltinCapabilities> {
        None
    }

    fn own_keys(&self) -> Result<Vec<Value>, Value>;

    fn type_of(&self) -> Typeof {
        Typeof::Object
    }
}

#[derive(Debug, Clone)]
pub struct NamedObject {
    prototype: RefCell<Option<Handle<dyn Object>>>,
    constructor: RefCell<Option<Handle<dyn Object>>>,
    values: RefCell<HashMap<PropertyKey<'static>, PropertyValue>>,
}

// TODO: optimization opportunity: some kind of Number variant for faster indexing without .to_string()
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum PropertyKey<'a> {
    String(Cow<'a, str>),
    Symbol(Symbol),
}

#[derive(Debug, Clone)]
pub enum PropertyValue {
    /// Accessor property
    Trap {
        get: Option<Handle<dyn Object>>,
        set: Option<Handle<dyn Object>>,
    },
    /// Static value property
    Static(Value),
}

impl PropertyValue {
    pub fn getter(get: Handle<dyn Object>) -> Self {
        Self::Trap {
            get: Some(get),
            set: None,
        }
    }

    pub fn setter(set: Handle<dyn Object>) -> Self {
        Self::Trap {
            set: Some(set),
            get: None,
        }
    }

    pub fn as_static(&self) -> Option<&Value> {
        match self {
            Self::Static(val) => Some(val),
            _ => None,
        }
    }

    pub fn get_or_apply(&self, sc: &mut LocalScope, this: Value) -> Result<Value, Value> {
        match self {
            Self::Static(value) => Ok(value.clone()),
            Self::Trap { get, .. } => match get {
                Some(handle) => handle.apply(sc, this, Vec::new()),
                None => Ok(Value::undefined()),
            },
        }
    }
}

unsafe impl Trace for PropertyValue {
    fn trace(&self) {
        match self {
            Self::Static(value) => value.trace(),
            Self::Trap { get, set } => {
                if let Some(get) = get {
                    get.trace();
                }
                if let Some(set) = set {
                    set.trace();
                }
            }
        }
    }
}

impl<'a> PropertyKey<'a> {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            PropertyKey::String(s) => Some(s.as_ref()),
            _ => None,
        }
    }
}

impl<'a> From<&'a str> for PropertyKey<'a> {
    fn from(s: &'a str) -> Self {
        PropertyKey::String(Cow::Borrowed(s))
    }
}

impl From<String> for PropertyKey<'static> {
    fn from(s: String) -> Self {
        PropertyKey::String(Cow::Owned(s))
    }
}

impl From<Symbol> for PropertyKey<'static> {
    fn from(s: Symbol) -> Self {
        PropertyKey::Symbol(s)
    }
}

impl<'a> PropertyKey<'a> {
    pub fn as_value(&self) -> Value {
        match self {
            PropertyKey::String(s) => Value::String(s.as_ref().into()),
            PropertyKey::Symbol(s) => Value::Symbol(s.clone()),
        }
    }

    pub fn from_value(sc: &mut LocalScope, value: Value) -> Result<Self, Value> {
        match value {
            Value::Symbol(s) => Ok(Self::Symbol(s.into())),
            other => {
                let key = other.to_string(sc)?;
                Ok(PropertyKey::String(ToString::to_string(&key).into()))
            }
        }
    }
}

impl NamedObject {
    pub fn new(vm: &mut Vm) -> Self {
        Self::with_values(vm, HashMap::new())
    }

    pub fn with_values(vm: &mut Vm, values: HashMap<PropertyKey<'static>, PropertyValue>) -> Self {
        let objp = vm.statics.object_prototype.clone();
        let objc = vm.statics.object_ctor.clone(); // TODO: function_ctor instead

        Self {
            prototype: RefCell::new(Some(objp)),
            constructor: RefCell::new(Some(objc)),
            values: RefCell::new(values),
        }
    }

    /// Creates an empty object with a null prototype
    pub fn null() -> Self {
        Self {
            prototype: RefCell::new(None),
            constructor: RefCell::new(None),
            values: RefCell::new(HashMap::new()),
        }
    }

    pub fn with_prototype_and_constructor(prototype: Handle<dyn Object>, ctor: Handle<dyn Object>) -> Self {
        Self {
            constructor: RefCell::new(Some(ctor)),
            prototype: RefCell::new(Some(prototype)),
            values: RefCell::new(HashMap::new()),
        }
    }

    pub fn get_raw_property(&self, pk: PropertyKey) -> Option<PropertyValue> {
        self.values.borrow().get(&pk).cloned()
    }
}

unsafe impl Trace for NamedObject {
    fn trace(&self) {
        let values = self.values.borrow();
        for value in values.values() {
            value.trace();
        }

        let prototype = self.prototype.borrow();
        if let Some(prototype) = &*prototype {
            prototype.trace();
        }

        let constructor = self.constructor.borrow();
        if let Some(constructor) = &*constructor {
            constructor.trace();
        }
    }
}

impl Object for NamedObject {
    fn get_property(&self, sc: &mut LocalScope, key: PropertyKey) -> Result<Value, Value> {
        if let PropertyKey::String(st) = &key {
            match st.as_ref() {
                "__proto__" => return self.get_prototype(sc),
                "constructor" => {
                    return Ok(self
                        .constructor
                        .borrow()
                        .as_ref()
                        .map(|x| Value::from(x.clone()))
                        .unwrap_or_undefined());
                }
                _ => {}
            }
        };

        let values = self.values.borrow();
        if let Some(value) = values.get(&key) {
            // TODO: this shouldnt be undefined
            return value.get_or_apply(sc, Value::undefined());
        }

        if let Some(prototype) = self.prototype.borrow().as_ref() {
            return prototype.get_property(sc, key);
        }

        Ok(Value::undefined())
    }

    fn set_property(&self, sc: &mut LocalScope, key: PropertyKey<'static>, value: PropertyValue) -> Result<(), Value> {
        match key.as_string() {
            Some("__proto__") => {
                return self.set_prototype(
                    sc,
                    match value {
                        PropertyValue::Static(value) => value,
                        _ => throw!(sc, "Prototype cannot be a trap"),
                    },
                )
            }
            Some("constructor") => {
                match value {
                    PropertyValue::Static(Value::Object(obj) | Value::External(obj)) => {
                        self.constructor.replace(Some(obj));
                        return Ok(());
                    }
                    _ => throw!(sc, "constructor is not an object"), // TODO: it doesn't need to be
                }
            }
            _ => {}
        };

        // TODO: check if we are invoking a setter

        let mut map = self.values.borrow_mut();
        map.insert(key, value);
        Ok(())
    }

    fn delete_property(&self, sc: &mut LocalScope, key: PropertyKey) -> Result<Value, Value> {
        let key = unsafe { &*addr_of!(key).cast::<PropertyKey<'static>>() };

        let mut values = self.values.borrow_mut();
        let value = values.remove(key);

        match value {
            Some(PropertyValue::Static(ref value @ (Value::Object(ref o) | Value::External(ref o)))) => {
                // If a GC'd value is being removed, put it in the LocalScope so it doesn't get removed too early
                sc.add_ref(o.clone());
                Ok(value.clone())
            }
            // Primitive values can just be returned normally
            Some(PropertyValue::Static(value)) => Ok(value),
            Some(PropertyValue::Trap { get, set }) => {
                // Accessors need to be added to the LocalScope too
                get.map(|v| sc.add_ref(v));
                set.map(|v| sc.add_ref(v));

                // Kind of unclear what to return here...
                // We can't invoke the getters/setters
                Ok(Value::undefined())
            }
            None => Ok(Value::undefined()),
        }
    }

    fn apply(
        &self,
        _sc: &mut LocalScope,
        _handle: Handle<dyn Object>,
        _this: Value,
        _args: Vec<Value>,
    ) -> Result<Value, Value> {
        Ok(Value::undefined())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn set_prototype(&self, sc: &mut LocalScope, value: Value) -> Result<(), Value> {
        match value {
            Value::Null(_) => self.prototype.replace(None),
            Value::Object(handle) => self.prototype.replace(Some(handle)),
            Value::External(handle) => self.prototype.replace(Some(handle)), // TODO: check that handle is an object
            _ => throw!(sc, "prototype must be an object"),
        };

        Ok(())
    }

    fn get_prototype(&self, _sc: &mut LocalScope) -> Result<Value, Value> {
        let prototype = self.prototype.borrow();
        match prototype.as_ref() {
            Some(handle) => Ok(Value::Object(handle.clone())),
            None => Ok(Value::null()),
        }
    }

    fn own_keys(&self) -> Result<Vec<Value>, Value> {
        let values = self.values.borrow();
        Ok(values.keys().map(PropertyKey::as_value).collect())
    }
}

impl Object for Handle<dyn Object> {
    fn get_property(&self, sc: &mut LocalScope, key: PropertyKey) -> Result<Value, Value> {
        (**self).get_property(sc, key)
    }

    fn set_property(&self, sc: &mut LocalScope, key: PropertyKey<'static>, value: PropertyValue) -> Result<(), Value> {
        (**self).set_property(sc, key, value)
    }

    fn delete_property(&self, sc: &mut LocalScope, key: PropertyKey) -> Result<Value, Value> {
        (**self).delete_property(sc, key)
    }

    fn set_prototype(&self, sc: &mut LocalScope, value: Value) -> Result<(), Value> {
        (**self).set_prototype(sc, value)
    }

    fn get_prototype(&self, sc: &mut LocalScope) -> Result<Value, Value> {
        (**self).get_prototype(sc)
    }

    fn apply(
        &self,
        scope: &mut LocalScope,
        callee: Handle<dyn Object>,
        this: Value,
        args: Vec<Value>,
    ) -> Result<Value, Value> {
        (**self).apply(scope, callee, this, args)
    }

    fn as_any(&self) -> &dyn Any {
        (**self).as_any()
    }

    fn own_keys(&self) -> Result<Vec<Value>, Value> {
        (**self).own_keys()
    }

    fn type_of(&self) -> Typeof {
        (**self).type_of()
    }

    fn as_builtin_capable(&self) -> Option<&dyn BuiltinCapabilities> {
        (**self).as_builtin_capable()
    }
}

impl Handle<dyn Object> {
    pub fn apply(&self, sc: &mut LocalScope, this: Value, args: Vec<Value>) -> Result<Value, Value> {
        let callee = self.clone();
        (**self).apply(sc, callee, this, args)
    }

    pub fn construct(&self, sc: &mut LocalScope, this: Value, args: Vec<Value>) -> Result<Value, Value> {
        let callee = self.clone();
        (**self).construct(sc, callee, this, args)
    }
}
