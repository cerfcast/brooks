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

use std::{error::Error, fmt::Display};

use crate::{
    common::TextLocation,
    mel::{ast::BinaryInfixOperator, interpreter::interpret::MelInterpContext, types::Type},
};

#[derive(Debug, Clone)]
pub enum InterpAssertion {
    TypeMismatch(Type, Type),
    SuccessWithoutValue(String, String),
    UnexpectedOperator(BinaryInfixOperator),
}

impl Display for InterpAssertion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpAssertion::SuccessWithoutValue(what, whre) => write!(
                f,
                "Successful evaluation of subexpression {what} did not yield a value in {whre}"
            ),
            InterpAssertion::UnexpectedOperator(oper) => write!(
                f,
                "Unexpected binary infix operator during evaluation: {oper}",
            ),
            InterpAssertion::TypeMismatch(expected, actual) => {
                write!(f, "Expected type {expected}, have type {actual}",)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum MelInterpPreconditions {}

impl Display for MelInterpPreconditions {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum InterpError {
    Assertion(Box<InterpAssertion>),
    ValueError(Box<TypedValueError>),
    UnknownIdentifier(String),
    UnknownField(String),
    MistypedField(String, Type, Type),
    BuiltinError(Box<BuiltinInterpError>),
}

impl Display for InterpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mel interpreter error: ")?;
        match self {
            InterpError::ValueError(mel_value_error) => {
                write!(f, "value failure: {}", mel_value_error)
            }
            InterpError::Assertion(mel_interp_assertion) => {
                write!(f, "assertion failure: {}", mel_interp_assertion)
            }
            InterpError::UnknownIdentifier(id) => {
                write!(f, "unknown identifier: {}", id)
            }
            InterpError::UnknownField(field) => {
                write!(f, "unknown field: {}", field)
            }
            InterpError::BuiltinError(bi) => {
                write!(f, "error executing builtin function: {}", bi)
            }
            InterpError::MistypedField(field, expected, actual) => {
                write!(
                    f,
                    "error adding field to struct: field {field} has wrong type (expected: {expected}, actual: {actual})",
                )
            }
        }
    }
}

impl Error for InterpError {}

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

#[derive(Debug, Clone)]
pub enum TypedValueError {
    ConversionError(String),
}

impl Display for TypedValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypedValueError::ConversionError(ce) => write!(f, "Conversion error: {ce}"),
        }
    }
}
#[derive(Debug)]
pub struct InterpLocatableError {
    pub error: Box<InterpError>,
    pub location: TextLocation,
    pub context: MelInterpContext,
}

impl Display for InterpLocatableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.error, self.location)
    }
}

impl std::error::Error for InterpLocatableError {}
