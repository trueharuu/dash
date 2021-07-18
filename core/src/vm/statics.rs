use crate::gc::{Gc, Handle};
use crate::js_std;
use crate::vm::value::function::Constructor;
use crate::vm::value::object::AnyObject;

use super::value::function::NativeFunction;
use super::value::Value;

/// Static data used by the VM
pub struct Statics {
    /// Represents Boolean.prototype
    pub boolean_proto: Handle<Value>,
    /// Represents Number.prototype
    pub number_proto: Handle<Value>,
    /// Represents String.prototype
    pub string_proto: Handle<Value>,
    /// Represents Function.prototype
    pub function_proto: Handle<Value>,
    /// Represents Array.prototype
    pub array_proto: Handle<Value>,
    /// Represents WeakSet.prototype
    pub weakset_proto: Handle<Value>,
    /// Represents WeakMap.prototype
    pub weakmap_proto: Handle<Value>,
    /// Represents Object.prototype
    pub object_proto: Handle<Value>,
    /// Represents Error.prototype
    pub error_proto: Handle<Value>,
    /// Represents Promise.prototype
    pub promise_proto: Handle<Value>,
    /// Represents the Boolean constructor
    pub boolean_ctor: Handle<Value>,
    /// Represents the Number constructor
    pub number_ctor: Handle<Value>,
    /// Represents the String constructor
    pub string_ctor: Handle<Value>,
    /// Represents the Function constructor
    pub function_ctor: Handle<Value>,
    /// Represents the Array constructor
    pub array_ctor: Handle<Value>,
    /// Represents the WeakSet constructor
    pub weakset_ctor: Handle<Value>,
    /// Represents the WeakMap constructor
    pub weakmap_ctor: Handle<Value>,
    /// Represents the Object constructor
    pub object_ctor: Handle<Value>,
    /// Represents the Error constructor
    pub error_ctor: Handle<Value>,
    /// Represents the Promise constructor
    pub promise_ctor: Handle<Value>,
    /// Represents console.log
    pub console_log: Handle<Value>,
    /// Represents isNaN
    pub isnan: Handle<Value>,
    /// Represents Array.prototype.push
    pub array_push: Handle<Value>,
    /// Represents Array.prototype.concat
    pub array_concat: Handle<Value>,
    /// Represents Array.prototype.map
    pub array_map: Handle<Value>,
    /// Represents Array.prototype.every
    pub array_every: Handle<Value>,
    /// Represents Array.prototype.fill
    pub array_fill: Handle<Value>,
    /// Represents Array.prototype.filter
    pub array_filter: Handle<Value>,
    /// Represents Array.prototype.find
    pub array_find: Handle<Value>,
    /// Represents Array.prototype.findIndex
    pub array_find_index: Handle<Value>,
    /// Represents Array.prototype.flat
    pub array_flat: Handle<Value>,
    /// Represents Array.prototype.forEach
    pub array_for_each: Handle<Value>,
    /// Represents Array.from
    pub array_from: Handle<Value>,
    /// Represents Array.prototype.includes
    pub array_includes: Handle<Value>,
    /// Represents Array.prototype.indexOf
    pub array_index_of: Handle<Value>,
    /// Represents Array.isArray
    pub array_is_array: Handle<Value>,
    /// Represents Array.prototype.join
    pub array_join: Handle<Value>,
    /// Represents Array.prototype.lastIndexOf
    pub array_last_index_of: Handle<Value>,
    /// Represents Array.of
    pub array_of: Handle<Value>,
    /// Represents Array.prototype.pop
    pub array_pop: Handle<Value>,
    /// Represents Array.prototype.reduce
    pub array_reduce: Handle<Value>,
    /// Represents Array.prototype.reduceRight
    pub array_reduce_right: Handle<Value>,
    /// Represents Array.prototype.reverse
    pub array_reverse: Handle<Value>,
    /// Represents Array.prototype.shift
    pub array_shift: Handle<Value>,
    /// Represents Array.prototype.slice
    pub array_slice: Handle<Value>,
    /// Represents Array.prototype.some
    pub array_some: Handle<Value>,
    /// Represents Array.prototype.sort
    pub array_sort: Handle<Value>,
    /// Represents Array.prototype.splice
    pub array_splice: Handle<Value>,
    /// Represents Array.prototype.unshift
    pub array_unshift: Handle<Value>,
    /// Represents String.prototype.charAt
    pub string_char_at: Handle<Value>,
    /// Represents String.prototype.charCodeAt
    pub string_char_code_at: Handle<Value>,
    /// Represents String.prototype.endsWith
    pub string_ends_with: Handle<Value>,
    /// Represents String.prototype.anchor
    pub string_anchor: Handle<Value>,
    /// Represents String.prototype.big
    pub string_big: Handle<Value>,
    /// Represents String.prototype.blink
    pub string_blink: Handle<Value>,
    /// Represents String.prototype.bold
    pub string_bold: Handle<Value>,
    /// Represents String.prototype.fixed
    pub string_fixed: Handle<Value>,
    /// Represents String.prototype.fontcolor
    pub string_fontcolor: Handle<Value>,
    /// Represents String.prototype.fontsize
    pub string_fontsize: Handle<Value>,
    /// Represents String.prototype.italics
    pub string_italics: Handle<Value>,
    /// Represents String.prototype.link
    pub string_link: Handle<Value>,
    /// Represents String.prototype.small
    pub string_small: Handle<Value>,
    /// Represents String.prototype.strike
    pub string_strike: Handle<Value>,
    /// Represents String.prototype.sub
    pub string_sub: Handle<Value>,
    /// Represents String.prototype.sup
    pub string_sup: Handle<Value>,
    /// Represents Math.pow
    pub math_pow: Handle<Value>,
    /// Represents Math.abs
    pub math_abs: Handle<Value>,
    /// Represents Math.ceil
    pub math_ceil: Handle<Value>,
    /// Represents Math.floor
    pub math_floor: Handle<Value>,
    /// Represents Math.max
    pub math_max: Handle<Value>,
    /// Represents Math.random
    pub math_random: Handle<Value>,
    /// Represents Object.defineProperty
    pub object_define_property: Handle<Value>,
    /// Represents Object.getOwnPropertyNames
    pub object_get_own_property_names: Handle<Value>,
    /// Represents Object.getPrototypeOf
    pub object_get_prototype_of: Handle<Value>,
    /// Represents Object.prototype.toString
    pub object_to_string: Handle<Value>,
    /// Represents WeakSet.prototype.has
    pub weakset_has: Handle<Value>,
    /// Represents WeakSet.prototype.add
    pub weakset_add: Handle<Value>,
    /// Represents WeakSet.prototype.delete
    pub weakset_delete: Handle<Value>,
    /// Represents WeakMap.prototype.has
    pub weakmap_has: Handle<Value>,
    /// Represents WeakMap.prototype.add
    pub weakmap_add: Handle<Value>,
    /// Represents WeakMap.prototype.get
    pub weakmap_get: Handle<Value>,
    /// Represents WeakMap.prototype.delete
    pub weakmap_delete: Handle<Value>,
    /// Represents JSON.parse
    pub json_parse: Handle<Value>,
    /// Represents JSON.stringify
    pub json_stringify: Handle<Value>,
    /// Represents Promise.resolve
    pub promise_resolve: Handle<Value>,
    /// Represents Promise.reject
    pub promise_reject: Handle<Value>,
}

macro_rules! register_glob_method {
    ($gc:expr, $name:expr, $path:expr, $marker:expr) => {
        $gc.register(
            Value::from(NativeFunction::new($name, $path, None, Constructor::NoCtor)),
            $marker,
        )
    };
}

macro_rules! register_ctor {
    ($gc:expr, $name:expr, $path:expr, $marker:expr) => {
        $gc.register(
            Value::from(NativeFunction::new($name, $path, None, Constructor::Ctor)),
            $marker,
        )
    };
}

impl Statics {
    /// Creates a new global data object
    pub fn new(gc: &mut Gc<Value>, marker: *const ()) -> Self {
        Self {
            // Proto
            boolean_proto: gc.register(Value::from(AnyObject {}), marker),
            number_proto: gc.register(Value::from(AnyObject {}), marker),
            string_proto: gc.register(Value::from(AnyObject {}), marker),
            function_proto: gc.register(Value::from(AnyObject {}), marker),
            array_proto: gc.register(Value::from(AnyObject {}), marker),
            weakset_proto: gc.register(Value::from(AnyObject {}), marker),
            weakmap_proto: gc.register(Value::from(AnyObject {}), marker),
            object_proto: gc.register(Value::from(AnyObject {}), marker),
            error_proto: gc.register(Value::from(AnyObject {}), marker),
            promise_proto: gc.register(Value::from(AnyObject {}), marker),
            // Ctor
            error_ctor: register_ctor!(gc, "Error", js_std::error::error_constructor, marker),
            weakset_ctor: register_ctor!(
                gc,
                "WeakSet",
                js_std::weakset::weakset_constructor,
                marker
            ),
            weakmap_ctor: register_ctor!(
                gc,
                "WeakMap",
                js_std::weakmap::weakmap_constructor,
                marker
            ),
            boolean_ctor: register_ctor!(
                gc,
                "Boolean",
                js_std::boolean::boolean_constructor,
                marker
            ),
            number_ctor: register_ctor!(gc, "Number", js_std::number::number_constructor, marker),
            string_ctor: register_ctor!(gc, "String", js_std::string::string_constructor, marker),
            function_ctor: register_ctor!(
                gc,
                "Function",
                js_std::function::function_constructor,
                marker
            ),
            array_ctor: register_ctor!(gc, "Array", js_std::array::array_constructor, marker),
            object_ctor: register_ctor!(gc, "Object", js_std::object::object_constructor, marker),
            promise_ctor: register_ctor!(
                gc,
                "Promise",
                js_std::promise::promise_constructor,
                marker
            ),
            // Methods
            console_log: register_glob_method!(gc, "log", js_std::console::log, marker),
            isnan: register_glob_method!(gc, "isNaN", js_std::functions::is_nan, marker),
            array_push: register_glob_method!(gc, "push", js_std::array::push, marker),
            array_concat: register_glob_method!(gc, "concat", js_std::array::concat, marker),
            array_map: register_glob_method!(gc, "map", js_std::array::map, marker),
            array_every: register_glob_method!(gc, "every", js_std::array::every, marker),
            array_fill: register_glob_method!(gc, "fill", js_std::array::fill, marker),
            array_filter: register_glob_method!(gc, "filter", js_std::array::filter, marker),
            array_find: register_glob_method!(gc, "find", js_std::array::find, marker),
            array_find_index: register_glob_method!(
                gc,
                "findIndex",
                js_std::array::find_index,
                marker
            ),
            array_flat: register_glob_method!(gc, "flat", js_std::array::flat, marker),
            array_for_each: register_glob_method!(gc, "forEach", js_std::array::for_each, marker),
            array_from: register_glob_method!(gc, "from", js_std::array::from, marker),
            array_includes: register_glob_method!(gc, "includes", js_std::array::includes, marker),
            array_index_of: register_glob_method!(gc, "indexOf", js_std::array::index_of, marker),
            array_is_array: register_glob_method!(gc, "isArray", js_std::array::is_array, marker),
            array_join: register_glob_method!(gc, "join", js_std::array::join, marker),
            array_last_index_of: register_glob_method!(
                gc,
                "lastIndexOf",
                js_std::array::last_index_of,
                marker
            ),
            array_of: register_glob_method!(gc, "of", js_std::array::of, marker),
            array_pop: register_glob_method!(gc, "pop", js_std::array::pop, marker),
            array_reduce: register_glob_method!(gc, "reduce", js_std::array::reduce, marker),
            array_reduce_right: register_glob_method!(
                gc,
                "reduceRight",
                js_std::array::reduce_right,
                marker
            ),
            array_reverse: register_glob_method!(gc, "reverse", js_std::array::reverse, marker),
            array_shift: register_glob_method!(gc, "shift", js_std::array::shift, marker),
            array_slice: register_glob_method!(gc, "slice", js_std::array::slice, marker),
            array_some: register_glob_method!(gc, "some", js_std::array::some, marker),
            array_sort: register_glob_method!(gc, "sort", js_std::array::sort, marker),
            array_splice: register_glob_method!(gc, "splice", js_std::array::splice, marker),
            array_unshift: register_glob_method!(gc, "unshift", js_std::array::unshift, marker),
            string_char_at: register_glob_method!(gc, "charAt", js_std::string::char_at, marker),
            string_char_code_at: register_glob_method!(
                gc,
                "charCodeAt",
                js_std::string::char_code_at,
                marker
            ),
            string_ends_with: register_glob_method!(
                gc,
                "endsWith",
                js_std::string::ends_with,
                marker
            ),
            string_anchor: register_glob_method!(gc, "anchor", js_std::string::anchor, marker),
            string_big: register_glob_method!(gc, "big", js_std::string::big, marker),
            string_blink: register_glob_method!(gc, "blink", js_std::string::blink, marker),
            string_bold: register_glob_method!(gc, "bold", js_std::string::bold, marker),
            string_fixed: register_glob_method!(gc, "fixed", js_std::string::fixed, marker),
            string_fontcolor: register_glob_method!(
                gc,
                "fontcolor",
                js_std::string::fontcolor,
                marker
            ),
            string_fontsize: register_glob_method!(
                gc,
                "fontsize",
                js_std::string::fontsize,
                marker
            ),
            string_italics: register_glob_method!(gc, "italics", js_std::string::italics, marker),
            string_link: register_glob_method!(gc, "link", js_std::string::link, marker),
            string_small: register_glob_method!(gc, "small", js_std::string::small, marker),
            string_strike: register_glob_method!(gc, "strike", js_std::string::strike, marker),
            string_sub: register_glob_method!(gc, "sub", js_std::string::sub, marker),
            string_sup: register_glob_method!(gc, "sup", js_std::string::sup, marker),
            math_pow: register_glob_method!(gc, "pow", js_std::math::pow, marker),
            math_abs: register_glob_method!(gc, "abs", js_std::math::abs, marker),
            math_ceil: register_glob_method!(gc, "ceil", js_std::math::ceil, marker),
            math_floor: register_glob_method!(gc, "floor", js_std::math::floor, marker),
            math_max: register_glob_method!(gc, "max", js_std::math::max, marker),
            math_random: register_glob_method!(gc, "random", js_std::math::random, marker),
            object_define_property: register_glob_method!(
                gc,
                "defineProperty",
                js_std::object::define_property,
                marker
            ),
            object_get_own_property_names: register_glob_method!(
                gc,
                "getOwnPropertyNames",
                js_std::object::get_own_property_names,
                marker
            ),
            object_get_prototype_of: register_glob_method!(
                gc,
                "getPrototypeOf",
                js_std::object::get_prototype_of,
                marker
            ),
            object_to_string: register_glob_method!(
                gc,
                "toString",
                js_std::object::to_string,
                marker
            ),
            weakset_has: register_glob_method!(gc, "has", js_std::weakset::has, marker),
            weakset_add: register_glob_method!(gc, "add", js_std::weakset::add, marker),
            weakset_delete: register_glob_method!(gc, "delete", js_std::weakset::delete, marker),
            weakmap_has: register_glob_method!(gc, "has", js_std::weakmap::has, marker),
            weakmap_add: register_glob_method!(gc, "add", js_std::weakmap::add, marker),
            weakmap_get: register_glob_method!(gc, "get", js_std::weakmap::get, marker),
            weakmap_delete: register_glob_method!(gc, "delete", js_std::weakmap::delete, marker),
            json_parse: register_glob_method!(gc, "parse", js_std::json::parse, marker),
            json_stringify: register_glob_method!(gc, "stringify", js_std::json::stringify, marker),
            promise_resolve: register_glob_method!(gc, "resolve", js_std::promise::resolve, marker),
            promise_reject: register_glob_method!(gc, "reject", js_std::promise::reject, marker),
        }
    }
}
