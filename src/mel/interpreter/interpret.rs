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
    net::IpAddr,
    sync::Arc,
};

use regex::Regex;

use crate::{
    common::GrammarLocation, environment::scope::Scopes,
    mel::interpreter::builtins::BuiltinInterpError,
};
use crate::{
    logging::{LogLevel, LogMsg, LogMsgs},
    mel::{interpreter::builtins::BuiltinFunctionInterpreter, tvs::BuiltinFunctionType},
};

use crate::mel::{
    analysis::{Analyzed, CompiledConstant},
    ast::{
        self, Argument, ArgumentList, AstVisitor, AstVisitorDriver, AstVisitorResult, BinaryExpr,
        BinaryInfixOperator, BooleanLiteral,
        ComparisonOperator::{self, IP, Re},
        FunctionCall, IPAddressLiteral, Identifier, NumberLiteral, RegexLiteral, StringLiteral,
        TernaryExpr,
    },
    interpreter::interpret::{
        MelInterpAssertion::SuccessWithoutValue, MelInterpError::UnknownIdentifier,
    },
    tvs::{Struct, Type},
};

#[derive(Debug, Clone)]
pub struct StructValue {
    pub fields: HashMap<String, TypedValue>,
    pub tpe: Struct,
}

impl StructValue {
    pub fn new(tpe: Struct) -> StructValue {
        StructValue {
            fields: HashMap::new(),
            tpe,
        }
    }

    pub fn insert_field(
        &mut self,
        name: &str,
        value: TypedValue,
    ) -> Result<(), Box<MelInterpError>> {
        let ft = self
            .tpe
            .get_field(name)
            .ok_or(MelInterpError::UnknownField(name.to_string()))?;

        if ft != value.tipe {
            return Err(
                MelInterpError::MistypedField(name.to_string(), ft.clone(), value.tipe).into(),
            );
        }

        self.fields.insert(name.to_string(), value);
        Ok(())
    }
}

impl Display for StructValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut field_values = self
            .fields
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect::<Vec<_>>();
        field_values.sort();
        let field_values = field_values.join(", ");
        write!(
            f,
            "Type: {}, Field Values: {}",
            self.tpe.name,
            if !field_values.is_empty() {
                field_values
            } else {
                "None".to_string()
            }
        )
    }
}

pub trait BuiltinFunction: BuiltinFunctionType + BuiltinFunctionInterpreter {}

#[derive(Default, Debug, Clone)]
pub enum Value {
    Integer(i64),
    Real(f64),
    String(String),
    Boolean(bool),
    Regex(Regex),
    IPAddress(IpAddr),
    Function(Arc<dyn BuiltinFunction>),
    Struct(StructValue),
    ArgumentList(Vec<TypedValue>),
    #[default]
    Uninitialized,
}

#[derive(Default, Debug, Clone)]
pub struct TypedValue {
    pub value: Value,
    pub tipe: Type,
}

impl Display for TypedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Value::Integer(i) => write!(f, "{i}"),
            Value::Real(i) => write!(f, "{i}"),
            Value::String(s) => write!(f, "\"{s}\""),
            Value::Boolean(b) => write!(f, "{b}"),
            Value::Regex(regex) => write!(f, "{regex}"),
            Value::IPAddress(ip_addr) => write!(f, "{ip_addr}"),
            Value::Function(bft) => write!(f, "Function: {}", bft.name()),
            Value::Struct(struct_value) => write!(f, "{}", struct_value),
            Value::ArgumentList(typed_values) => write!(
                f,
                "Argument List: {}",
                typed_values
                    .iter()
                    .map(|tv| { tv.to_string() })
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Value::Uninitialized => write!(f, "Uninitialized"),
        }
    }
}

impl From<&CompiledConstant> for TypedValue {
    fn from(value: &CompiledConstant) -> Self {
        match value {
            CompiledConstant::Integer(i) => TypedValue {
                value: Value::Integer(*i),
                tipe: Type::Integer,
            },
            CompiledConstant::String(s) => TypedValue {
                value: Value::String(s.clone()),
                tipe: Type::String,
            },
            CompiledConstant::Boolean(b) => TypedValue {
                value: Value::Boolean(*b),
                tipe: Type::Boolean,
            },
            CompiledConstant::IPAddress(ip) => TypedValue {
                value: Value::IPAddress(*ip),
                tipe: Type::IPAddress,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum MelInterpAssertion {
    TypeMismatch(Type, Type),
    SuccessWithoutValue(String, String),
    UnexpectedOperator(BinaryInfixOperator),
}

impl Display for MelInterpAssertion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MelInterpAssertion::SuccessWithoutValue(what, whre) => write!(
                f,
                "Successful evaluation of subexpression {what} did not yield a value in {whre}"
            ),
            MelInterpAssertion::UnexpectedOperator(oper) => write!(
                f,
                "Unexpected binary infix operator during evaluation: {oper}",
            ),
            MelInterpAssertion::TypeMismatch(expected, actual) => {
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
pub enum MelInterpError {
    Assertion(Box<MelInterpAssertion>),
    UnknownIdentifier(String),
    UnknownField(String),
    MistypedField(String, Type, Type),
    BuiltinError(Box<BuiltinInterpError>),
}

impl Display for MelInterpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mel interpreter error: ")?;
        match self {
            MelInterpError::Assertion(mel_interp_assertion) => {
                write!(f, "assertion failure: {}", mel_interp_assertion)
            }
            MelInterpError::UnknownIdentifier(id) => {
                write!(f, "unknown identifier: {}", id)
            }
            MelInterpError::UnknownField(field) => {
                write!(f, "unknown field: {}", field)
            }
            MelInterpError::BuiltinError(bi) => {
                write!(f, "error executing builtin function: {}", bi)
            }
            MelInterpError::MistypedField(field, expected, actual) => {
                write!(
                    f,
                    "error adding field to struct: field {field} has wrong type (expected: {expected}, actual: {actual})",
                )
            }
        }
    }
}

#[derive(Debug)]
pub struct MelInterpLocatableError {
    pub error: Box<MelInterpError>,
    pub location: GrammarLocation,
    pub context: MelInterpContext,
}

impl Display for MelInterpLocatableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.error, self.location)
    }
}

pub type MelInterpResult = Result<MelInterpContext, Box<MelInterpLocatableError>>;

#[derive(Debug, Default)]
pub struct MelInterpContext {
    pub val: Option<TypedValue>,
    pub scopes: Scopes<TypedValue>,
    pub log: LogMsgs,
}

impl MelInterpContext {
    pub fn update_val(self, new: Option<TypedValue>) -> Self {
        MelInterpContext {
            val: new,
            scopes: self.scopes,
            log: self.log,
        }
    }
    pub fn update_scopes(self, new: &Scopes<TypedValue>) -> Self {
        MelInterpContext {
            val: self.val,
            scopes: new.clone(),
            log: self.log,
        }
    }

    pub fn update_log(self, new: LogMsgs) -> Self {
        MelInterpContext {
            val: self.val,
            scopes: self.scopes,
            log: new,
        }
    }
}

pub struct MelInterp {}

impl MelInterp {
    pub fn interp_binary_expr(
        op: &BinaryInfixOperator,
        left: &TypedValue,
        right: &TypedValue,
    ) -> Result<TypedValue, Box<MelInterpError>> {
        match op {
            ast::BinaryInfixOperator::Logic(logic_operator) => {
                let left = if let Value::Boolean(l) = left.value {
                    l
                } else {
                    return Err(MelInterpError::Assertion(
                        MelInterpAssertion::TypeMismatch(Type::Boolean, left.tipe.clone()).into(),
                    )
                    .into());
                };
                let right = if let Value::Boolean(r) = right.value {
                    r
                } else {
                    return Err(MelInterpError::Assertion(
                        MelInterpAssertion::TypeMismatch(Type::Boolean, right.tipe.clone()).into(),
                    )
                    .into());
                };
                match logic_operator {
                    ast::LogicOperator::And => Ok(TypedValue {
                        value: Value::Boolean(left && right),
                        tipe: Type::Boolean,
                    }),
                    ast::LogicOperator::Or => Ok(TypedValue {
                        value: Value::Boolean(left || right),
                        tipe: Type::Boolean,
                    }),
                }
            }
            ast::BinaryInfixOperator::Math(math_operator) => {
                let left = if let Value::Integer(l) = left.value {
                    l
                } else {
                    return Err(MelInterpError::Assertion(
                        MelInterpAssertion::TypeMismatch(Type::Integer, left.tipe.clone()).into(),
                    )
                    .into());
                };
                let right = if let Value::Integer(r) = right.value {
                    r
                } else {
                    return Err(MelInterpError::Assertion(
                        MelInterpAssertion::TypeMismatch(Type::Integer, right.tipe.clone()).into(),
                    )
                    .into());
                };
                match math_operator {
                    ast::MathOperator::Plus => Ok(TypedValue {
                        value: Value::Integer(left + right),
                        tipe: Type::Integer,
                    }),
                    ast::MathOperator::Minus => Ok(TypedValue {
                        value: Value::Integer(left - right),
                        tipe: Type::Integer,
                    }),
                    ast::MathOperator::Multiply => Ok(TypedValue {
                        value: Value::Integer(left * right),
                        tipe: Type::Integer,
                    }),
                    ast::MathOperator::Divide => Ok(TypedValue {
                        value: Value::Integer(left / right),
                        tipe: Type::Integer,
                    }),
                    ast::MathOperator::Modulo => Ok(TypedValue {
                        value: Value::Integer(left % right),
                        tipe: Type::Integer,
                    }),
                }
            }
            ast::BinaryInfixOperator::Concat(_) => {
                let left = if let Value::String(l) = &left.value {
                    l
                } else {
                    return Err(MelInterpError::Assertion(
                        MelInterpAssertion::TypeMismatch(Type::String, left.tipe.clone()).into(),
                    )
                    .into());
                };
                let right = if let Value::String(r) = &right.value {
                    r
                } else {
                    return Err(MelInterpError::Assertion(
                        MelInterpAssertion::TypeMismatch(Type::String, right.tipe.clone()).into(),
                    )
                    .into());
                };
                Ok(TypedValue {
                    value: Value::String(left.clone() + right),
                    tipe: Type::String,
                })
            }
            ast::BinaryInfixOperator::Comparison(IP) => {
                let left = if let Value::IPAddress(l) = left.value {
                    l
                } else {
                    return Err(MelInterpError::Assertion(
                        MelInterpAssertion::TypeMismatch(Type::IPAddress, left.tipe.clone()).into(),
                    )
                    .into());
                };
                let right = if let Value::IPAddress(r) = right.value {
                    r
                } else {
                    return Err(MelInterpError::Assertion(
                        MelInterpAssertion::TypeMismatch(Type::IPAddress, right.tipe.clone())
                            .into(),
                    )
                    .into());
                };
                Ok(TypedValue {
                    value: Value::Boolean(left == right),
                    tipe: Type::Boolean,
                })
            }
            ast::BinaryInfixOperator::Comparison(Re) => {
                let left = if let Value::String(l) = &left.value {
                    l
                } else {
                    return Err(MelInterpError::Assertion(
                        MelInterpAssertion::TypeMismatch(Type::String, left.tipe.clone()).into(),
                    )
                    .into());
                };
                let right = if let Value::Regex(r) = &right.value {
                    r
                } else {
                    return Err(MelInterpError::Assertion(
                        MelInterpAssertion::TypeMismatch(Type::Regex, right.tipe.clone()).into(),
                    )
                    .into());
                };
                Ok(TypedValue {
                    value: Value::Boolean(right.is_match(left)),
                    tipe: Type::String,
                })
            }
            ast::BinaryInfixOperator::Comparison(cop) => match left.tipe {
                Type::Boolean => {
                    let left = if let Value::Boolean(l) = left.value {
                        l
                    } else {
                        return Err(MelInterpError::Assertion(
                            MelInterpAssertion::TypeMismatch(Type::Boolean, left.tipe.clone())
                                .into(),
                        )
                        .into());
                    };
                    let right = if let Value::Boolean(r) = right.value {
                        r
                    } else {
                        return Err(MelInterpError::Assertion(
                            MelInterpAssertion::TypeMismatch(Type::Boolean, right.tipe.clone())
                                .into(),
                        )
                        .into());
                    };

                    match cop {
                        ComparisonOperator::Eq => Ok(TypedValue {
                            value: Value::Boolean(left == right),
                            tipe: Type::Boolean,
                        }),
                        #[allow(clippy::bool_comparison)]
                        ComparisonOperator::Lt => Ok(TypedValue {
                            value: Value::Boolean(left < right),
                            tipe: Type::Boolean,
                        }),
                        ComparisonOperator::Lte => Ok(TypedValue {
                            value: Value::Boolean(left <= right),
                            tipe: Type::Boolean,
                        }),
                        #[allow(clippy::bool_comparison)]
                        ComparisonOperator::Gt => Ok(TypedValue {
                            value: Value::Boolean(left > right),
                            tipe: Type::Boolean,
                        }),
                        ComparisonOperator::Gte => Ok(TypedValue {
                            value: Value::Boolean(left >= right),
                            tipe: Type::Boolean,
                        }),
                        _ => todo!(),
                    }
                }
                Type::Integer => {
                    let left = if let Value::Integer(l) = left.value {
                        l
                    } else {
                        return Err(MelInterpError::Assertion(
                            MelInterpAssertion::TypeMismatch(Type::Integer, left.tipe.clone())
                                .into(),
                        )
                        .into());
                    };
                    let right = if let Value::Integer(r) = right.value {
                        r
                    } else {
                        return Err(MelInterpError::Assertion(
                            MelInterpAssertion::TypeMismatch(Type::Integer, right.tipe.clone())
                                .into(),
                        )
                        .into());
                    };

                    match cop {
                        ComparisonOperator::Eq => Ok(TypedValue {
                            value: Value::Boolean(left == right),
                            tipe: Type::Boolean,
                        }),
                        ComparisonOperator::Lt => Ok(TypedValue {
                            value: Value::Boolean(left < right),
                            tipe: Type::Boolean,
                        }),
                        ComparisonOperator::Lte => Ok(TypedValue {
                            value: Value::Boolean(left <= right),
                            tipe: Type::Boolean,
                        }),
                        ComparisonOperator::Gt => Ok(TypedValue {
                            value: Value::Boolean(left > right),
                            tipe: Type::Boolean,
                        }),
                        ComparisonOperator::Gte => Ok(TypedValue {
                            value: Value::Boolean(left >= right),
                            tipe: Type::Boolean,
                        }),
                        _ => todo!(),
                    }
                }
                Type::String => {
                    let left = if let Value::String(l) = &left.value {
                        l
                    } else {
                        return Err(MelInterpError::Assertion(
                            MelInterpAssertion::TypeMismatch(Type::String, left.tipe.clone())
                                .into(),
                        )
                        .into());
                    };
                    let right = if let Value::String(r) = &right.value {
                        r
                    } else {
                        return Err(MelInterpError::Assertion(
                            MelInterpAssertion::TypeMismatch(Type::String, right.tipe.clone())
                                .into(),
                        )
                        .into());
                    };

                    match cop {
                        ComparisonOperator::Eq => Ok(TypedValue {
                            value: Value::Boolean(left == right),
                            tipe: Type::Boolean,
                        }),
                        ComparisonOperator::Lt => Ok(TypedValue {
                            value: Value::Boolean(left < right),
                            tipe: Type::Boolean,
                        }),
                        ComparisonOperator::Lte => Ok(TypedValue {
                            value: Value::Boolean(left <= right),
                            tipe: Type::Boolean,
                        }),
                        ComparisonOperator::Gt => Ok(TypedValue {
                            value: Value::Boolean(left > right),
                            tipe: Type::Boolean,
                        }),
                        ComparisonOperator::Gte => Ok(TypedValue {
                            value: Value::Boolean(left >= right),
                            tipe: Type::Boolean,
                        }),
                        _ => todo!(),
                    }
                }
                _ => todo!(),
            },
            ast::BinaryInfixOperator::MemberAccess(_) => Err(MelInterpError::Assertion(
                MelInterpAssertion::UnexpectedOperator(op.clone()).into(),
            )
            .into()),
        }
    }
}

macro_rules! use_log {
    ($context:ident, $logx:expr) => {
        match $context {
            MelInterpContext { val, scopes, log } => {
                let logp = $logx(log);
                MelInterpContext {
                    log: logp,
                    scopes,
                    val,
                }
            }
        }
    };
}

#[macro_export]
macro_rules! use_constant {
    ($const:expr, $loc:expr, $context:ident) => {
        if let Some(constant) = &$const {
            let $context = use_log!($context, |log: LogMsgs| {
                trace_with_loc!(log, $loc.clone(), "Using constant")
            });

            return Ok($context.update_val(Some(constant.into())));
        }
    };
}

impl AstVisitor<MelInterpContext, Analyzed, Box<MelInterpLocatableError>> for MelInterp {
    fn visit_function_call(
        &self,
        ast: &FunctionCall<Analyzed>,
        context: MelInterpContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelInterpContext, Box<MelInterpLocatableError>> {
        let context = use_log!(context, |log: LogMsgs| {
            trace_with_loc!(
                log,
                ast.location.clone(),
                "Evaluating function call expression"
            )
        });

        use_constant!(ast.aug.constant, ast.location, context);

        let context = context.update_val(None);
        let context = driver.visit(&ast.callee, self, context)?;

        let callee_value = match &context.val {
            Some(v) => v.clone(),
            None => {
                return Err(MelInterpLocatableError {
                    error: MelInterpError::Assertion(
                        SuccessWithoutValue(
                            "callee".to_string(),
                            "visit_function_call".to_string(),
                        )
                        .into(),
                    )
                    .into(),
                    location: ast.callee.location(),
                    context,
                }
                .into());
            }
        };

        let callee_value = match callee_value {
            TypedValue {
                value: Value::Function(f),
                tipe: _,
            } => f,
            TypedValue { value: _, tipe: t } => {
                return Err(MelInterpLocatableError {
                    error: MelInterpError::Assertion(
                        MelInterpAssertion::TypeMismatch(ast.callee.tipe(), t.clone()).into(),
                    )
                    .into(),
                    location: ast.callee.location(),
                    context,
                }
                .into());
            }
        };

        let context = context.update_val(None);
        let context = self.visit_argument_list(&ast.arguments, context, driver)?;

        let argument_list_values = match &context.val {
            Some(v) => v.clone(),
            None => {
                return Err(MelInterpLocatableError {
                    error: MelInterpError::Assertion(
                        SuccessWithoutValue(
                            "arguments".to_string(),
                            "visit_function_call".to_string(),
                        )
                        .into(),
                    )
                    .into(),
                    location: ast.callee.location(),
                    context,
                }
                .into());
            }
        };

        let argument_list_values = match argument_list_values {
            TypedValue {
                value: f @ Value::ArgumentList(_),
                tipe: _,
            } => f,
            TypedValue { value: _, tipe: t } => {
                return Err(MelInterpLocatableError {
                    error: MelInterpError::Assertion(
                        MelInterpAssertion::TypeMismatch(ast.callee.tipe(), t.clone()).into(),
                    )
                    .into(),
                    location: ast.callee.location(),
                    context,
                }
                .into());
            }
        };

        let res = match (*callee_value).interpw(argument_list_values.clone()) {
            Ok(res) => res,
            Err(e) => {
                return Err(MelInterpLocatableError {
                    error: MelInterpError::BuiltinError(e).into(),
                    location: ast.location.clone(),
                    context,
                }
                .into());
            }
        };
        Ok(context.update_val(Some(res)))
    }

    fn visit_identifier(
        &self,
        ast: &Identifier<Analyzed>,
        context: MelInterpContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelInterpContext, Box<MelInterpLocatableError>> {
        let context = use_log!(context, |log: LogMsgs| {
            trace_with_loc!(
                log,
                ast.location.clone(),
                "Evaluating identifier expression"
            )
        });

        let found_id = match context.scopes.lookup(&ast.identifier) {
            Some(v) => v,
            None => {
                return Err(MelInterpLocatableError {
                    error: UnknownIdentifier(ast.identifier.clone()).into(),
                    location: ast.location.clone(),
                    context,
                }
                .into());
            }
        };

        Ok(context.update_val(Some(found_id)))
    }

    fn visit_argument_list(
        &self,
        ast: &ArgumentList<Analyzed>,
        context: MelInterpContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelInterpContext, Box<MelInterpLocatableError>> {
        let context = use_log!(context, |log: LogMsgs| {
            trace_with_loc!(
                log,
                ast.location.clone(),
                "Evaluating argument list expression"
            )
        });

        use_constant!(ast.aug.constant, ast.location, context);

        let mut context = context.update_val(None);

        let mut arg_values: Vec<TypedValue> = vec![];
        for arg in &ast.arguments {
            context = self.visit_argument(arg, context, driver)?;

            let arg_value = match &context.val {
                Some(v) => v.clone(),
                None => {
                    return Err(MelInterpLocatableError {
                        error: MelInterpError::Assertion(
                            SuccessWithoutValue(
                                "argument".to_string(),
                                "visit_function_call".to_string(),
                            )
                            .into(),
                        )
                        .into(),
                        location: arg.location.clone(),
                        context,
                    }
                    .into());
                }
            };

            arg_values.push(arg_value.clone());
        }

        Ok(context.update_val(Some(TypedValue {
            value: Value::ArgumentList(arg_values),
            tipe: ast.aug.tipe.clone(),
        })))
    }

    fn visit_argument(
        &self,
        ast: &Argument<Analyzed>,
        context: MelInterpContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelInterpContext, Box<MelInterpLocatableError>> {
        let context = use_log!(context, |log: LogMsgs| {
            trace_with_loc!(
                log,
                ast.location.clone(),
                "Evaluating argument list expression"
            )
        });

        use_constant!(ast.aug.constant, ast.location, context);

        let context = context.update_val(None);
        let context = driver.visit(&ast.expr, self, context)?;

        let argument_value = match &context.val {
            Some(v) => v.clone(),
            None => {
                return Err(MelInterpLocatableError {
                    error: MelInterpError::Assertion(
                        SuccessWithoutValue("argument".to_string(), "visit_argument".to_string())
                            .into(),
                    )
                    .into(),
                    location: ast.expr.location(),
                    context,
                }
                .into());
            }
        };

        if argument_value.tipe != ast.aug.tipe {
            return Err(MelInterpLocatableError {
                error: MelInterpError::Assertion(
                    MelInterpAssertion::TypeMismatch(
                        ast.aug.tipe.clone(),
                        argument_value.tipe.clone(),
                    )
                    .into(),
                )
                .into(),
                location: ast.location.clone(),
                context,
            }
            .into());
        }

        Ok(context.update_val(Some(argument_value.clone())))
    }

    fn visit_binary_expr(
        &self,
        ast: &BinaryExpr<Analyzed>,
        context: MelInterpContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelInterpContext, Box<MelInterpLocatableError>> {
        let context = use_log!(context, |log: LogMsgs| {
            trace_with_loc!(log, ast.location.clone(), "Evaluating binary expression")
        });

        use_constant!(ast.aug.constant, ast.location, context);

        let context = context.update_val(None);
        let context = driver.visit(&ast.left, self, context)?;

        let left_value = match &context.val {
            Some(v) => v.clone(),
            None => {
                return Err(MelInterpLocatableError {
                    error: MelInterpError::Assertion(
                        SuccessWithoutValue(
                            "left_operand".to_string(),
                            "visit_binary_expr".to_string(),
                        )
                        .into(),
                    )
                    .into(),
                    location: ast.left.location(),
                    context,
                }
                .into());
            }
        };

        let context = context.update_val(None);
        let context = driver.visit(&ast.right, self, context)?;

        let right_value = match &context.val {
            Some(v) => v.clone(),
            None => {
                return Err(MelInterpLocatableError {
                    error: MelInterpError::Assertion(
                        SuccessWithoutValue(
                            "right_operand".to_string(),
                            "visit_binary_expr".to_string(),
                        )
                        .into(),
                    )
                    .into(),
                    location: ast.right.location(),
                    context,
                }
                .into());
            }
        };

        let result = match Self::interp_binary_expr(&ast.op, &left_value, &right_value) {
            Ok(o) => o,
            Err(e) => {
                return Err(MelInterpLocatableError {
                    error: e,
                    location: ast.location.clone(),
                    context,
                }
                .into());
            }
        };

        Ok(context.update_val(Some(result)))
    }

    fn visit_literal(
        &self,
        ast: (&ast::Literal, &GrammarLocation, &Analyzed),
        context: MelInterpContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelInterpContext, Box<MelInterpLocatableError>> {
        let context = use_log!(context, |log: LogMsgs| {
            trace_with_loc!(log, ast.1.clone(), "Evaluating literal expression")
        });

        match ast {
            (ast::Literal::Boolean(b), _, _) => Ok(context.update_val(Some(TypedValue {
                value: Value::Boolean(*b == BooleanLiteral::True),
                tipe: Type::Boolean,
            }))),
            (ast::Literal::Number(NumberLiteral { literal: l }), _, _) => {
                Ok(context.update_val(Some(TypedValue {
                    value: Value::Integer(*l as i64),
                    tipe: Type::Integer,
                })))
            }
            (ast::Literal::String(StringLiteral { literal: s }), _, _) => {
                Ok(context.update_val(Some(TypedValue {
                    value: Value::String(s.clone()),
                    tipe: Type::String,
                })))
            }
            (ast::Literal::Regex(RegexLiteral { literal: rl }), _, _) => {
                Ok(context.update_val(Some(TypedValue {
                    value: Value::Regex(rl.clone()),
                    tipe: Type::Regex,
                })))
            }
            (ast::Literal::IPAddress(IPAddressLiteral { literal: rl }), _, _) => Ok(context
                .update_val(Some(TypedValue {
                    value: Value::IPAddress(*rl),
                    tipe: Type::IPAddress,
                }))),
        }
    }

    fn visit_ternary_expr(
        &self,
        ast: &TernaryExpr<Analyzed>,
        context: MelInterpContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelInterpContext, Box<MelInterpLocatableError>> {
        let context = use_log!(context, |log: LogMsgs| {
            trace_with_loc!(log, ast.location.clone(), "Evaluating ternary expression")
        });

        use_constant!(ast.aug.constant, ast.location, context);

        let context = context.update_val(None);
        let context = driver.visit(&ast.condition, self, context)?;

        let condition_value = match &context.val {
            Some(v) => v.clone(),
            None => {
                return Err(MelInterpLocatableError {
                    error: MelInterpError::Assertion(
                        SuccessWithoutValue(
                            "condition".to_string(),
                            "visit_ternary_expr".to_string(),
                        )
                        .into(),
                    )
                    .into(),
                    location: ast.condition.location(),
                    context,
                }
                .into());
            }
        };

        let condition_value = match condition_value {
            TypedValue {
                value: Value::Boolean(b),
                tipe: Type::Boolean,
            } => b,
            e => {
                return Err(MelInterpLocatableError {
                    error: MelInterpError::Assertion(
                        MelInterpAssertion::TypeMismatch(Type::Boolean, e.tipe.clone()).into(),
                    )
                    .into(),
                    location: ast.location.clone(),
                    context,
                }
                .into());
            }
        };

        let context = context.update_val(None);

        let (context, result) = if condition_value {
            let context = driver.visit(&ast.yes, self, context)?;

            let val = match &context.val {
                Some(v) => v.clone(),
                None => {
                    return Err(MelInterpLocatableError {
                        error: MelInterpError::Assertion(
                            SuccessWithoutValue(
                                "true branch".to_string(),
                                "visit_ternary_expr".to_string(),
                            )
                            .into(),
                        )
                        .into(),
                        location: ast.condition.location(),
                        context,
                    }
                    .into());
                }
            };
            (context, val)
        } else {
            let context = driver.visit(&ast.no, self, context)?;

            let val = match &context.val {
                Some(v) => v.clone(),
                None => {
                    return Err(MelInterpLocatableError {
                        error: MelInterpError::Assertion(
                            SuccessWithoutValue(
                                "false branch".to_string(),
                                "visit_ternary_expr".to_string(),
                            )
                            .into(),
                        )
                        .into(),
                        location: ast.condition.location(),
                        context,
                    }
                    .into());
                }
            };
            (context, val)
        };

        if result.tipe != ast.aug.tipe {
            return Err(MelInterpLocatableError {
                error: MelInterpError::Assertion(
                    MelInterpAssertion::TypeMismatch(ast.aug.tipe.clone(), result.tipe.clone())
                        .into(),
                )
                .into(),
                location: ast.location.clone(),
                context,
            }
            .into());
        }

        Ok(context.update_val(Some(result.clone())))
    }

    fn visit_member_access_expr(
        &self,
        ast: &ast::MemberAccessExpression<Analyzed>,
        context: MelInterpContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelInterpContext, Box<MelInterpLocatableError>> {
        let context = use_log!(context, |log: LogMsgs| {
            trace_with_loc!(
                log,
                ast.location.clone(),
                "Evaluating member access expression"
            )
        });

        use_constant!(ast.aug.constant, ast.location, context);

        let context = driver.visit(&ast.base, self, context)?;

        let base = match &context.val {
            Some(v) => v.clone(),
            None => {
                return Err(MelInterpLocatableError {
                    error: MelInterpError::Assertion(
                        SuccessWithoutValue(
                            "base".to_string(),
                            "visit_member_access_expr".to_string(),
                        )
                        .into(),
                    )
                    .into(),
                    location: ast.base.location(),
                    context,
                }
                .into());
            }
        };

        let (base_value, base_type) = match base {
            TypedValue {
                value: Value::Struct(sv),
                tipe: Type::Struct(st),
            } => (sv, st),
            t => {
                return Err(MelInterpLocatableError {
                    error: MelInterpError::Assertion(
                        MelInterpAssertion::TypeMismatch(ast.base.tipe(), t.tipe).into(),
                    )
                    .into(),
                    location: ast.base.location(),
                    context,
                }
                .into());
            }
        };

        // Make sure that the evaluated type and the analyzed type are the same.
        if Type::Struct(base_type.clone()) != ast.base.tipe() {
            return Err(MelInterpLocatableError {
                error: MelInterpError::Assertion(
                    MelInterpAssertion::TypeMismatch(ast.base.tipe(), Type::Struct(base_type))
                        .into(),
                )
                .into(),
                location: ast.base.location(),
                context,
            }
            .into());
        }

        let member_type = match base_type.get_field(&ast.member.identifier) {
            Some(v) => v.clone(),
            None => {
                return Err(MelInterpLocatableError {
                    error: MelInterpError::UnknownField(ast.member.identifier.clone()).into(),
                    location: ast.member.location.clone(),
                    context,
                })?;
            }
        };

        let member_value = match base_value.fields.get(&ast.member.identifier) {
            Some(v) => v.clone(),
            None => {
                return Err(MelInterpLocatableError {
                    error: MelInterpError::UnknownField(ast.member.identifier.clone()).into(),
                    location: ast.member.location.clone(),
                    context,
                }
                .into());
            }
        };

        if member_type != member_value.tipe {
            return Err(MelInterpLocatableError {
                error: MelInterpError::Assertion(
                    MelInterpAssertion::TypeMismatch(
                        member_type.clone(),
                        member_value.tipe.clone(),
                    )
                    .into(),
                )
                .into(),
                location: ast.member.location.clone(),
                context,
            }
            .into());
        }

        Ok(context.update_val(Some(member_value)))
    }
}
