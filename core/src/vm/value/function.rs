use crate::compiler::compiler::FunctionKind as CompilerFunctionKind;
use crate::gc::Handle;
use crate::vm::instruction::Constant;
use crate::vm::{instruction::Instruction, upvalue::Upvalue, VM};
use core::fmt::{self, Debug, Formatter};
use std::collections::HashMap;

use super::object::Object;
use super::Value;

/// A native function that can be called from JavaScript code
pub type NativeFunctionCallback =
    for<'a> fn(CallContext<'a>) -> Result<Handle<Value>, Handle<Value>>;

/// Represents whether a function can be invoked as a constructor
#[derive(Debug, Clone, Copy)]
pub enum Constructor {
    /// Function can be invoked with or without the new keyword
    Any,
    /// Function can be invoked as a constructor using `new`, but also works without
    Ctor,
    /// Function is not a constructor and cannot be called with `new`
    NoCtor,
}

impl Constructor {
    /// Returns whether the function is constructable
    pub fn constructable(&self) -> bool {
        matches!(self, Constructor::Ctor | Constructor::Any)
    }
}

/// Native function call context
pub struct CallContext<'a> {
    /// A mutable reference to the underlying VM
    pub vm: &'a mut VM,
    /// Arguments that were passed to this function
    ///
    /// Note that the order of arguments is last to first,
    /// i.e. the first argument is the last item of the vec
    /// due to the nature of a stack
    pub args: &'a mut Vec<Handle<Value>>,
    /// The receiver (`this`) value
    pub receiver: Option<Handle<Value>>,
    /// Whether this function call is invoked as a constructor call
    pub ctor: bool,
}

impl<'a> CallContext<'a> {
    /// An iterator over arguments in fixed order
    pub fn arguments(&self) -> impl Iterator<Item = &Handle<Value>> {
        self.args.iter()
    }
}

/// The type of a function at runtime
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum FunctionType {
    /// Top frame
    ///
    /// This is typically the initial script
    Top,
    /// A normal function
    Function,
    /// A closure
    Closure,
    /// A JavaScript module
    Module,
    /// A JavaScript generator
    Generator,
    /// A JavaScript async generator
    AsyncGenerator,
}

impl FunctionType {
    /// Returns whether this function is a generator
    pub fn is_generator(&self) -> bool {
        matches!(self, FunctionType::Generator | FunctionType::AsyncGenerator)
    }
}

impl From<CompilerFunctionKind> for FunctionType {
    fn from(kind: CompilerFunctionKind) -> Self {
        match kind {
            CompilerFunctionKind::Function => Self::Function,
            CompilerFunctionKind::Generator => Self::Generator,
            CompilerFunctionKind::Module => Self::Module,
        }
    }
}

/// The receiver (`this`) of a function
#[derive(Debug, Clone)]
pub enum Receiver {
    /// Receiver is pinned and may not be changed
    Pinned(Handle<Value>),
    /// Receiver is bound to a specific value
    Bound(Handle<Value>),
}

impl Receiver {
    /// Returns the inner `this` value
    pub fn get(&self) -> &Handle<Value> {
        match self {
            Self::Pinned(p) => p,
            Self::Bound(b) => b,
        }
    }

    /// Rebinds this
    // TODO: this should be a no op if self is pinned
    pub fn bind(&mut self, recv: Receiver) {
        *self = recv;
    }

    /// Rebinds this by consuming the Receiver and returning it
    pub fn rebind(self, recv: Receiver) -> Self {
        recv
    }
}

/// A closure, wrapping a user function with values from the upper scope
#[derive(Debug, Clone)]
pub struct Closure {
    /// The inner value
    pub func: UserFunction,
    /// Values from the upper scope
    pub upvalues: Vec<Upvalue>,
}

impl Closure {
    /// Creates a new closure
    pub fn new(func: UserFunction) -> Self {
        Self {
            func,
            upvalues: Vec::new(),
        }
    }

    /// Creates a new closure given a user function and a vector of upvalues
    pub fn with_upvalues(func: UserFunction, upvalues: Vec<Upvalue>) -> Self {
        Self { func, upvalues }
    }
}

/// A JavaScript function created in JavaScript code
#[derive(Debug, Clone)]
pub struct UserFunction {
    /// Whether this function is constructable
    pub ctor: Constructor,
    /// The prototype of this function
    pub prototype: Option<Handle<Value>>,
    /// Number of parameters this function takes
    pub params: u32,
    /// The receiver of this function
    pub receiver: Option<Receiver>,
    /// The type of function
    pub ty: FunctionType,
    /// Function bytecode
    pub buffer: Box<[Instruction]>,
    /// A pool of constants
    pub constants: Box<[Constant]>,
    /// The name of this function
    pub name: Option<String>,
    /// Number of values
    pub upvalues: u32,
}

impl UserFunction {
    /// Creates a new user function
    pub fn new(
        buffer: impl Into<Box<[Instruction]>>,
        params: u32,
        ty: FunctionType,
        upvalues: u32,
        ctor: Constructor,
        constants: impl Into<Box<[Constant]>>,
    ) -> Self {
        Self {
            buffer: buffer.into(),
            constants: constants.into(),
            params,
            name: None,
            ty,
            receiver: None,
            ctor,
            upvalues,
            prototype: None,
        }
    }

    /// Call `bind` on the underlying [Receiver]
    pub fn bind(&mut self, new_recv: Receiver) {
        if let Some(recv) = &mut self.receiver {
            recv.bind(new_recv);
        } else {
            self.receiver = Some(new_recv);
        }
    }

    /// Call `rebind` on the underlying [Receiver]
    pub fn rebind(mut self, new_recv: Receiver) -> Self {
        if let Some(recv) = &mut self.receiver {
            recv.bind(new_recv);
        } else {
            self.receiver = Some(new_recv);
        }
        self
    }

    /// Returns whether this function is contructable
    pub fn constructable(&self) -> bool {
        self.ctor.constructable() && !matches!(self.ty, FunctionType::Generator)
    }

    /// Gets the prototype of this function, or sets it
    pub fn get_or_set_prototype(&mut self, this: &Handle<Value>, vm: &VM) -> Handle<Value> {
        self.prototype
            .get_or_insert_with(|| {
                let mut o = vm.create_object();
                o.constructor = Some(Handle::clone(this));
                o.into_handle(vm)
            })
            .clone()
    }
}

/// A native function that can be called from JavaScript code
pub struct NativeFunction {
    /// Whether this function can be invoked as a constructor
    pub ctor: Constructor,
    /// The name of this function
    pub name: &'static str,
    /// A pointer to the function
    pub func: NativeFunctionCallback,
    /// The receiver of this function
    pub receiver: Option<Receiver>,
    /// The prototype of this function
    pub prototype: Option<Handle<Value>>,
}

impl NativeFunction {
    /// Creates a new native function
    pub fn new(
        name: &'static str,
        func: NativeFunctionCallback,
        receiver: Option<Receiver>,
        ctor: Constructor,
    ) -> Self {
        Self {
            ctor,
            name,
            func,
            receiver,
            prototype: None,
        }
    }

    /// Gets the prototype of this function, or sets it if not yet set
    pub fn get_or_set_prototype(&mut self, this: &Handle<Value>, vm: &VM) -> Handle<Value> {
        self.prototype
            .get_or_insert_with(|| {
                let mut o = vm.create_object();
                o.constructor = Some(Handle::clone(this));
                o.into_handle(vm)
            })
            .clone()
    }
}

impl Clone for NativeFunction {
    fn clone(&self) -> Self {
        Self {
            prototype: self.prototype.clone(),
            ctor: self.ctor,
            func: self.func,
            name: self.name,
            receiver: self.receiver.clone(),
        }
    }
}

impl Debug for NativeFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeFunction")
            .field("name", &self.name)
            .finish()
    }
}

/// A JavaScript module
#[derive(Debug, Clone)]
pub struct Module {
    /// Module bytecode, if present
    pub buffer: Option<Box<[Instruction]>>,
    /// The exports namespace
    pub exports: Exports,
    /// Compile-time constants used within the module
    pub constants: Box<[Constant]>,
}

impl Module {
    /// Creates a new module
    pub fn new(
        buffer: impl Into<Box<[Instruction]>>,
        constants: impl Into<Box<[Constant]>>,
    ) -> Self {
        Self {
            buffer: Some(buffer.into()),
            exports: Exports::default(),
            constants: constants.into(),
        }
    }
}

/// JavaScript module exports
#[derive(Debug, Clone, Default)]
pub struct Exports {
    /// The default export, if set
    pub default: Option<Handle<Value>>,
    /// Named exports
    pub named: HashMap<Box<str>, Handle<Value>>,
}

/// The kind of this function
#[derive(Debug, Clone)]
pub enum FunctionKind {
    /// A closure
    Closure(Closure),
    /// A user function
    User(UserFunction),
    /// A native function
    Native(NativeFunction),
    /// A JavaScript module
    Module(Module),
}

impl ToString for FunctionKind {
    fn to_string(&self) -> String {
        match self {
            // Users cannot access modules directly
            Self::Module(_) => unreachable!(),
            Self::Native(n) => format!("function {}() {{ [native code] }}", n.name),
            Self::User(u) => format!("function {}() {{ ... }}", u.name.as_deref().unwrap_or("")),
            Self::Closure(c) => {
                let func = &c.func;
                format!(
                    "function{} {}() {{ [code] }}",
                    if matches!(func.ty, FunctionType::Generator) {
                        "*"
                    } else {
                        ""
                    },
                    func.name.as_deref().unwrap_or("")
                )
            }
        }
    }
}

impl FunctionKind {
    /// Returns the name of this function, if present
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Closure(c) => c.func.name.as_deref(),
            Self::User(u) => u.name.as_deref(),
            Self::Native(n) => Some(n.name),
            _ => None,
        }
    }

    /// Returns a [Handle] to the prototype of this function, if it has one
    pub fn get_or_set_prototype(&mut self, this: &Handle<Value>, vm: &VM) -> Option<Handle<Value>> {
        match self {
            Self::Closure(c) => Some(c.func.get_or_set_prototype(this, vm)),
            Self::User(u) => Some(u.get_or_set_prototype(this, vm)),
            Self::Native(n) => Some(n.get_or_set_prototype(this, vm)),
            _ => None,
        }
    }

    pub(crate) fn mark(&self) {
        match self {
            FunctionKind::Module(module) => {
                if let Some(handle) = &module.exports.default {
                    Value::mark(handle)
                }

                for handle in module.exports.named.values() {
                    Value::mark(handle)
                }
            }
            FunctionKind::Native(native) => {
                if let Some(handle) = &native.receiver {
                    Value::mark(handle.get())
                }

                if let Some(handle) = &native.prototype {
                    Value::mark(handle)
                }
            }
            FunctionKind::User(func) => {
                // Constants need to be marked, otherwise constants_gc will GC these
                for constant in func.constants.iter() {
                    if let Constant::JsValue(handle) = constant {
                        Value::mark(handle);
                    }
                }

                if let Some(handle) = &func.receiver {
                    Value::mark(handle.get())
                }

                if let Some(handle) = &func.prototype {
                    Value::mark(handle)
                }
            }
            FunctionKind::Closure(closure) => {
                if let Some(handle) = &closure.func.receiver {
                    Value::mark(handle.get())
                }

                if let Some(handle) = &closure.func.prototype {
                    Value::mark(handle)
                }

                for upvalue in &closure.upvalues {
                    upvalue.mark_visited();
                }
            }
        }
    }

    /// Attempts to create an object with its [[Prototype]] set to this
    /// functions prototype
    pub fn construct(&mut self, this: &Handle<Value>, vm: &VM) -> Value {
        let mut o = Value::from(Object::Ordinary);
        o.proto = self.get_or_set_prototype(this, vm);
        o.constructor = Some(Handle::clone(this));
        o
    }

    /// Sets the prototype of this function
    pub fn set_prototype(&mut self, proto: Handle<Value>) {
        match self {
            Self::Closure(c) => c.func.prototype = Some(proto),
            Self::User(u) => u.prototype = Some(proto),
            Self::Native(n) => n.prototype = Some(proto),
            _ => {}
        };
    }

    /// Returns self as a closure, if it is one
    pub fn as_closure(&self) -> Option<&Closure> {
        match self {
            Self::Closure(c) => Some(c),
            _ => None,
        }
    }

    /// Returns self as an owned closure, if it is one
    pub fn into_closure(self) -> Option<Closure> {
        match self {
            Self::Closure(c) => Some(c),
            _ => None,
        }
    }

    /// Returns self as a user function, if it is one
    pub fn as_user(&self) -> Option<&UserFunction> {
        match self {
            Self::User(u) => Some(u),
            Self::Closure(c) => Some(&c.func),
            _ => None,
        }
    }

    /// Returns self as an owned user function, if it is one
    pub fn into_user(self) -> Option<UserFunction> {
        match self {
            Self::User(u) => Some(u),
            _ => None,
        }
    }

    /// Returns self as a native function, if it is one
    pub fn as_native(&self) -> Option<&NativeFunction> {
        match self {
            Self::Native(n) => Some(n),
            _ => None,
        }
    }

    /// Returns self as an owned native function, if it is one
    pub fn into_native(self) -> Option<NativeFunction> {
        match self {
            Self::Native(n) => Some(n),
            _ => None,
        }
    }

    /// Returns self as a JavaScript module, if it is one
    pub fn as_module(&self) -> Option<&Module> {
        match self {
            Self::Module(m) => Some(m),
            _ => None,
        }
    }

    /// Returns self as a mutable reference to the underlying JavaScript module,
    /// if it is one
    pub fn as_module_mut(&mut self) -> Option<&mut Module> {
        match self {
            Self::Module(m) => Some(m),
            _ => None,
        }
    }

    /// Returns self as an owned JavaScript module, if it is one
    pub fn into_module(self) -> Option<Module> {
        match self {
            Self::Module(m) => Some(m),
            _ => None,
        }
    }

    /// Returns a reference to constants used by this function
    pub(crate) fn constants(&self) -> Option<&[Constant]> {
        match self {
            Self::User(u) => Some(&u.constants),
            Self::Closure(c) => Some(&c.func.constants),
            Self::Module(m) => Some(&m.constants),
            _ => None,
        }
    }
}
