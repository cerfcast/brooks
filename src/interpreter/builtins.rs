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

use std::fmt::{Debug, Display};

use brooks_macros::builtin_function;

use crate::{
    interpreter::{TypedValue, Value},
    tvs::{self, Type},
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
                "Expected argument {which} to have type {} but it has {}",
                expected.to_string(),
                actual.to_string()
            ),
            BuiltinInterpError::ArgumentsInvalid => write!(f, "Invalid arguments"),
            BuiltinInterpError::RuntimeError(e) => write!(f, "Runtime error: {e}"),
        }
    }
}

type BuiltinInterpResult = Result<TypedValue, BuiltinInterpError>;

#[allow(clippy::result_large_err)]
pub trait BuiltinFunction: Debug {
    fn name(&self) -> String;
    fn parameters(&self) -> tvs::Params;
    fn return_type(&self) -> Type;
    fn interpw(&self, args: Value) -> BuiltinInterpResult;
}

#[allow(non_camel_case_types)]
#[derive(Clone, Default, Debug)]
#[builtin_function(Type::String, Type::String, Type::Integer)]
pub struct Path_ElementBuiltin {}

#[allow(clippy::result_large_err)]
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
            )))
        }
    }
}

#[derive(Debug, Clone)]
#[builtin_function(Type::Boolean, Type::Integer)]
pub struct BooleanBuiltin {}

#[allow(clippy::result_large_err)]
impl BooleanBuiltin {
    fn interp(&self, c: &i64) -> BuiltinInterpResult {
        Ok(TypedValue {
            value: Value::Boolean(*c != 0),
            tipe: Type::Boolean,
        })
    }
}
