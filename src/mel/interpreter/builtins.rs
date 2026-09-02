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
    collections::HashMap,
    fmt::{Debug, Display},
    sync::Arc,
};

use brooks_macros::builtin_function_interpreter;

use crate::mel::{
    interpreter::interpret::{BuiltinFunction, TypedValue, Value},
    scope::Scope,
    tvs::{
        Add_Query_MultiBuiltin, Add_QueryBuiltin, BooleanBuiltin, BuiltinFunctionType,
        IntegerBuiltin, Keep_Query_MultiBuiltin, LowerBuiltin, Match_ReplaceBuiltin, MatchBuiltin,
        Path_ElementBuiltin, Path_ElementsBuiltin, RealBuiltin, Remove_Query_MultiBuiltin,
        Remove_QueryBuiltin, StringBuiltin, Type, UpperBuiltin,
    },
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

#[builtin_function_interpreter(Type::String, Type::String, Type::String)]
impl MatchBuiltin {
    fn interp(&self, input: &str, mtch: &str) -> BuiltinInterpResult {
        let re = regex::RegexBuilder::new(mtch).build().map_err(|_| {
            // TODO: Figure out how to use regex error messages nicely.
            BuiltinInterpError::RuntimeError(format!("{mtch} is not a valid regular expression"))
        })?;

        let result = match re.find(input) {
            Some(found) => found.as_str().to_string(),
            None => "".to_string(),
        };
        Ok(TypedValue {
            value: Value::String(result),
            tipe: Type::String,
        })
    }
}

#[builtin_function_interpreter(Type::String, Type::String, Type::String, Type::String)]
impl Match_ReplaceBuiltin {
    fn interp(&self, input: &str, mtch: &str, replace: &str) -> BuiltinInterpResult {
        let re = regex::RegexBuilder::new(mtch).build().map_err(|_| {
            // TODO: Figure out how to use regex error messages nicely.
            BuiltinInterpError::RuntimeError(format!("{mtch} is not a valid regular expression"))
        })?;

        let result = re.replace(input, replace);

        Ok(TypedValue {
            value: Value::String(result.to_string()),
            tipe: Type::String,
        })
    }
}

#[derive(Debug, Clone, Default)]
struct ParsedQuery {
    elems: HashMap<String, String>,
    order: Vec<String>,
}

impl TryFrom<&str> for ParsedQuery {
    type Error = BuiltinInterpError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // Based on https://url.spec.whatwg.org/#urlencoded-parsing
        let sequences = value.split('&');
        let mut elems: HashMap<String, String> = HashMap::new();
        let mut order: Vec<String> = vec![];
        for bytes in sequences {
            if bytes.is_empty() {
                continue;
            }

            let (name, value) = match bytes.split_once('=') {
                None => (bytes.to_string(), "".to_string()),
                Some((n, v)) => (n.to_string(), v.to_string()),
            };

            elems.insert(name.clone(), value);
            order.push(name);
        }

        Ok(ParsedQuery { elems, order })
    }
}

impl Display for ParsedQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res: Vec<_> = self
            .order
            .iter()
            .map(|e| {
                if let Some((k, v)) = self.elems.get_key_value(e) {
                    let rest = if !v.is_empty() {
                        format!("={v}")
                    } else {
                        "".to_string()
                    };
                    k.to_string() + &rest
                } else {
                    "".to_string()
                }
            })
            .filter(|e| !e.is_empty())
            .collect();

        write!(f, "{}", res.join("&"))
    }
}

#[builtin_function_interpreter(Type::String, Type::String, Type::String, Type::String)]
impl Add_QueryBuiltin {
    fn interp(&self, existing: &str, newq: &str, newv: &str) -> BuiltinInterpResult {
        let mut pq = ParsedQuery::try_from(existing)?;

        if pq
            .elems
            .insert(newq.to_string(), newv.to_string())
            .is_none()
        {
            pq.order.push(newq.to_string())
        }

        Ok(TypedValue {
            value: Value::String(pq.to_string()),
            tipe: Type::String,
        })
    }
}

#[builtin_function_interpreter(Type::String, Type::String, Type::String)]
impl Add_Query_MultiBuiltin {
    fn interp(&self, existing: &str, news: &str) -> BuiltinInterpResult {
        let mut pq = ParsedQuery::try_from(existing)?;

        for newi in news.split(',') {
            if let Some((n, v)) = newi.split_once('=')
                && !v.is_empty()
                && pq.elems.insert(n.to_string(), v.to_string()).is_none()
            {
                pq.order.push(n.to_string())
            };
        }

        Ok(TypedValue {
            value: Value::String(pq.to_string()),
            tipe: Type::String,
        })
    }
}

#[builtin_function_interpreter(Type::String, Type::String, Type::String)]
impl Remove_QueryBuiltin {
    fn interp(&self, existing: &str, oldq: &str) -> BuiltinInterpResult {
        let mut pq = ParsedQuery::try_from(existing)?;

        pq.elems.remove(oldq);

        Ok(TypedValue {
            value: Value::String(pq.to_string()),
            tipe: Type::String,
        })
    }
}

#[builtin_function_interpreter(Type::String, Type::String, Type::String)]
impl Remove_Query_MultiBuiltin {
    fn interp(&self, existing: &str, news: &str) -> BuiltinInterpResult {
        let mut pq = ParsedQuery::try_from(existing)?;

        for oldi in news.split(',') {
            pq.elems.remove(oldi);
        }

        Ok(TypedValue {
            value: Value::String(pq.to_string()),
            tipe: Type::String,
        })
    }
}

#[builtin_function_interpreter(Type::String, Type::String, Type::String)]
impl Keep_Query_MultiBuiltin {
    fn interp(&self, existing: &str, keeps: &str) -> BuiltinInterpResult {
        let mut pq = ParsedQuery::try_from(existing)?;

        // TODO: This could be so much nicer.
        let keep: Vec<_> = keeps.split(',').collect();
        let keys: Vec<_> = pq.elems.keys().cloned().collect();

        for k in keys {
            if !keep.contains(&k.as_str()) {
                pq.elems.remove(&k);
            }
        }

        Ok(TypedValue {
            value: Value::String(pq.to_string()),
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

#[builtin_function_interpreter(Type::String, Type::String)]
impl UpperBuiltin {
    fn interp(&self, s: &str) -> BuiltinInterpResult {
        Ok(TypedValue {
            value: Value::String(s.to_uppercase()),
            tipe: Type::String,
        })
    }
}

#[builtin_function_interpreter(Type::String, Type::String)]
impl LowerBuiltin {
    fn interp(&self, s: &str) -> BuiltinInterpResult {
        Ok(TypedValue {
            value: Value::String(s.to_lowercase()),
            tipe: Type::String,
        })
    }
}

impl BuiltinFunctionInterpreter for IntegerBuiltin {
    fn interpw(&self, args: Value) -> BuiltinInterpResult {
        let args = match args {
            Value::ArgumentList(args) => args,
            _ => return Err(BuiltinInterpError::ArgumentsInvalid.into()),
        };

        if args.len() != 1 {
            return Err(BuiltinInterpError::ArgumentMiscount(1, args.len()).into());
        }

        // Now, interpret!

        todo!()
    }
}

impl BuiltinFunctionInterpreter for RealBuiltin {
    fn interpw(&self, args: Value) -> BuiltinInterpResult {
        let args = match args {
            Value::ArgumentList(args) => args,
            _ => return Err(BuiltinInterpError::ArgumentsInvalid.into()),
        };

        if args.len() != 1 {
            return Err(BuiltinInterpError::ArgumentMiscount(1, args.len()).into());
        }

        // Now, interpret!
        todo!()
    }
}

impl BuiltinFunctionInterpreter for StringBuiltin {
    fn interpw(&self, args: Value) -> BuiltinInterpResult {
        let args = match args {
            Value::ArgumentList(args) => args,
            _ => return Err(BuiltinInterpError::ArgumentsInvalid.into()),
        };

        if args.len() != 1 {
            return Err(BuiltinInterpError::ArgumentMiscount(1, args.len()).into());
        }

        // Now, interpret!
        todo!()
    }
}

macro_rules! add_builtin_function_interpreter_to_scope {
    ($scope:ident, $builtin:ident) => {
        $scope.insert(
            &$builtin.name(),
            TypedValue {
                value: Value::Function(Arc::new($builtin.clone())),
                tipe: Type::Function(
                    $builtin.name(),
                    $builtin.return_type_calculator(),
                    $builtin.params_type_checker(),
                ),
            },
        )
    };
}

pub fn builtin_builtin_function_interpreters() -> Scope<TypedValue> {
    let path_element = Path_ElementBuiltin {};
    let path_elements = Path_ElementsBuiltin {};
    let mtch = MatchBuiltin {};
    let match_replace = Match_ReplaceBuiltin {};
    let add_query = Add_QueryBuiltin {};
    let add_query_multi = Add_Query_MultiBuiltin {};
    let remove_query = Remove_QueryBuiltin {};
    let remove_query_multi = Remove_Query_MultiBuiltin {};
    let keep_query_multi = Keep_Query_MultiBuiltin {};
    let lower = LowerBuiltin {};
    let upper = UpperBuiltin {};
    let boolean = BooleanBuiltin {};

    let mut scopes = Scope::<TypedValue>::default();

    scopes = add_builtin_function_interpreter_to_scope!(scopes, path_element);
    scopes = add_builtin_function_interpreter_to_scope!(scopes, path_elements);
    scopes = add_builtin_function_interpreter_to_scope!(scopes, mtch);
    scopes = add_builtin_function_interpreter_to_scope!(scopes, match_replace);
    scopes = add_builtin_function_interpreter_to_scope!(scopes, add_query);
    scopes = add_builtin_function_interpreter_to_scope!(scopes, add_query_multi);
    scopes = add_builtin_function_interpreter_to_scope!(scopes, remove_query);
    scopes = add_builtin_function_interpreter_to_scope!(scopes, remove_query_multi);
    scopes = add_builtin_function_interpreter_to_scope!(scopes, keep_query_multi);
    scopes = add_builtin_function_interpreter_to_scope!(scopes, lower);
    scopes = add_builtin_function_interpreter_to_scope!(scopes, upper);
    scopes = add_builtin_function_interpreter_to_scope!(scopes, boolean);
    scopes
}
