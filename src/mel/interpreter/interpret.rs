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

use std::fmt::Debug;

use crate::{
    common::TextLocation,
    environment::scope::Scopes,
    mel::interpreter::{
        error::{
            InterpAssertion::{self, SuccessWithoutValue},
            InterpError::{self, UnknownIdentifier},
            InterpLocatableError,
        },
        values::{TypedValue, Value},
    },
};
use crate::{
    logging::{LogLevel, LogMsg, LogMsgs},
    mel::{interpreter::builtins::BuiltinFunctionInterpreter, types::BuiltinFunctionType},
};

use crate::mel::{
    analysis::Analyzed,
    ast::{
        self, Argument, ArgumentList, AstVisitor, AstVisitorDriver, AstVisitorResult, BinaryExpr,
        BinaryInfixOperator, BooleanLiteral,
        ComparisonOperator::{self, IP, Re},
        FunctionCall, IPAddressLiteral, Identifier, NumberLiteral, RegexLiteral, StringLiteral,
        TernaryExpr,
    },
    types::Type,
};

pub trait BuiltinFunction: BuiltinFunctionType + BuiltinFunctionInterpreter {}

pub type MelInterpResult = Result<MelInterpContext, Box<InterpLocatableError>>;

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
    ) -> Result<TypedValue, Box<InterpError>> {
        match op {
            ast::BinaryInfixOperator::Logic(logic_operator) => {
                let left = if let Value::Boolean(l) = left.value {
                    l
                } else {
                    return Err(InterpError::Assertion(
                        InterpAssertion::TypeMismatch(Type::Boolean, left.tpe.clone()).into(),
                    )
                    .into());
                };
                let right = if let Value::Boolean(r) = right.value {
                    r
                } else {
                    return Err(InterpError::Assertion(
                        InterpAssertion::TypeMismatch(Type::Boolean, right.tpe.clone()).into(),
                    )
                    .into());
                };
                match logic_operator {
                    ast::LogicOperator::And => Ok(TypedValue {
                        value: Value::Boolean(left && right),
                        tpe: Type::Boolean,
                    }),
                    ast::LogicOperator::Or => Ok(TypedValue {
                        value: Value::Boolean(left || right),
                        tpe: Type::Boolean,
                    }),
                }
            }
            ast::BinaryInfixOperator::Math(math_operator) => {
                let left = if let Value::Integer(l) = left.value {
                    l
                } else {
                    return Err(InterpError::Assertion(
                        InterpAssertion::TypeMismatch(Type::Integer, left.tpe.clone()).into(),
                    )
                    .into());
                };
                let right = if let Value::Integer(r) = right.value {
                    r
                } else {
                    return Err(InterpError::Assertion(
                        InterpAssertion::TypeMismatch(Type::Integer, right.tpe.clone()).into(),
                    )
                    .into());
                };
                match math_operator {
                    ast::MathOperator::Plus => Ok(TypedValue {
                        value: Value::Integer(left + right),
                        tpe: Type::Integer,
                    }),
                    ast::MathOperator::Minus => Ok(TypedValue {
                        value: Value::Integer(left - right),
                        tpe: Type::Integer,
                    }),
                    ast::MathOperator::Multiply => Ok(TypedValue {
                        value: Value::Integer(left * right),
                        tpe: Type::Integer,
                    }),
                    ast::MathOperator::Divide => Ok(TypedValue {
                        value: Value::Integer(left / right),
                        tpe: Type::Integer,
                    }),
                    ast::MathOperator::Modulo => Ok(TypedValue {
                        value: Value::Integer(left % right),
                        tpe: Type::Integer,
                    }),
                }
            }
            ast::BinaryInfixOperator::Concat(_) => {
                let left = if let Value::String(l) = &left.value {
                    l
                } else {
                    return Err(InterpError::Assertion(
                        InterpAssertion::TypeMismatch(Type::String, left.tpe.clone()).into(),
                    )
                    .into());
                };
                let right = if let Value::String(r) = &right.value {
                    r
                } else {
                    return Err(InterpError::Assertion(
                        InterpAssertion::TypeMismatch(Type::String, right.tpe.clone()).into(),
                    )
                    .into());
                };
                Ok(TypedValue {
                    value: Value::String(left.clone() + right),
                    tpe: Type::String,
                })
            }
            ast::BinaryInfixOperator::Comparison(IP) => {
                let left = if let Value::IPAddress(l) = left.value {
                    l
                } else {
                    return Err(InterpError::Assertion(
                        InterpAssertion::TypeMismatch(Type::IPAddress, left.tpe.clone()).into(),
                    )
                    .into());
                };
                let right = if let Value::IPAddress(r) = right.value {
                    r
                } else {
                    return Err(InterpError::Assertion(
                        InterpAssertion::TypeMismatch(Type::IPAddress, right.tpe.clone()).into(),
                    )
                    .into());
                };
                Ok(TypedValue {
                    value: Value::Boolean(left == right),
                    tpe: Type::Boolean,
                })
            }
            ast::BinaryInfixOperator::Comparison(Re) => {
                let left = if let Value::String(l) = &left.value {
                    l
                } else {
                    return Err(InterpError::Assertion(
                        InterpAssertion::TypeMismatch(Type::String, left.tpe.clone()).into(),
                    )
                    .into());
                };
                let right = if let Value::Regex(r) = &right.value {
                    r
                } else {
                    return Err(InterpError::Assertion(
                        InterpAssertion::TypeMismatch(Type::Regex, right.tpe.clone()).into(),
                    )
                    .into());
                };
                Ok(TypedValue {
                    value: Value::Boolean(right.is_match(left)),
                    tpe: Type::Boolean,
                })
            }
            ast::BinaryInfixOperator::Comparison(cop) => match left.tpe {
                Type::Boolean => {
                    let left = if let Value::Boolean(l) = left.value {
                        l
                    } else {
                        return Err(InterpError::Assertion(
                            InterpAssertion::TypeMismatch(Type::Boolean, left.tpe.clone()).into(),
                        )
                        .into());
                    };
                    let right = if let Value::Boolean(r) = right.value {
                        r
                    } else {
                        return Err(InterpError::Assertion(
                            InterpAssertion::TypeMismatch(Type::Boolean, right.tpe.clone()).into(),
                        )
                        .into());
                    };

                    match cop {
                        ComparisonOperator::Eq => Ok(TypedValue {
                            value: Value::Boolean(left == right),
                            tpe: Type::Boolean,
                        }),
                        #[allow(clippy::bool_comparison)]
                        ComparisonOperator::Lt => Ok(TypedValue {
                            value: Value::Boolean(left < right),
                            tpe: Type::Boolean,
                        }),
                        ComparisonOperator::Lte => Ok(TypedValue {
                            value: Value::Boolean(left <= right),
                            tpe: Type::Boolean,
                        }),
                        #[allow(clippy::bool_comparison)]
                        ComparisonOperator::Gt => Ok(TypedValue {
                            value: Value::Boolean(left > right),
                            tpe: Type::Boolean,
                        }),
                        ComparisonOperator::Gte => Ok(TypedValue {
                            value: Value::Boolean(left >= right),
                            tpe: Type::Boolean,
                        }),
                        _ => todo!(),
                    }
                }
                Type::Integer => {
                    let left = if let Value::Integer(l) = left.value {
                        l
                    } else {
                        return Err(InterpError::Assertion(
                            InterpAssertion::TypeMismatch(Type::Integer, left.tpe.clone()).into(),
                        )
                        .into());
                    };
                    let right = if let Value::Integer(r) = right.value {
                        r
                    } else {
                        return Err(InterpError::Assertion(
                            InterpAssertion::TypeMismatch(Type::Integer, right.tpe.clone()).into(),
                        )
                        .into());
                    };

                    match cop {
                        ComparisonOperator::Eq => Ok(TypedValue {
                            value: Value::Boolean(left == right),
                            tpe: Type::Boolean,
                        }),
                        ComparisonOperator::Lt => Ok(TypedValue {
                            value: Value::Boolean(left < right),
                            tpe: Type::Boolean,
                        }),
                        ComparisonOperator::Lte => Ok(TypedValue {
                            value: Value::Boolean(left <= right),
                            tpe: Type::Boolean,
                        }),
                        ComparisonOperator::Gt => Ok(TypedValue {
                            value: Value::Boolean(left > right),
                            tpe: Type::Boolean,
                        }),
                        ComparisonOperator::Gte => Ok(TypedValue {
                            value: Value::Boolean(left >= right),
                            tpe: Type::Boolean,
                        }),
                        _ => todo!(),
                    }
                }
                Type::String => {
                    let left = if let Value::String(l) = &left.value {
                        l
                    } else {
                        return Err(InterpError::Assertion(
                            InterpAssertion::TypeMismatch(Type::String, left.tpe.clone()).into(),
                        )
                        .into());
                    };
                    let right = if let Value::String(r) = &right.value {
                        r
                    } else {
                        return Err(InterpError::Assertion(
                            InterpAssertion::TypeMismatch(Type::String, right.tpe.clone()).into(),
                        )
                        .into());
                    };

                    match cop {
                        ComparisonOperator::Eq => Ok(TypedValue {
                            value: Value::Boolean(left == right),
                            tpe: Type::Boolean,
                        }),
                        ComparisonOperator::Lt => Ok(TypedValue {
                            value: Value::Boolean(left < right),
                            tpe: Type::Boolean,
                        }),
                        ComparisonOperator::Lte => Ok(TypedValue {
                            value: Value::Boolean(left <= right),
                            tpe: Type::Boolean,
                        }),
                        ComparisonOperator::Gt => Ok(TypedValue {
                            value: Value::Boolean(left > right),
                            tpe: Type::Boolean,
                        }),
                        ComparisonOperator::Gte => Ok(TypedValue {
                            value: Value::Boolean(left >= right),
                            tpe: Type::Boolean,
                        }),
                        _ => todo!(),
                    }
                }
                _ => todo!(),
            },
            ast::BinaryInfixOperator::MemberAccess(_) => Err(InterpError::Assertion(
                InterpAssertion::UnexpectedOperator(op.clone()).into(),
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

impl AstVisitor<MelInterpContext, Analyzed, Box<InterpLocatableError>> for MelInterp {
    fn visit_function_call(
        &self,
        ast: &FunctionCall<Analyzed>,
        context: MelInterpContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelInterpContext, Box<InterpLocatableError>> {
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
                return Err(InterpLocatableError {
                    error: InterpError::Assertion(
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
                tpe: _,
            } => f,
            TypedValue { value: _, tpe: t } => {
                return Err(InterpLocatableError {
                    error: InterpError::Assertion(
                        InterpAssertion::TypeMismatch(ast.callee.tipe(), t.clone()).into(),
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
                return Err(InterpLocatableError {
                    error: InterpError::Assertion(
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
                tpe: _,
            } => f,
            TypedValue { value: _, tpe: t } => {
                return Err(InterpLocatableError {
                    error: InterpError::Assertion(
                        InterpAssertion::TypeMismatch(ast.callee.tipe(), t.clone()).into(),
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
                return Err(InterpLocatableError {
                    error: InterpError::BuiltinError(e).into(),
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
    ) -> AstVisitorResult<MelInterpContext, Box<InterpLocatableError>> {
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
                return Err(InterpLocatableError {
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
    ) -> AstVisitorResult<MelInterpContext, Box<InterpLocatableError>> {
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
                    return Err(InterpLocatableError {
                        error: InterpError::Assertion(
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
            tpe: ast.aug.tipe.clone(),
        })))
    }

    fn visit_argument(
        &self,
        ast: &Argument<Analyzed>,
        context: MelInterpContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelInterpContext, Box<InterpLocatableError>> {
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
                return Err(InterpLocatableError {
                    error: InterpError::Assertion(
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

        if argument_value.tpe != ast.aug.tipe {
            return Err(InterpLocatableError {
                error: InterpError::Assertion(
                    InterpAssertion::TypeMismatch(ast.aug.tipe.clone(), argument_value.tpe.clone())
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
    ) -> AstVisitorResult<MelInterpContext, Box<InterpLocatableError>> {
        let context = use_log!(context, |log: LogMsgs| {
            trace_with_loc!(log, ast.location.clone(), "Evaluating binary expression")
        });

        use_constant!(ast.aug.constant, ast.location, context);

        let context = context.update_val(None);
        let context = driver.visit(&ast.left, self, context)?;

        let left_value = match &context.val {
            Some(v) => v.clone(),
            None => {
                return Err(InterpLocatableError {
                    error: InterpError::Assertion(
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
                return Err(InterpLocatableError {
                    error: InterpError::Assertion(
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
                return Err(InterpLocatableError {
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
        ast: (&ast::Literal, &TextLocation, &Analyzed),
        context: MelInterpContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelInterpContext, Box<InterpLocatableError>> {
        let context = use_log!(context, |log: LogMsgs| {
            trace_with_loc!(log, ast.1.clone(), "Evaluating literal expression")
        });

        match ast {
            (ast::Literal::Boolean(b), _, _) => Ok(context.update_val(Some(TypedValue {
                value: Value::Boolean(*b == BooleanLiteral::True),
                tpe: Type::Boolean,
            }))),
            (ast::Literal::Number(NumberLiteral { literal: l }), _, _) => {
                Ok(context.update_val(Some(TypedValue {
                    value: Value::Integer(*l as i64),
                    tpe: Type::Integer,
                })))
            }
            (ast::Literal::String(StringLiteral { literal: s }), _, _) => {
                Ok(context.update_val(Some(TypedValue {
                    value: Value::String(s.clone()),
                    tpe: Type::String,
                })))
            }
            (ast::Literal::Regex(RegexLiteral { literal: rl }), _, _) => {
                Ok(context.update_val(Some(TypedValue {
                    value: Value::Regex(rl.clone()),
                    tpe: Type::Regex,
                })))
            }
            (ast::Literal::IPAddress(IPAddressLiteral { literal: rl }), _, _) => Ok(context
                .update_val(Some(TypedValue {
                    value: Value::IPAddress(*rl),
                    tpe: Type::IPAddress,
                }))),
        }
    }

    fn visit_ternary_expr(
        &self,
        ast: &TernaryExpr<Analyzed>,
        context: MelInterpContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelInterpContext, Box<InterpLocatableError>> {
        let context = use_log!(context, |log: LogMsgs| {
            trace_with_loc!(log, ast.location.clone(), "Evaluating ternary expression")
        });

        use_constant!(ast.aug.constant, ast.location, context);

        let context = context.update_val(None);
        let context = driver.visit(&ast.condition, self, context)?;

        let condition_value = match &context.val {
            Some(v) => v.clone(),
            None => {
                return Err(InterpLocatableError {
                    error: InterpError::Assertion(
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
                tpe: Type::Boolean,
            } => b,
            e => {
                return Err(InterpLocatableError {
                    error: InterpError::Assertion(
                        InterpAssertion::TypeMismatch(Type::Boolean, e.tpe.clone()).into(),
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
                    return Err(InterpLocatableError {
                        error: InterpError::Assertion(
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
                    return Err(InterpLocatableError {
                        error: InterpError::Assertion(
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

        if result.tpe != ast.aug.tipe {
            return Err(InterpLocatableError {
                error: InterpError::Assertion(
                    InterpAssertion::TypeMismatch(ast.aug.tipe.clone(), result.tpe.clone()).into(),
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
    ) -> AstVisitorResult<MelInterpContext, Box<InterpLocatableError>> {
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
                return Err(InterpLocatableError {
                    error: InterpError::Assertion(
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
                tpe: Type::Struct(st),
            } => (sv, st),
            t => {
                return Err(InterpLocatableError {
                    error: InterpError::Assertion(
                        InterpAssertion::TypeMismatch(ast.base.tipe(), t.tpe).into(),
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
            return Err(InterpLocatableError {
                error: InterpError::Assertion(
                    InterpAssertion::TypeMismatch(ast.base.tipe(), Type::Struct(base_type)).into(),
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
                return Err(InterpLocatableError {
                    error: InterpError::UnknownField(ast.member.identifier.clone()).into(),
                    location: ast.member.location.clone(),
                    context,
                })?;
            }
        };

        let member_value = match base_value.fields.get(&ast.member.identifier) {
            Some(v) => v.clone(),
            None => {
                return Err(InterpLocatableError {
                    error: InterpError::UnknownField(ast.member.identifier.clone()).into(),
                    location: ast.member.location.clone(),
                    context,
                }
                .into());
            }
        };

        if member_type != member_value.tpe {
            return Err(InterpLocatableError {
                error: InterpError::Assertion(
                    InterpAssertion::TypeMismatch(member_type.clone(), member_value.tpe.clone())
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
