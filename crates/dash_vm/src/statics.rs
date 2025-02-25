use dash_proc_macro::Trace;

use crate::gc::handle::Handle;
use crate::gc::Gc;
use crate::js_std;
use crate::value::error::AggregateError;
use crate::value::error::EvalError;
use crate::value::error::RangeError;
use crate::value::error::ReferenceError;
use crate::value::error::SyntaxError;
use crate::value::error::TypeError;
use crate::value::error::URIError;
use crate::value::function::Function;
use crate::value::function::FunctionKind;
use crate::value::map::Map;
use crate::value::regex::RegExp;
use crate::value::set::Set;
use crate::value::PureBuiltin;

use super::value::array::Array;
use super::value::array::ArrayIterator;
use super::value::arraybuffer::ArrayBuffer;
use super::value::boxed::Boolean as BoxedBoolean;
use super::value::boxed::Number as BoxedNumber;
use super::value::boxed::String as BoxedString;
use super::value::boxed::Symbol as BoxedSymbol;
use super::value::error::Error;
use super::value::function::generator::GeneratorIterator;
use super::value::function::native::NativeFunction;
use super::value::object::NamedObject;
use super::value::object::Object;
use super::value::primitive::Symbol;

use std::rc::Rc;

#[derive(Trace)]
pub struct Statics {
    pub empty_str: Rc<str>,
    pub undefined_str: Rc<str>,
    pub null_str: Rc<str>,
    // Boolean strings
    pub true_lit: Rc<str>,
    pub false_lit: Rc<str>,
    // PreferredType strings
    pub number_str: Rc<str>,
    pub string_str: Rc<str>,
    pub default_str: Rc<str>,
    pub function_proto: Handle<dyn Object>,
    pub function_ctor: Handle<dyn Object>,
    pub function_bind: Handle<dyn Object>,
    pub function_call: Handle<dyn Object>,
    pub function_to_string: Handle<dyn Object>,
    pub is_nan: Handle<dyn Object>,
    pub is_finite: Handle<dyn Object>,
    pub parse_float: Handle<dyn Object>,
    pub parse_int: Handle<dyn Object>,
    pub console: Handle<dyn Object>,
    pub console_log: Handle<dyn Object>,
    pub math: Handle<dyn Object>,
    pub math_floor: Handle<dyn Object>,
    pub math_abs: Handle<dyn Object>,
    pub math_acos: Handle<dyn Object>,
    pub math_acosh: Handle<dyn Object>,
    pub math_asin: Handle<dyn Object>,
    pub math_asinh: Handle<dyn Object>,
    pub math_atan: Handle<dyn Object>,
    pub math_atanh: Handle<dyn Object>,
    pub math_atan2: Handle<dyn Object>,
    pub math_cbrt: Handle<dyn Object>,
    pub math_ceil: Handle<dyn Object>,
    pub math_clz32: Handle<dyn Object>,
    pub math_cos: Handle<dyn Object>,
    pub math_cosh: Handle<dyn Object>,
    pub math_exp: Handle<dyn Object>,
    pub math_expm1: Handle<dyn Object>,
    pub math_log: Handle<dyn Object>,
    pub math_log1p: Handle<dyn Object>,
    pub math_log10: Handle<dyn Object>,
    pub math_log2: Handle<dyn Object>,
    pub math_round: Handle<dyn Object>,
    pub math_sin: Handle<dyn Object>,
    pub math_sinh: Handle<dyn Object>,
    pub math_sqrt: Handle<dyn Object>,
    pub math_tan: Handle<dyn Object>,
    pub math_tanh: Handle<dyn Object>,
    pub math_trunc: Handle<dyn Object>,
    pub math_random: Handle<dyn Object>,
    pub math_max: Handle<dyn Object>,
    pub math_min: Handle<dyn Object>,
    pub object_ctor: Handle<dyn Object>,
    pub object_prototype: Handle<dyn Object>,
    pub object_create: Handle<dyn Object>,
    pub object_keys: Handle<dyn Object>,
    pub object_to_string: Handle<dyn Object>,
    pub object_get_own_property_descriptor: Handle<dyn Object>,
    pub object_get_own_property_descriptors: Handle<dyn Object>,
    pub object_has_own_property: Handle<dyn Object>,
    pub number_ctor: Handle<dyn Object>,
    pub number_prototype: Handle<dyn Object>,
    pub number_tostring: Handle<dyn Object>,
    pub number_is_finite: Handle<dyn Object>,
    pub number_is_nan: Handle<dyn Object>,
    pub number_is_safe_integer: Handle<dyn Object>,
    pub number_to_fixed: Handle<dyn Object>,
    pub boolean_ctor: Handle<dyn Object>,
    pub boolean_tostring: Handle<dyn Object>,
    pub boolean_prototype: Handle<dyn Object>,
    pub boolean_valueof: Handle<dyn Object>,
    pub string_ctor: Handle<dyn Object>,
    pub string_prototype: Handle<dyn Object>,
    pub string_tostring: Handle<dyn Object>,
    pub string_char_at: Handle<dyn Object>,
    pub string_char_code_at: Handle<dyn Object>,
    pub string_concat: Handle<dyn Object>,
    pub string_ends_with: Handle<dyn Object>,
    pub string_starts_with: Handle<dyn Object>,
    pub string_includes: Handle<dyn Object>,
    pub string_index_of: Handle<dyn Object>,
    pub string_last_index_of: Handle<dyn Object>,
    pub string_pad_end: Handle<dyn Object>,
    pub string_pad_start: Handle<dyn Object>,
    pub string_repeat: Handle<dyn Object>,
    pub string_replace: Handle<dyn Object>,
    pub string_replace_all: Handle<dyn Object>,
    pub string_split: Handle<dyn Object>,
    pub string_to_uppercase: Handle<dyn Object>,
    pub string_to_lowercase: Handle<dyn Object>,
    pub string_big: Handle<dyn Object>,
    pub string_blink: Handle<dyn Object>,
    pub string_bold: Handle<dyn Object>,
    pub string_fixed: Handle<dyn Object>,
    pub string_italics: Handle<dyn Object>,
    pub string_strike: Handle<dyn Object>,
    pub string_sub: Handle<dyn Object>,
    pub string_sup: Handle<dyn Object>,
    pub string_fontcolor: Handle<dyn Object>,
    pub string_fontsize: Handle<dyn Object>,
    pub string_link: Handle<dyn Object>,
    pub string_trim: Handle<dyn Object>,
    pub string_trim_start: Handle<dyn Object>,
    pub string_trim_end: Handle<dyn Object>,
    pub string_from_char_code: Handle<dyn Object>,
    pub string_substr: Handle<dyn Object>,
    pub string_substring: Handle<dyn Object>,
    pub string_iterator: Handle<dyn Object>,
    pub array_ctor: Handle<dyn Object>,
    pub array_tostring: Handle<dyn Object>,
    pub array_prototype: Handle<dyn Object>,
    pub array_join: Handle<dyn Object>,
    pub array_values: Handle<dyn Object>,
    pub symbol_ctor: Handle<dyn Object>,
    pub symbol_prototype: Handle<dyn Object>,
    pub symbol_async_iterator: Symbol,
    pub symbol_has_instance: Symbol,
    pub symbol_is_concat_spreadable: Symbol,
    pub symbol_iterator: Symbol,
    pub symbol_match: Symbol,
    pub symbol_match_all: Symbol,
    pub symbol_replace: Symbol,
    pub symbol_search: Symbol,
    pub symbol_species: Symbol,
    pub symbol_split: Symbol,
    pub symbol_to_primitive: Symbol,
    pub symbol_to_string_tag: Symbol,
    pub symbol_unscopables: Symbol,
    pub array_iterator_prototype: Handle<dyn Object>,
    pub array_iterator_next: Handle<dyn Object>,
    pub identity_this: Handle<dyn Object>,
    pub array_at: Handle<dyn Object>,
    pub array_concat: Handle<dyn Object>,
    pub array_entries: Handle<dyn Object>,
    pub array_keys: Handle<dyn Object>,
    pub array_every: Handle<dyn Object>,
    pub array_some: Handle<dyn Object>,
    pub array_fill: Handle<dyn Object>,
    pub array_filter: Handle<dyn Object>,
    pub array_reduce: Handle<dyn Object>,
    pub array_find: Handle<dyn Object>,
    pub array_find_index: Handle<dyn Object>,
    pub array_flat: Handle<dyn Object>,
    pub array_for_each: Handle<dyn Object>,
    pub array_includes: Handle<dyn Object>,
    pub array_index_of: Handle<dyn Object>,
    pub array_map: Handle<dyn Object>,
    pub array_pop: Handle<dyn Object>,
    pub array_push: Handle<dyn Object>,
    pub array_reverse: Handle<dyn Object>,
    pub array_shift: Handle<dyn Object>,
    pub array_unshift: Handle<dyn Object>,
    pub array_slice: Handle<dyn Object>,
    pub array_last_index_of: Handle<dyn Object>,
    pub array_from: Handle<dyn Object>,
    pub generator_iterator_prototype: Handle<dyn Object>,
    pub generator_iterator_next: Handle<dyn Object>,
    pub error_ctor: Handle<dyn Object>,
    pub error_prototype: Handle<dyn Object>,
    pub error_to_string: Handle<dyn Object>,
    pub eval_error_ctor: Handle<dyn Object>,
    pub eval_error_prototype: Handle<dyn Object>,
    pub range_error_ctor: Handle<dyn Object>,
    pub range_error_prototype: Handle<dyn Object>,
    pub reference_error_ctor: Handle<dyn Object>,
    pub reference_error_prototype: Handle<dyn Object>,
    pub syntax_error_ctor: Handle<dyn Object>,
    pub syntax_error_prototype: Handle<dyn Object>,
    pub type_error_ctor: Handle<dyn Object>,
    pub type_error_prototype: Handle<dyn Object>,
    pub uri_error_ctor: Handle<dyn Object>,
    pub uri_error_prototype: Handle<dyn Object>,
    pub aggregate_error_ctor: Handle<dyn Object>,
    pub aggregate_error_prototype: Handle<dyn Object>,
    pub arraybuffer_ctor: Handle<dyn Object>,
    pub arraybuffer_prototype: Handle<dyn Object>,
    pub uint8array_ctor: Handle<dyn Object>,
    pub uint8array_prototype: Handle<dyn Object>,
    pub int8array_ctor: Handle<dyn Object>,
    pub int8array_prototype: Handle<dyn Object>,
    pub uint16array_ctor: Handle<dyn Object>,
    pub uint16array_prototype: Handle<dyn Object>,
    pub int16array_ctor: Handle<dyn Object>,
    pub int16array_prototype: Handle<dyn Object>,
    pub uint32array_ctor: Handle<dyn Object>,
    pub uint32array_prototype: Handle<dyn Object>,
    pub int32array_ctor: Handle<dyn Object>,
    pub int32array_prototype: Handle<dyn Object>,
    pub float32array_ctor: Handle<dyn Object>,
    pub float32array_prototype: Handle<dyn Object>,
    pub float64array_ctor: Handle<dyn Object>,
    pub float64array_prototype: Handle<dyn Object>,
    pub typedarray_fill: Handle<dyn Object>,
    pub promise_ctor: Handle<dyn Object>,
    pub promise_proto: Handle<dyn Object>,
    pub promise_resolve: Handle<dyn Object>,
    pub promise_reject: Handle<dyn Object>,
    pub promise_then: Handle<dyn Object>,
    pub set_constructor: Handle<dyn Object>,
    pub set_prototype: Handle<dyn Object>,
    pub set_add: Handle<dyn Object>,
    pub set_has: Handle<dyn Object>,
    pub set_delete: Handle<dyn Object>,
    pub set_clear: Handle<dyn Object>,
    pub set_size: Handle<dyn Object>,
    pub map_constructor: Handle<dyn Object>,
    pub map_prototype: Handle<dyn Object>,
    pub map_set: Handle<dyn Object>,
    pub map_get: Handle<dyn Object>,
    pub map_has: Handle<dyn Object>,
    pub map_delete: Handle<dyn Object>,
    pub map_clear: Handle<dyn Object>,
    pub map_size: Handle<dyn Object>,
    pub regexp_ctor: Handle<dyn Object>,
    pub regexp_prototype: Handle<dyn Object>,
    pub regexp_test: Handle<dyn Object>,
    pub date_ctor: Handle<dyn Object>,
    pub date_prototype: Handle<dyn Object>,
    pub date_now: Handle<dyn Object>,
}

fn builtin_object<O: Object + 'static>(gc: &mut Gc, obj: O) -> Handle<dyn Object> {
    gc.register(PureBuiltin::new(obj))
}

fn empty_object(gc: &mut Gc) -> Handle<dyn Object> {
    builtin_object(gc, NamedObject::null())
}

fn function(gc: &mut Gc, name: &str, cb: NativeFunction) -> Handle<dyn Object> {
    let f = Function::with_obj(Some(name.into()), FunctionKind::Native(cb), NamedObject::null());
    gc.register(PureBuiltin::new(f))
}

impl Statics {
    pub fn new(gc: &mut Gc) -> Self {
        let empty_str: Rc<str> = "".into();

        Self {
            true_lit: "true".into(),
            false_lit: "false".into(),
            empty_str: empty_str.clone(),
            null_str: "null".into(),
            undefined_str: "undefined".into(),
            default_str: "default".into(),
            number_str: "number".into(),
            string_str: "string".into(),
            function_proto: empty_object(gc),
            function_ctor: function(gc, "Function", js_std::function::constructor),
            function_bind: function(gc, "bind", js_std::function::bind),
            function_call: function(gc, "call", js_std::function::call),
            function_to_string: function(gc, "toString", js_std::function::to_string),
            console: empty_object(gc),
            console_log: function(gc, "log", js_std::global::log),
            math: empty_object(gc),
            math_floor: function(gc, "floor", js_std::math::floor),
            object_ctor: function(gc, "Object", js_std::object::constructor),
            object_create: function(gc, "create", js_std::object::create),
            object_keys: function(gc, "keys", js_std::object::keys),
            object_prototype: empty_object(gc),
            object_to_string: function(gc, "toString", js_std::object::to_string),
            object_get_own_property_descriptor: function(
                gc,
                "getOwnPropertyDescriptor",
                js_std::object::get_own_property_descriptor,
            ),
            object_get_own_property_descriptors: function(
                gc,
                "getOwnPropertyDescriptors",
                js_std::object::get_own_property_descriptors,
            ),
            object_has_own_property: function(gc, "hasOwnProperty", js_std::object::has_own_property),
            number_ctor: function(gc, "Number", js_std::number::constructor),
            number_prototype: builtin_object(gc, BoxedNumber::with_obj(0.0, NamedObject::null())),
            number_tostring: function(gc, "toString", js_std::number::to_string),
            boolean_ctor: function(gc, "Boolean", js_std::boolean::constructor),
            boolean_tostring: function(gc, "toString", js_std::boolean::to_string),
            boolean_prototype: builtin_object(gc, BoxedBoolean::with_obj(false, NamedObject::null())),
            string_ctor: function(gc, "Boolean", js_std::string::constructor),
            string_prototype: builtin_object(gc, BoxedString::with_obj(empty_str.clone(), NamedObject::null())),
            is_nan: function(gc, "isNaN", js_std::global::is_nan),
            is_finite: function(gc, "isFinite", js_std::global::is_finite),
            parse_float: function(gc, "parseFloat", js_std::global::parse_float),
            parse_int: function(gc, "parseInt", js_std::global::parse_int),
            math_abs: function(gc, "abs", js_std::math::abs),
            math_acos: function(gc, "acos", js_std::math::acos),
            math_acosh: function(gc, "acosh", js_std::math::acosh),
            math_asin: function(gc, "asin", js_std::math::asin),
            math_asinh: function(gc, "asinh", js_std::math::asinh),
            math_atan: function(gc, "atan", js_std::math::atan),
            math_atanh: function(gc, "atanh", js_std::math::atanh),
            math_atan2: function(gc, "atan2", js_std::math::atan2),
            math_cbrt: function(gc, "cbrt", js_std::math::cbrt),
            math_ceil: function(gc, "ceil", js_std::math::ceil),
            math_clz32: function(gc, "clz32", js_std::math::clz32),
            math_cos: function(gc, "cos", js_std::math::cos),
            math_cosh: function(gc, "cosh", js_std::math::cosh),
            math_exp: function(gc, "exp", js_std::math::exp),
            math_expm1: function(gc, "expm1", js_std::math::expm1),
            math_log: function(gc, "log", js_std::math::log),
            math_log1p: function(gc, "log1p", js_std::math::log1p),
            math_log10: function(gc, "log10", js_std::math::log10),
            math_log2: function(gc, "log2", js_std::math::log2),
            math_round: function(gc, "round", js_std::math::round),
            math_sin: function(gc, "sin", js_std::math::sin),
            math_sinh: function(gc, "sinh", js_std::math::sinh),
            math_sqrt: function(gc, "sqrt", js_std::math::sqrt),
            math_tan: function(gc, "tan", js_std::math::tan),
            math_tanh: function(gc, "tanh", js_std::math::tanh),
            math_trunc: function(gc, "trunc", js_std::math::trunc),
            math_random: function(gc, "random", js_std::math::random),
            math_max: function(gc, "max", js_std::math::max),
            math_min: function(gc, "min", js_std::math::min),
            number_is_finite: function(gc, "isFinite", js_std::number::is_finite),
            number_is_nan: function(gc, "isNaN", js_std::number::is_nan),
            number_is_safe_integer: function(gc, "isSafeInteger", js_std::number::is_safe_integer),
            number_to_fixed: function(gc, "toFixed", js_std::number::to_fixed),
            boolean_valueof: function(gc, "valueOf", js_std::boolean::value_of),
            string_tostring: function(gc, "toString", js_std::string::to_string),
            string_char_at: function(gc, "charAt", js_std::string::char_at),
            string_char_code_at: function(gc, "charCodeAt", js_std::string::char_code_at),
            string_concat: function(gc, "concat", js_std::string::concat),
            string_ends_with: function(gc, "endsWith", js_std::string::ends_with),
            string_starts_with: function(gc, "startsWith", js_std::string::starts_with),
            string_includes: function(gc, "includes", js_std::string::includes),
            string_index_of: function(gc, "indexOf", js_std::string::index_of),
            string_last_index_of: function(gc, "lastIndexOf", js_std::string::last_index_of),
            string_pad_end: function(gc, "padEnd", js_std::string::pad_end),
            string_pad_start: function(gc, "padStart", js_std::string::pad_start),
            string_repeat: function(gc, "repeat", js_std::string::repeat),
            string_replace: function(gc, "replace", js_std::string::replace),
            string_replace_all: function(gc, "replaceAll", js_std::string::replace_all),
            string_split: function(gc, "split", js_std::string::split),
            string_to_uppercase: function(gc, "toUpperCase", js_std::string::to_uppercase),
            string_to_lowercase: function(gc, "toLowerCase", js_std::string::to_lowercase),
            string_big: function(gc, "big", js_std::string::big),
            string_blink: function(gc, "blink", js_std::string::blink),
            string_bold: function(gc, "bold", js_std::string::bold),
            string_fixed: function(gc, "fixed", js_std::string::fixed),
            string_italics: function(gc, "italics", js_std::string::italics),
            string_strike: function(gc, "strike", js_std::string::strike),
            string_sub: function(gc, "sub", js_std::string::sub),
            string_sup: function(gc, "sup", js_std::string::sup),
            string_fontcolor: function(gc, "fontcolor", js_std::string::fontcolor),
            string_fontsize: function(gc, "fontsize", js_std::string::fontsize),
            string_link: function(gc, "link", js_std::string::link),
            string_trim: function(gc, "trim", js_std::string::trim),
            string_trim_start: function(gc, "trimStart", js_std::string::trim_start),
            string_trim_end: function(gc, "trimEnd", js_std::string::trim_end),
            string_from_char_code: function(gc, "fromCharCode", js_std::string::from_char_code),
            string_substr: function(gc, "substr", js_std::string::substr),
            string_substring: function(gc, "substring", js_std::string::substring),
            string_iterator: function(gc, "iterator", js_std::string::iterator),
            array_ctor: function(gc, "Array", js_std::array::constructor),
            array_tostring: function(gc, "toString", js_std::array::to_string),
            array_prototype: builtin_object(gc, Array::with_obj(NamedObject::null())),
            array_join: function(gc, "join", js_std::array::join),
            array_values: function(gc, "values", js_std::array::values),
            array_reverse: function(gc, "reverse", js_std::array::reverse),
            symbol_ctor: function(gc, "Symbol", js_std::symbol::constructor),
            symbol_prototype: builtin_object(gc, BoxedSymbol::with_obj(Symbol::new(empty_str), NamedObject::null())),
            symbol_async_iterator: Symbol::new("Symbol.asyncIterator".into()),
            symbol_has_instance: Symbol::new("Symbol.hasInstance".into()),
            symbol_is_concat_spreadable: Symbol::new("Symbol.isConcatSpreadable".into()),
            symbol_iterator: Symbol::new("Symbol.iterator".into()),
            symbol_match: Symbol::new("Symbol.match".into()),
            symbol_match_all: Symbol::new("Symbol.matchAll".into()),
            symbol_replace: Symbol::new("Symbol.replace".into()),
            symbol_search: Symbol::new("Symbol.search".into()),
            symbol_species: Symbol::new("Symbol.species".into()),
            symbol_split: Symbol::new("Symbol.split".into()),
            symbol_to_primitive: Symbol::new("SymboltoPrimitive".into()),
            symbol_to_string_tag: Symbol::new("Symbol.toStringTag".into()),
            symbol_unscopables: Symbol::new("Symbol.unscopables".into()),
            array_iterator_prototype: builtin_object(gc, ArrayIterator::empty()),
            array_iterator_next: function(gc, "next", js_std::array_iterator::next),
            identity_this: function(gc, "iterator", js_std::identity_this),
            array_at: function(gc, "at", js_std::array::at),
            array_concat: function(gc, "concat", js_std::array::concat),
            array_entries: function(gc, "entries", js_std::array::entries),
            array_keys: function(gc, "keys", js_std::array::keys),
            array_every: function(gc, "every", js_std::array::every),
            array_some: function(gc, "some", js_std::array::some),
            array_fill: function(gc, "fill", js_std::array::fill),
            array_filter: function(gc, "filter", js_std::array::filter),
            array_reduce: function(gc, "reduce", js_std::array::reduce),
            array_find: function(gc, "find", js_std::array::find),
            array_find_index: function(gc, "findIndex", js_std::array::find_index),
            array_flat: function(gc, "flat", js_std::array::flat),
            array_for_each: function(gc, "forEach", js_std::array::for_each),
            array_includes: function(gc, "includes", js_std::array::includes),
            array_index_of: function(gc, "indexOf", js_std::array::index_of),
            array_map: function(gc, "map", js_std::array::map),
            array_pop: function(gc, "pop", js_std::array::pop),
            array_push: function(gc, "push", js_std::array::push),
            array_shift: function(gc, "shift", js_std::array::shift),
            array_unshift: function(gc, "unshift", js_std::array::unshift),
            array_slice: function(gc, "slice", js_std::array::slice),
            array_last_index_of: function(gc, "lastIndexOf", js_std::array::last_index_of),
            array_from: function(gc, "from", js_std::array::from),
            generator_iterator_prototype: {
                let obj = gc.register(NamedObject::null());
                builtin_object(gc, GeneratorIterator::empty(obj))
            },
            generator_iterator_next: function(gc, "next", js_std::generator::next),
            error_ctor: function(gc, "Error", js_std::error::error_constructor),
            error_prototype: builtin_object(gc, Error::empty()),
            error_to_string: function(gc, "toString", js_std::error::to_string),
            eval_error_ctor: function(gc, "EvalError", js_std::error::eval_error_constructor),
            eval_error_prototype: builtin_object(gc, EvalError::empty()),
            range_error_ctor: function(gc, "RangeError", js_std::error::range_error_constructor),
            range_error_prototype: builtin_object(gc, RangeError::empty()),
            reference_error_ctor: function(gc, "ReferenceError", js_std::error::reference_error_constructor),
            reference_error_prototype: builtin_object(gc, ReferenceError::empty()),
            syntax_error_ctor: function(gc, "SyntaxError", js_std::error::syntax_error_constructor),
            syntax_error_prototype: builtin_object(gc, SyntaxError::empty()),
            type_error_ctor: function(gc, "TypeError", js_std::error::type_error_constructor),
            type_error_prototype: builtin_object(gc, TypeError::empty()),
            uri_error_ctor: function(gc, "URIError", js_std::error::uri_error_constructor),
            uri_error_prototype: builtin_object(gc, URIError::empty()),
            aggregate_error_ctor: function(gc, "AggregateError", js_std::error::aggregate_error_constructor),
            aggregate_error_prototype: builtin_object(gc, AggregateError::empty()),
            arraybuffer_ctor: function(gc, "ArrayBuffer", js_std::arraybuffer::constructor),
            arraybuffer_prototype: builtin_object(gc, ArrayBuffer::empty()),
            uint8array_ctor: function(gc, "Uint8Array", js_std::typedarray::u8array::constructor),
            uint8array_prototype: empty_object(gc),
            int8array_ctor: function(gc, "Int8Array", js_std::typedarray::i8array::constructor),
            int8array_prototype: empty_object(gc),
            uint16array_ctor: function(gc, "Uint16Array", js_std::typedarray::u16array::constructor),
            uint16array_prototype: empty_object(gc),
            int16array_ctor: function(gc, "Int16Array", js_std::typedarray::i16array::constructor),
            int16array_prototype: empty_object(gc),
            uint32array_ctor: function(gc, "Uint32Array", js_std::typedarray::u32array::constructor),
            uint32array_prototype: empty_object(gc),
            int32array_ctor: function(gc, "Int32Array", js_std::typedarray::i32array::constructor),
            int32array_prototype: empty_object(gc),
            float32array_ctor: function(gc, "Float32Array", js_std::typedarray::f32array::constructor),
            float32array_prototype: empty_object(gc),
            float64array_ctor: function(gc, "Float64Array", js_std::typedarray::f64array::constructor),
            float64array_prototype: empty_object(gc),
            typedarray_fill: function(gc, "fill", js_std::typedarray::fill),
            promise_ctor: function(gc, "Promise", js_std::promise::constructor),
            promise_proto: empty_object(gc),
            promise_resolve: function(gc, "resolve", js_std::promise::resolve),
            promise_reject: function(gc, "reject", js_std::promise::reject),
            promise_then: function(gc, "then", js_std::promise::then),
            set_constructor: function(gc, "Set", js_std::set::constructor),
            set_add: function(gc, "add", js_std::set::add),
            set_has: function(gc, "has", js_std::set::has),
            set_delete: function(gc, "delete", js_std::set::delete),
            set_prototype: builtin_object(gc, Set::with_obj(NamedObject::null())),
            set_clear: function(gc, "clear", js_std::set::clear),
            set_size: function(gc, "size", js_std::set::size),
            map_constructor: function(gc, "Map", js_std::map::constructor),
            map_set: function(gc, "set", js_std::map::set),
            map_get: function(gc, "get", js_std::map::get),
            map_has: function(gc, "has", js_std::map::has),
            map_delete: function(gc, "delete", js_std::map::delete),
            map_prototype: builtin_object(gc, Map::with_obj(NamedObject::null())),
            map_clear: function(gc, "clear", js_std::map::clear),
            map_size: function(gc, "size", js_std::map::size),
            regexp_ctor: function(gc, "RegExp", js_std::regex::constructor),
            regexp_prototype: builtin_object(gc, RegExp::empty()),
            regexp_test: function(gc, "test", js_std::regex::test),
            date_ctor: function(gc, "Date", js_std::date::constructor),
            date_prototype: builtin_object(gc, NamedObject::null()),
            date_now: function(gc, "now", js_std::date::now),
        }
    }

    pub fn get_true(&self) -> Rc<str> {
        self.true_lit.clone()
    }

    pub fn get_false(&self) -> Rc<str> {
        self.false_lit.clone()
    }

    pub fn empty_str(&self) -> Rc<str> {
        self.empty_str.clone()
    }

    pub fn null_str(&self) -> Rc<str> {
        self.null_str.clone()
    }

    pub fn undefined_str(&self) -> Rc<str> {
        self.undefined_str.clone()
    }
}
