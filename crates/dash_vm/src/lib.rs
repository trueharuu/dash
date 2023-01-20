
use std::{fmt, ops::RangeBounds, vec::Drain, mem};

use crate::{
    gc::{handle::Handle, trace::Trace, Gc},
    value::function::Function,
};

use self::{
    dispatch::HandleResult,
    external::Externals,
    frame::{Exports, Frame, FrameState, TryBlock},
    local::LocalScope,
    params::VmParams,
    statics::Statics,
    value::{
        object::{Object, PropertyValue},
        Value,
    },
};

use dash_middle::compiler::instruction::Instruction;
use util::unlikely;
use value::{promise::{Promise, PromiseState}, ValueContext, function::bound::BoundFunction, PureBuiltin, object::NamedObject};

#[cfg(feature = "jit")]
mod jit;

pub mod dispatch;
#[cfg(feature = "eval")]
pub mod eval;
pub mod external;
pub mod frame;
pub mod gc;
pub mod js_std;
pub mod local;
pub mod params;
pub mod statics;
pub mod util;
pub mod value;
mod macros;
#[cfg(test)]
mod test;

pub const MAX_FRAME_STACK_SIZE: usize = 1024;
pub const MAX_STACK_SIZE: usize = 8192;
const DEFAULT_GC_OBJECT_COUNT_THRESHOLD: usize = 8192;

pub struct Vm {
    frames: Vec<Frame>,
    async_tasks: Vec<Handle<dyn Object>>,
    stack: Vec<Value>,
    gc: Gc<dyn Object>,
    global: Handle<dyn Object>,
    externals: Externals,
    statics: Box<Statics>, // TODO: we should box this... maybe?
    try_blocks: Vec<TryBlock>,
    params: VmParams,
    gc_object_threshold: usize,
    /// Keeps track of the "purity" of the builtins of this VM.
    /// Purity here refers to whether builtins have been (in one way or another) mutated.
    /// Removing a property from the global object (e.g. `Math`) or any other builtin,
    /// or adding a property to a builtin, will cause this to be set to `false`, which in turn
    /// will disable many optimizations such as specialized intrinsics.
    builtins_pure: bool,
    #[cfg(feature = "jit")]
    jit: jit::Frontend
}


impl Vm {
    pub fn new(params: VmParams) -> Self {
        let mut gc = Gc::new();
        let statics = Statics::new(&mut gc);
        // TODO: global __proto__ and constructor
        let global = gc.register(PureBuiltin::new(NamedObject::null())); 
        let gc_object_threshold = params
            .initial_gc_object_threshold()
            .unwrap_or(DEFAULT_GC_OBJECT_COUNT_THRESHOLD);

        let mut vm = Self {
            frames: Vec::new(),
            async_tasks: Vec::new(),
            stack: Vec::with_capacity(512),
            gc,
            global,
            externals: Externals::new(),
            statics: Box::new(statics),
            try_blocks: Vec::new(),
            params,
            gc_object_threshold,
            builtins_pure: true,

            #[cfg(feature = "jit")]
            jit: jit::Frontend::new(),
        };
        vm.prepare();
        vm
    }

    pub fn global(&self) -> Handle<dyn Object> {
        self.global.clone()
    }

    /// Prepare the VM for execution.
    #[rustfmt::skip]
    fn prepare(&mut self) {
        fn set_fn_prototype(v: &dyn Object, proto: &Handle<dyn Object>, name: &str) {
            let fun = v.as_any().downcast_ref::<Function>().unwrap();
            fun.set_name(name.into());
            fun.set_fn_prototype(proto.clone());
        }

        let mut scope = LocalScope::new(self);

        let global = scope.global.clone();

        /// #[prototype] - Internal [[Prototype]] field for this value
        /// #[fn_prototype] - Only valid on function values
        ///                   This will set the [[Prototype]] field of the function
        /// #[properties] - "Reference" properties (i.e. object of some kind)
        /// #[symbols] - Symbol properties (e.g. @@iterator)
        /// #[fields] - Primitive fields (e.g. PI: 3.1415)
        macro_rules! register_builtin_type {
            (
                $base:expr, {
                    #[prototype] $prototype:expr;
                    #[constructor] $constructor:expr;
                    $(
                        #[fn_prototype] $fnprototype:expr;
                        #[fn_name] $fnname:ident;
                    )?
                    $( #[properties] $( $prop:ident: $prop_path:expr; )+ )?
                    $( #[symbols] $( $symbol:expr => $symbol_path:expr; )+ )?
                    $( #[fields] $( $field:ident: $value:expr; )+ )?
                }
            ) => {{
                let base = $base.clone();

                // Prototype
                {
                    let proto = $prototype.clone();
                    let constructor = $constructor.clone();
                    base.set_property(&mut scope, "constructor".into(), PropertyValue::static_default(constructor.into())).unwrap();
                    base.set_prototype(&mut scope, proto.into()).unwrap();
                }

                // Properties
                $(
                    $({
                        let method = stringify!($prop);
                        let path = $prop_path.clone();
                        register_builtin_type!(path, {
                            #[prototype] scope.statics.function_proto;
                            #[constructor] scope.statics.function_ctor;
                        });
                        base.set_property(&mut scope, method.into(), PropertyValue::static_default(path.into())).unwrap();
                    })+
                )?

                // Symbols
                $(
                    $({
                        let method = $symbol.clone();
                        let path = $symbol_path.clone();
                        register_builtin_type!(path, {
                            #[prototype] scope.statics.function_proto;
                            #[constructor] scope.statics.function_ctor;
                        });
                        base.set_property(&mut scope, method.into(), PropertyValue::static_default(path.into())).unwrap();
                    })+
                )?

                // Fields
                $(
                    $({
                        let method = stringify!($field);
                        let value = $value.clone();
                        base.set_property(&mut scope, method.into(), PropertyValue::static_default(value.into())).unwrap();
                    })+
                )?

                // Function prototype
                $(
                    set_fn_prototype(&base, &$fnprototype, stringify!($fnname));
                )?

                base
            }}            
        }

        let function_ctor = register_builtin_type!(scope.statics.function_ctor, {
            #[prototype] scope.statics.function_proto;
            #[constructor] scope.statics.function_ctor;
            #[fn_prototype] scope.statics.function_proto;
            #[fn_name] Function;
        });

        let function_proto = register_builtin_type!(scope.statics.function_proto, {
            #[prototype] scope.statics.object_prototype;
            #[constructor] function_ctor;

            #[properties]
            bind: scope.statics.function_bind;
            call: scope.statics.function_call;
        });

        let object_ctor = register_builtin_type!(scope.statics.object_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.object_prototype;
            #[fn_name] Object;

            #[properties]
            create: scope.statics.object_create;
            keys: scope.statics.object_keys;
            getOwnPropertyDescriptor: scope.statics.object_get_own_property_descriptor;
            getOwnPropertyDescriptors: scope.statics.object_get_own_property_descriptors;
        });

        let object_proto = register_builtin_type!(scope.statics.object_prototype, {
            #[prototype] Value::null();
            #[constructor] object_ctor;

            #[properties]
            toString: scope.statics.object_to_string;
            hasOwnProperty: scope.statics.object_has_own_property;
        });

        let console = register_builtin_type!(scope.statics.console, {
            #[prototype] scope.statics.object_prototype;
            #[constructor] object_ctor;

            #[properties]
            log: scope.statics.console_log;
        });

        let math = register_builtin_type!(scope.statics.math, {
            #[prototype] scope.statics.object_prototype;
            #[constructor] object_ctor;

            #[properties]
            floor: scope.statics.math_floor;
            abs: scope.statics.math_abs;
            acos: scope.statics.math_acos;
            acosh: scope.statics.math_acosh;
            asin: scope.statics.math_asin;
            asinh: scope.statics.math_asinh;
            atan: scope.statics.math_atan;
            atanh: scope.statics.math_atanh;
            atan2: scope.statics.math_atan2;
            cbrt: scope.statics.math_cbrt;
            ceil: scope.statics.math_ceil;
            clz32: scope.statics.math_clz32;
            cos: scope.statics.math_cos;
            cosh: scope.statics.math_cosh;
            exp: scope.statics.math_exp;
            expm1: scope.statics.math_expm1;
            log: scope.statics.math_log;
            log1p: scope.statics.math_log1p;
            log10: scope.statics.math_log10;
            log2: scope.statics.math_log2;
            round: scope.statics.math_round;
            sin: scope.statics.math_sin;
            sinh: scope.statics.math_sinh;
            sqrt: scope.statics.math_sqrt;
            tan: scope.statics.math_tan;
            tanh: scope.statics.math_tanh;
            trunc: scope.statics.math_trunc;
            random: scope.statics.math_random;

            #[fields]
            PI: Value::number(std::f64::consts::PI);
        });

        let number_ctor = register_builtin_type!(scope.statics.number_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.number_prototype;
            #[fn_name] Number;

            #[properties]
            isFinite: scope.statics.number_is_finite;
            isNaN: scope.statics.number_is_nan;
            isSafeInteger: scope.statics.number_is_safe_integer;
        });

        register_builtin_type!(scope.statics.number_prototype, {
            #[prototype] object_proto;
            #[constructor] number_ctor;
            
            #[properties]
            toString: scope.statics.number_tostring;
            toFixed: scope.statics.number_to_fixed;
        });

        let boolean_ctor = register_builtin_type!(scope.statics.boolean_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.boolean_prototype;
            #[fn_name] Boolean;
        });

        register_builtin_type!(scope.statics.boolean_prototype, {
            #[prototype] object_proto;
            #[constructor] boolean_ctor;

            #[properties]
            toString: scope.statics.boolean_tostring;
            valueOf: scope.statics.boolean_valueof;
        });

        let string_ctor = register_builtin_type!(scope.statics.string_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.string_prototype;
            #[fn_name] String;
            #[properties]
            fromCharCode: scope.statics.string_from_char_code;
        });
        
        register_builtin_type!(scope.statics.string_prototype, {
            #[prototype] object_proto;
            #[constructor] string_ctor;

            #[properties]
            toString: scope.statics.string_tostring;
            charAt: scope.statics.string_char_at;
            charCodeAt: scope.statics.string_char_code_at;
            concat: scope.statics.string_concat;
            endsWith: scope.statics.string_ends_with;
            startsWith: scope.statics.string_starts_with;
            includes: scope.statics.string_includes;
            indexOf: scope.statics.string_index_of;
            lastIndexOf: scope.statics.string_last_index_of;
            padEnd: scope.statics.string_pad_end;
            padStart: scope.statics.string_pad_start;
            repeat: scope.statics.string_repeat;
            replace: scope.statics.string_replace;
            replaceAll: scope.statics.string_replace_all;
            split: scope.statics.string_split;
            toLowerCase: scope.statics.string_to_lowercase;
            toUpperCase: scope.statics.string_to_uppercase;
            big: scope.statics.string_big;
            blink: scope.statics.string_blink;
            bold: scope.statics.string_bold;
            fixed: scope.statics.string_fixed;
            italics: scope.statics.string_italics;
            strike: scope.statics.string_strike;
            sub: scope.statics.string_sub;
            sup: scope.statics.string_sup;
            fontcolor: scope.statics.string_fontcolor;
            fontsize: scope.statics.string_fontsize;
            link: scope.statics.string_link;
            trim: scope.statics.string_trim;
            trimStart: scope.statics.string_trim_start;
            trimEnd: scope.statics.string_trim_end;
            substr: scope.statics.string_substr;
            substring: scope.statics.string_substring;
        });

        let array_ctor = register_builtin_type!(scope.statics.array_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.array_prototype;
            #[fn_name] Array;
        });
        
        register_builtin_type!(scope.statics.array_prototype, {
            #[prototype] object_proto;
            #[constructor] array_ctor;

            #[properties]
            toString: scope.statics.array_tostring;
            join: scope.statics.array_join;
            values: scope.statics.array_values;
            at: scope.statics.array_at;
            concat: scope.statics.array_concat;
            entries: scope.statics.array_entries;
            keys: scope.statics.array_keys;
            every: scope.statics.array_every;
            some: scope.statics.array_some;
            fill: scope.statics.array_fill;
            filter: scope.statics.array_filter;
            reduce: scope.statics.array_reduce;
            find: scope.statics.array_find;
            findIndex: scope.statics.array_find_index;
            flat: scope.statics.array_flat;
            forEach: scope.statics.array_for_each;
            includes: scope.statics.array_includes;
            indexOf: scope.statics.array_index_of;
            map: scope.statics.array_map;
            pop: scope.statics.array_pop;
            push: scope.statics.array_push;
            reverse: scope.statics.array_reverse;
            shift: scope.statics.array_shift;
            unshift: scope.statics.array_unshift;
            slice: scope.statics.array_slice;
            lastIndexOf: scope.statics.array_last_index_of;

            #[symbols]
            scope.statics.symbol_iterator => scope.statics.array_values;
        });

        register_builtin_type!(scope.statics.array_iterator_prototype, {
            #[prototype] object_proto; // TODO: this is incorrect
            #[constructor] function_ctor; // TODO: ^

            #[properties]
            next: scope.statics.array_iterator_next;
        });

        register_builtin_type!(scope.statics.generator_iterator_prototype, {
            #[prototype] object_proto; // TODO: this is incorrect
            #[constructor] function_ctor; // TODO: ^

            #[properties]
            next: scope.statics.generator_iterator_next;
        });

        let symbol_ctor = register_builtin_type!(scope.statics.symbol_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.symbol_prototype;
            #[fn_name] Symbol;

            #[properties]
            asyncIterator: scope.statics.symbol_async_iterator;
            hasInstance: scope.statics.symbol_has_instance;
            iterator: scope.statics.symbol_iterator;
            match: scope.statics.symbol_match;
            matchAll: scope.statics.symbol_match_all;
            replace: scope.statics.symbol_replace;
            search: scope.statics.symbol_search;
            species: scope.statics.symbol_species;
            split: scope.statics.symbol_split;
            toPrimitive: scope.statics.symbol_to_primitive;
            toStringTag: scope.statics.symbol_to_string_tag;
            unscopables: scope.statics.symbol_unscopables;
        });

        let error_ctor = register_builtin_type!(scope.statics.error_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.error_prototype;
            #[fn_name] Error;
        });

        let error_proto = register_builtin_type!(scope.statics.error_prototype, {
            #[prototype] object_proto;
            #[constructor] error_ctor;
            #[properties]
            toString: scope.statics.error_to_string;
        });

        let arraybuffer_ctor = register_builtin_type!(scope.statics.arraybuffer_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.arraybuffer_prototype;
            #[fn_name] ArrayBuffer;
        });

        register_builtin_type!(scope.statics.arraybuffer_prototype, {
            #[prototype] object_proto;
            #[constructor] arraybuffer_ctor;
        });

        let u8array_ctor = register_builtin_type!(scope.statics.uint8array_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.uint8array_prototype;
            #[fn_name] Uint8Array;
        });

        register_builtin_type!(scope.statics.uint8array_prototype, {
            #[prototype] object_proto;
            #[constructor] u8array_ctor;
        });

        let i8array_ctor = register_builtin_type!(scope.statics.int8array_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.int8array_prototype;
            #[fn_name] Int8Array;
        });

        register_builtin_type!(scope.statics.int8array_prototype, {
            #[prototype] object_proto;
            #[constructor] i8array_ctor;
        });

        let u16array_ctor = register_builtin_type!(scope.statics.uint16array_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.uint16array_prototype;
            #[fn_name] Uint16Array;
        });

        register_builtin_type!(scope.statics.uint16array_prototype, {
            #[prototype] object_proto;
            #[constructor] u16array_ctor;
        });

        let i16array_ctor = register_builtin_type!(scope.statics.int16array_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.int16array_prototype;
            #[fn_name] Int16Array;
        });

        register_builtin_type!(scope.statics.int16array_prototype, {
            #[prototype] object_proto;
            #[constructor] i16array_ctor;
        });

        let u32array_ctor = register_builtin_type!(scope.statics.uint32array_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.uint32array_prototype;
            #[fn_name] Uint32Array;
        });

        register_builtin_type!(scope.statics.uint32array_prototype, {
            #[prototype] object_proto;
            #[constructor] u32array_ctor;
        });

        let i32array_ctor = register_builtin_type!(scope.statics.int32array_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.int32array_prototype;
            #[fn_name] Int32Array;
        });

        register_builtin_type!(scope.statics.int32array_prototype, {
            #[prototype] object_proto;
            #[constructor] i32array_ctor;
        });

        let f32array_ctor = register_builtin_type!(scope.statics.float32array_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.float32array_prototype;
            #[fn_name] Float32Array;
        });

        register_builtin_type!(scope.statics.float32array_prototype, {
            #[prototype] object_proto;
            #[constructor] f32array_ctor;
        });

        let f64array_ctor = register_builtin_type!(scope.statics.float64array_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.float64array_prototype;
            #[fn_name] Float64Array;
        });

        register_builtin_type!(scope.statics.float64array_prototype, {
            #[prototype] object_proto;
            #[constructor] f64array_ctor;
        });

        let promise_ctor = register_builtin_type!(scope.statics.promise_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.promise_proto;
            #[fn_name] Promise;
            #[properties]
            resolve: scope.statics.promise_resolve;
            reject: scope.statics.promise_reject;
        });

        register_builtin_type!(scope.statics.promise_proto, {
            #[prototype] object_proto;
            #[constructor] promise_ctor;
            #[properties]
            then: scope.statics.promise_then;
        });

        let set_ctor = register_builtin_type!(scope.statics.set_constructor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.set_prototype;
            #[fn_name] Set;
        });

        register_builtin_type!(scope.statics.set_prototype, {
            #[prototype] object_proto;
            #[constructor] set_ctor;
            #[properties]
            add: scope.statics.set_add;
            has: scope.statics.set_has;
            delete: scope.statics.set_delete;
            clear: scope.statics.set_clear;
            // size: scope.statics.set_size; // TODO: getter, not a function
        });

        let regexp_ctor = register_builtin_type!(scope.statics.regexp_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.regexp_prototype;
            #[fn_name] RegExp;
        });

        register_builtin_type!(scope.statics.regexp_prototype, {
            #[prototype] object_proto;
            #[constructor] regexp_ctor;
            #[properties]
            test: scope.statics.regexp_test;
        });

        let eval_error_ctor = register_builtin_type!(scope.statics.eval_error_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.eval_error_prototype;
            #[fn_name] EvalError;
        });

        register_builtin_type!(scope.statics.eval_error_prototype, {
            #[prototype] error_proto;
            #[constructor] eval_error_ctor;
        });

        let range_error_ctor = register_builtin_type!(scope.statics.range_error_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.range_error_prototype;
            #[fn_name] RangeError;
        });

        register_builtin_type!(scope.statics.range_error_prototype, {
            #[prototype] error_proto;
            #[constructor] range_error_ctor;
        });

        let reference_error_ctor = register_builtin_type!(scope.statics.reference_error_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.reference_error_prototype;
            #[fn_name] ReferenceError;
        });

        register_builtin_type!(scope.statics.reference_error_prototype, {
            #[prototype] error_proto;
            #[constructor] reference_error_ctor;
        });

        let syntax_error_ctor = register_builtin_type!(scope.statics.syntax_error_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.syntax_error_prototype;
            #[fn_name] SyntaxError;
        });

        register_builtin_type!(scope.statics.syntax_error_prototype, {
            #[prototype] error_proto;
            #[constructor] syntax_error_ctor;
        });

        let type_error_ctor = register_builtin_type!(scope.statics.type_error_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.type_error_prototype;
            #[fn_name] TypeError;
        });

        register_builtin_type!(scope.statics.type_error_prototype, {
            #[prototype] error_proto;
            #[constructor] type_error_ctor;
        });

        let uri_error_ctor = register_builtin_type!(scope.statics.uri_error_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.uri_error_prototype;
            #[fn_name] URIError;
        });

        register_builtin_type!(scope.statics.uri_error_prototype, {
            #[prototype] error_proto;
            #[constructor] uri_error_ctor;
        });

        let aggregate_error_ctor = register_builtin_type!(scope.statics.aggregate_error_ctor, {
            #[prototype] function_proto;
            #[constructor] function_ctor;
            #[fn_prototype] scope.statics.aggregate_error_prototype;
            #[fn_name] AggregateError;
        });

        register_builtin_type!(scope.statics.aggregate_error_prototype, {
            #[prototype] error_proto;
            #[constructor] aggregate_error_ctor;
        });

        register_builtin_type!(global, {
            #[prototype] object_proto;
            #[constructor] object_ctor;

            #[properties]
            isNaN: scope.statics.is_nan;
            isFinite: scope.statics.is_finite;
            parseFloat: scope.statics.parse_float;
            parseInt: scope.statics.parse_int;
            RegExp: regexp_ctor;
            Symbol: symbol_ctor;
            ArrayBuffer: arraybuffer_ctor;
            Uint8Array: u8array_ctor;
            Int8Array: i8array_ctor;
            Uint16Array: u16array_ctor;
            Int16Array: i16array_ctor;
            Uint32Array: u32array_ctor;
            Int32Array: i32array_ctor;
            Array: array_ctor;
            Error: error_ctor;
            EvalError: eval_error_ctor;
            RangeError: range_error_ctor;
            ReferenceError: reference_error_ctor;
            SyntaxError: syntax_error_ctor;
            TypeError: type_error_ctor;
            URIError: uri_error_ctor;
            AggregateError: aggregate_error_ctor;
            String: string_ctor;
            Object: object_ctor;
            Set: set_ctor;
            console: console;
            Math: math;
            Number: number_ctor;
            Boolean: boolean_ctor;
            Promise: promise_ctor;
        });

        scope.builtins_pure = true;
    }

    /// Fetches the current instruction/value in the currently executing frame
    /// and increments the instruction pointer
    pub(crate) fn fetch_and_inc_ip(&mut self) -> u8 {
        let frame = self.frames.last_mut().expect("No frame");
        let ip = frame.ip;
        frame.ip += 1;
        frame.function.buffer[ip]
    }

    /// Fetches a wide value (16-bit) in the currently executing frame
    /// and increments the instruction pointer
    pub(crate) fn fetchw_and_inc_ip(&mut self) -> u16 {
        let frame = self.frames.last_mut().expect("No frame");
        let value: [u8; 2] = frame.function.buffer[frame.ip..frame.ip + 2]
            .try_into()
            .expect("Failed to get wide instruction");

        frame.ip += 2;
        u16::from_ne_bytes(value)
    }

    pub(crate) fn get_frame_sp(&self) -> usize {
        self.frames.last().map(|frame| frame.sp).expect("No frame")
    }

    pub(crate) fn get_local(&self, id: usize) -> Option<Value> {
        self.stack.get(self.get_frame_sp() + id).cloned()
    }

    pub(crate) fn get_external(&self, id: usize) -> Option<&Handle<dyn Object>> {
        self.frames.last()?.externals.get(id)
    }

    pub(crate) fn set_local(&mut self, id: usize, value: Value) {
        let sp = self.get_frame_sp();
        let idx = sp + id;

        if let Value::External(o) = &mut self.stack[idx] {
            o.replace(value.into_boxed());
        } else {
            self.stack[idx] = value;
        }
    }

    pub(crate) fn try_push_frame(&mut self, frame: Frame) -> Result<(), Value> {
        if self.frames.len() > MAX_FRAME_STACK_SIZE {
            throw!(self, "Maximum call stack size exceeded");
        }

        self.frames.push(frame);
        Ok(())
    }

    pub(crate) fn try_push_stack(&mut self, value: Value) -> Result<(), Value> {
        if unlikely(self.stack.len() > MAX_STACK_SIZE) {
            throw!(self, "Maximum stack size exceeded");
        }

        self.stack.push(value);
        Ok(())
    }

    pub(crate) fn try_extend_stack<I>(&mut self, other: I) -> Result<(), Value>
    where
        I: IntoIterator<Item = Value>,
        <I as IntoIterator>::IntoIter: ExactSizeIterator,
    {
        let it = other.into_iter();
        let len = it.len();
        if self.stack.len() + len > MAX_STACK_SIZE {
            throw!(self, "Maximum stack size exceeded");
        }
        self.stack.extend(it);
        Ok(())
    }

    pub(crate) fn stack_size(&self) -> usize {
        self.stack.len()
    }

    pub(crate) fn pop_frame(&mut self) -> Option<Frame> {
        self.frames.pop()
    }

    pub(crate) fn drain_stack<R>(&mut self, range: R) -> Drain<'_, Value>
    where
        R: RangeBounds<usize>,
    {
        self.stack.drain(range)
    }

    fn handle_rt_error(&mut self, err: Value, max_fp: usize) -> Result<(), Value> {
        // Using .last() here instead of .pop() because there is a possibility that we
        // can't use this block (read the comment above the if statement try_fp < max_fp)
        if let Some(last) = self.try_blocks.last() {
            // if we're in a try-catch block, we need to jump to it
            let try_fp = last.frame_ip;
            let catch_ip = last.catch_ip;

            // Do not unwind further than we are allowed to. If the last try block is "outside" of
            // the frame that this execution context was instantiated in, then we can't jump there.
            if try_fp < max_fp {
                return Err(err);
            }

            // If we've found a suitable try block, actually remove it.
            self.try_blocks.pop();

            // Unwind frames
            drop(self.frames.drain(try_fp..));

            let frame = self.frames.last_mut().expect("No frame");
            frame.ip = catch_ip;

            let catch_ip = self.fetchw_and_inc_ip();
            if catch_ip != u16::MAX {
                // u16::MAX is used to indicate that there is no variable binding in the catch block
                self.set_local(catch_ip as usize, err);
            }

            Ok(())
        } else {
            self.frames.pop();
            Err(err)
        }
    }

    /// Adds a function to the async task queue.
    pub fn add_async_task(&mut self, fun: Handle<dyn Object>) {
        self.async_tasks.push(fun);
    }

    pub fn has_async_tasks(&self)  -> bool {
        !self.async_tasks.is_empty()
    }

    /// Processes all queued async tasks
    pub fn process_async_tasks(&mut self) {
        while !self.async_tasks.is_empty() {
            let tasks = mem::take(&mut self.async_tasks);

            let mut scope = LocalScope::new(self);

            for task in tasks {
                if let Err(ex) = task.apply(&mut scope, Value::undefined(), Vec::new()) {
                    if let Some(callback) = scope.params.unhandled_task_exception_callback() {
                        callback(&mut scope, ex);
                    }
                }
            }
        }
    }

    /// Executes a frame in this VM and initializes local variables (excluding parameters)
    /// 
    /// Parameters must be pushed onto the stack in the correct order by the caller before this function is called.
    pub fn execute_frame(&mut self, frame: Frame) -> Result<HandleResult, Value> {
        self.pad_stack_for_frame(&frame);
        self.execute_frame_raw(frame)
    }

    /// Does the necessary stack management that needs to be done before executing a JavaScript frame
    pub(crate) fn pad_stack_for_frame(&mut self, frame: &Frame) {
        // TODO: check that the stack space won't exceed our stack frame limit
        self.stack
            .resize(self.stack.len() + frame.extra_stack_space, Value::undefined());
    }

    /// Executes a frame in this VM, without doing any sort of stack management
    pub fn execute_frame_raw(&mut self, frame: Frame) -> Result<HandleResult, Value>
    {
        // TODO: if this fails, we MUST revert the stack management,
        // like reserving space for undefined values
        self.try_push_frame(frame)?;
        self.handle_instruction_loop()
    }

    fn handle_instruction_loop(&mut self) -> Result<HandleResult, Value> {
        let fp = self.frames.len();

        loop {
            if unlikely(self.gc.heap_size() > self.gc_object_threshold) {
                self.perform_gc();
            }

            let instruction = Instruction::from_repr(self.fetch_and_inc_ip()).unwrap();

            match dispatch::handle(self, instruction) {
                Ok(Some(hr)) => return Ok(hr),
                Ok(None) => continue,
                Err(e) => self.handle_rt_error(e, fp)?, // TODO: pop frame
            }
        }
    }

    pub fn execute_module(&mut self, mut frame: Frame) -> Result<Exports, Value> {
        frame.state = FrameState::Module(Exports::default());
        frame.sp = self.stack.len();
        self.execute_frame(frame)?;

        let frame = self.frames.pop().expect("Missing module frame");
        Ok(match frame.state {
            FrameState::Module(exports) => exports,
            _ => unreachable!(),
        })
    }

    pub fn perform_gc(&mut self) {
        self.trace_roots();

        // All reachable roots are marked.
        unsafe { self.gc.sweep() };

        // Adjust GC threshold
        let new_object_count = self.gc.heap_size();
        self.gc_object_threshold = new_object_count * 2;
    }

    fn trace_roots(&mut self) {
        self.frames.trace();
        self.async_tasks.trace();
        self.stack.trace();
        self.global.trace();
        self.externals.trace();
        self.statics.trace();
    }

    pub fn statics(&self) -> &Statics {
        &self.statics
    }

    pub fn gc_mut(&mut self) -> &mut Gc<dyn Object> {
        &mut self.gc
    }

    pub fn register<O: Object + 'static>(&mut self, obj: O) -> Handle<dyn Object> {
        self.gc.register(obj)
    }

    pub fn params(&self) -> &VmParams {
        &self.params
    }

    pub fn params_mut(&mut self) -> &mut VmParams {
        &mut self.params
    }

    pub fn drive_promise(&mut self, action: PromiseAction, promise: &Promise, args: Vec<Value>) {
        let arg = args.first().unwrap_or_undefined();
        let mut state = promise.state().borrow_mut();

        if let PromiseState::Pending { resolve, reject } = &mut *state {
            let handlers = match action {
                PromiseAction::Resolve => mem::take(resolve),
                PromiseAction::Reject => mem::take(reject)
            };

            for handler in handlers {
                let bf = BoundFunction::new(self, handler, None, Some(args.clone()));
                let bf = self.register(bf);
                self.add_async_task(bf);
            }
        }

        *state = match action {
            PromiseAction::Resolve => PromiseState::Resolved(arg),
            PromiseAction::Reject => PromiseState::Rejected(arg),
        };
    }

    pub(crate) fn builtins_purity(&self) -> bool {
        self.builtins_pure
    }

    pub(crate) fn impure_builtins(&mut self) {
        self.builtins_pure = false;
    }

    // -- JIT specific methods --

    /// Marks an instruction pointer (i.e. code region) as JIT-"poisoned".
    /// It will replace the instruction with one that does not attempt to trigger a trace.
    #[cfg(feature = "jit")]
    pub(crate) fn poison_ip(&mut self, ip: usize) {
        self.frames.last().unwrap().function.poison_ip(ip);
    }
}

pub enum PromiseAction {
    Resolve,
    Reject
}

impl fmt::Debug for Vm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Vm")
    }
}
