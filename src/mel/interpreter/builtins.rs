// brooks, Copyright 2026, Will Hawkins
//
// This file is part of brooks.

// This file is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use brooks_macros::builtin_function_interpreter;

use crate::mel::{
    interpreter::interpret::{BuiltinFunction, TypedValue, Value},
    scope::Scope,
    tvs::{BooleanBuiltin, BuiltinFunctionType, Path_ElementBuiltin, Path_ElementsBuiltin, Type},
};

#[derive(Debug, Clone)]
pub enum BuiltinInterpError {
    ArgumentMiscount(usize, usize),
    ArgumentMismatch(usize, Type, Type),
    RuntimeError(String),
    ArgumentsInvalid,
}

impl Display for BuiltinInterpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuiltinInterpError::ArgumentMiscount(expected, actual) => {
                write!(f, "Expected {expected} arguments but got {actual}")
            }
            BuiltinInterpError::ArgumentMismatch(which, expected, actual) => write!(
                f,
                "Expected argument {which} to have type {expected} but it has {actual}",
            ),
            BuiltinInterpError::ArgumentsInvalid => write!(f, "Invalid arguments"),
            BuiltinInterpError::RuntimeError(e) => write!(f, "Runtime error: {e}"),
        }
    }
}

type BuiltinInterpResult = Result<TypedValue, Box<BuiltinInterpError>>;

pub trait BuiltinFunctionInterpreter: Debug {
    fn interpw(&self, args: Value) -> BuiltinInterpResult;
}

#[builtin_function_interpreter(Type::String, Type::String, Type::Integer)]
impl Path_ElementBuiltin {
    fn interp(&self, path: &str, element: &i64) -> BuiltinInterpResult {
        let parts = path.split("/");

        if (*element as usize) < parts.clone().count() {
            let part =
                parts
                    .clone()
                    .nth((*element) as usize)
                    .ok_or(BuiltinInterpError::RuntimeError(format!(
                        "Index {} is out of bounds (max {})",
                        element,
                        parts.count()
                    )))?;
            Ok(TypedValue {
                value: Value::String(part.to_string()),
                tipe: Type::String,
            })
        } else {
            Err(BuiltinInterpError::RuntimeError(format!(
                "Index {} is out of bounds (max {})",
                element,
                parts.count()
            ))
            .into())
        }
    }
}

#[builtin_function_interpreter(Type::String, Type::String, Type::Integer, Type::Integer)]
impl Path_ElementsBuiltin {
    fn interp(&self, path: &str, element_n: &i64, element_m: &i64) -> BuiltinInterpResult {
        let parts = path.split("/");

        let element_n = *element_n as usize;
        let element_m = *element_m as usize;

        if element_m < element_n {
            return Err(BuiltinInterpError::RuntimeError(format!(
                "Cannot access elements from {element_n} to {element_m} -- out of order",
            ))
            .into());
        }

        let result = parts.skip(element_n).take(element_m - element_n + 1); // + 1 for inclusive.

        Ok(TypedValue {
            value: Value::String(result.collect::<Vec<_>>().join("/")),
            tipe: Type::String,
        })
    }
}

#[builtin_function_interpreter(Type::Boolean, Type::Integer)]
impl BooleanBuiltin {
    fn interp(&self, c: &i64) -> BuiltinInterpResult {
        Ok(TypedValue {
            value: Value::Boolean(*c != 0),
            tipe: Type::Boolean,
        })
    }
}

pub fn builtin_builtin_function_interpreters() -> Scope<TypedValue> {
    let b = Path_ElementBuiltin {};
    let boolean = BooleanBuiltin {};

    let mut scopes = Scope::<TypedValue>::default();

    scopes = scopes.insert(
        &b.name(),
        TypedValue {
            value: Value::Function(Arc::new(b.clone())),
            tipe: Type::Function(Arc::new(b.return_type()), b.parameters()),
        },
    );
    scopes = scopes.insert(
        &boolean.name(),
        TypedValue {
            value: Value::Function(Arc::new(boolean.clone())),
            tipe: Type::Function(Arc::new(boolean.return_type()), boolean.parameters()),
        },
    );
    scopes
}
