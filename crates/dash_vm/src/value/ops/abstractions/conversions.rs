use std::rc::Rc;

use crate::gc::handle::Handle;
use crate::local::LocalScope;
use crate::throw;
use crate::value::boxed::Boolean;
use crate::value::boxed::Number as BoxedNumber;
use crate::value::boxed::String as BoxedString;
use crate::value::boxed::Symbol as BoxedSymbol;
use crate::value::object::Object;
use crate::value::primitive::MAX_SAFE_INTEGERF;
use crate::value::Typeof;
use crate::value::Value;
use crate::Vm;

pub trait ValueConversion {
    fn to_primitive(&self, sc: &mut LocalScope, preferred_type: Option<PreferredType>) -> Result<Value, Value>;

    fn to_number(&self, sc: &mut LocalScope) -> Result<f64, Value>;

    fn to_length(&self, sc: &mut LocalScope) -> Result<f64, Value> {
        // Let len be ? ToIntegerOrInfinity(argument).
        let len = self.to_integer_or_infinity(sc)?;
        // 2. If len ≤ 0, return +0𝔽.
        if len <= 0.0 {
            return Ok(0.0);
        }

        // Return 𝔽(min(len, 253 - 1)).
        Ok(len.min(MAX_SAFE_INTEGERF))
    }

    fn to_length_u(&self, sc: &mut LocalScope) -> Result<usize, Value> {
        self.to_length(sc).map(|x| x as usize)
    }

    fn to_integer_or_infinity(&self, sc: &mut LocalScope) -> Result<f64, Value> {
        // Let number be ? ToNumber(argument).
        let number = self.to_number(sc)?;
        // 2. If number is NaN, +0𝔽, or -0𝔽, return 0.
        if number.is_nan() || number == 0.0 {
            return Ok(0.0);
        }

        // 3. If number is +∞𝔽, return +∞.
        // 4. If number is -∞𝔽, return -∞.
        if number == f64::INFINITY || number == f64::NEG_INFINITY {
            return Ok(number);
        }

        // 5. Let integer be floor(abs(ℝ(number))).
        let integer = number.abs().floor();

        // 6. If number < -0𝔽, set integer to -integer.
        if number < 0.0 {
            Ok(-integer)
        } else {
            Ok(integer)
        }
    }

    fn to_boolean(&self) -> Result<bool, Value>;

    fn to_string(&self, sc: &mut LocalScope) -> Result<Rc<str>, Value>;

    fn length_of_array_like(&self, sc: &mut LocalScope) -> Result<usize, Value>;

    fn to_object(&self, sc: &mut LocalScope) -> Result<Handle<dyn Object>, Value>;

    fn to_int32(&self, sc: &mut LocalScope) -> Result<i32, Value> {
        let n = self.to_number(sc)?;
        Ok(n as i32)
    }
}

impl ValueConversion for Value {
    fn to_number(&self, sc: &mut LocalScope) -> Result<f64, Value> {
        fn object_to_number(this: &Value, obj: &dyn Object, sc: &mut LocalScope) -> Result<f64, Value> {
            if let Some(prim) = obj.as_builtin_capable() {
                ValueConversion::to_number(prim, sc)
            } else {
                let prim = this.to_primitive(sc, Some(PreferredType::Number))?;
                prim.to_number(sc)
            }
        }

        match self {
            Value::Number(n) => Ok(*n),
            Value::Undefined(_) => Ok(f64::NAN),
            Value::Null(_) => Ok(0.0),
            Value::Boolean(b) => Ok(*b as i8 as f64),
            Value::String(s) => s.parse().or_else(|e| throw!(sc, "{}", e)),
            Value::Symbol(_) => throw!(sc, "Cannot convert symbol to number"),
            Value::Object(o) | Value::External(o) => object_to_number(self, o, sc),
        }
    }

    fn to_boolean(&self) -> Result<bool, Value> {
        match self {
            Value::Boolean(b) => Ok(*b),
            Value::Undefined(_) => Ok(false),
            Value::Null(_) => Ok(false),
            Value::Number(n) => Ok(*n != 0.0 && !n.is_nan()),
            Value::String(s) => Ok(!s.is_empty()),
            Value::Symbol(_) => Ok(true),
            Value::Object(_) => Ok(true),
            _ => todo!(), // TODO: implement other cases
        }
    }

    fn to_string(&self, sc: &mut LocalScope) -> Result<Rc<str>, Value> {
        fn object_to_string(this: &Value, obj: &dyn Object, sc: &mut LocalScope) -> Result<Rc<str>, Value> {
            if let Some(prim) = obj.as_builtin_capable() {
                ValueConversion::to_string(prim, sc)
            } else {
                let prim_value = this.to_primitive(sc, Some(PreferredType::String))?;
                prim_value.to_string(sc)
            }
        }

        match self {
            Value::String(s) => ValueConversion::to_string(s, sc),
            Value::Boolean(b) => ValueConversion::to_string(b, sc),
            Value::Null(n) => ValueConversion::to_string(n, sc),
            Value::Undefined(u) => ValueConversion::to_string(u, sc),
            Value::Number(n) => ValueConversion::to_string(n, sc),
            Value::External(o) | Value::Object(o) => object_to_string(self, o, sc),
            Value::Symbol(_) => throw!(sc, "Cannot convert symbol to a string"),
        }
    }

    fn to_primitive(&self, sc: &mut LocalScope, preferred_type: Option<PreferredType>) -> Result<Value, Value> {
        // 1. If Type(input) is Object, then
        if let Value::Object(obj) | Value::External(obj) = self {
            if let Some(prim) = obj.as_builtin_capable() {
                return prim.to_primitive(sc, preferred_type);
            }

            // a. Let exoticToPrim be ? GetMethod(input, @@toPrimitive).
            let to_primitive = sc.statics.symbol_to_primitive.clone();
            let exotic_to_prim = self.get_property(sc, to_primitive.into())?.into_option();

            // i. If preferredType is not present, let hint be "default".
            let preferred_type = preferred_type.unwrap_or(PreferredType::Default);

            // b. If exoticToPrim is not undefined, then
            if let Some(exotic_to_prim) = exotic_to_prim {
                let preferred_type = preferred_type.to_value(sc);

                // iv. Let result be ? Call(exoticToPrim, input, « hint »).
                let result = exotic_to_prim.apply(sc, self.clone(), vec![preferred_type])?;

                // If Type(result) is not Object, return result.
                // TODO: this can still be an object if Value::External
                if !matches!(result, Value::Object(_)) {
                    return Ok(result);
                }

                // vi. Throw a TypeError exception.
                throw!(sc, "Failed to convert to primitive");
            }

            self.ordinary_to_primitive(sc, preferred_type)
        } else {
            Ok(self.clone())
        }
    }

    fn length_of_array_like(&self, sc: &mut LocalScope) -> Result<usize, Value> {
        self.get_property(sc, "length".into())?.to_length_u(sc)
    }

    fn to_object(&self, sc: &mut LocalScope) -> Result<Handle<dyn Object>, Value> {
        fn register_dyn<O: Object + 'static, F: Fn(&mut LocalScope) -> O>(
            sc: &mut LocalScope,
            fun: F,
        ) -> Result<Handle<dyn Object>, Value> {
            let obj = fun(sc);
            Ok(sc.register(obj))
        }

        match self {
            Value::Object(o) => Ok(o.clone()),
            Value::Undefined(_) => throw!(sc, "Cannot convert undefined to object"),
            Value::Null(_) => throw!(sc, "Cannot convert null to object"),
            Value::Boolean(b) => register_dyn(sc, |sc| Boolean::new(sc, *b)),
            Value::Symbol(s) => register_dyn(sc, |sc| BoxedSymbol::new(sc, s.clone())),
            Value::Number(n) => register_dyn(sc, |sc| BoxedNumber::new(sc, *n)),
            Value::String(s) => register_dyn(sc, |sc| BoxedString::new(sc, s.clone())),
            Value::External(e) => Ok(e.clone()), // TODO: is this correct?
        }
    }
}

pub enum PreferredType {
    Default,
    String,
    Number,
}

impl PreferredType {
    pub fn to_value(&self, vm: &Vm) -> Value {
        let st = match self {
            PreferredType::Default => vm.statics.default_str.clone(),
            PreferredType::String => vm.statics.string_str.clone(),
            PreferredType::Number => vm.statics.number_str.clone(),
        };

        Value::String(st)
    }
}

impl Value {
    pub fn ordinary_to_primitive(&self, sc: &mut LocalScope, preferred_type: PreferredType) -> Result<Value, Value> {
        let method_names = match preferred_type {
            PreferredType::String => ["toString", "valueOf"],
            PreferredType::Number | PreferredType::Default => ["valueOf", "toString"],
        };

        for name in method_names {
            let method = self.get_property(sc, name.into())?;
            if matches!(method.type_of(), Typeof::Function) {
                let this = self.clone();
                let result = method.apply(sc, this, Vec::new())?;
                if !matches!(result, Value::Object(_)) {
                    return Ok(result);
                }
            }
        }

        throw!(sc, "Failed to convert to primitive")
    }
}
