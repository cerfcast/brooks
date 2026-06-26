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
    net::IpAddr,
    sync::Arc,
};

use crate::{
    analysis::{
        MelAnalysisAssertions::{ContextMissingExpr, ContextMissingParams, ContextWrongExprType},
        MelAnalysisError::{
            AssertionFailure, Incalculable, InvalidRegex, InvalidType, Miscount, Mismatch,
            OptimizationNotSupported, PreconditionFailure, RegexSame, UnknownField,
            UnknownIdentifier,
        },
    },
    ast::{
        self, Argument, ArgumentList, AstVisitor, AstVisitorDriver, AstVisitorResult, BinaryExpr,
        BinaryInfixOperator, BooleanLiteral,
        ComparisonOperator::{IP, Re},
        Expr, FunctionCall, IPAddressLiteral, Identifier, MemberAccessExpression, NumberLiteral,
        StringLiteral, TernaryExpr,
    },
    compiler::{CompilerError, MELCompilerContext, SyntaxError::EmptyContext, compile},
    expect_expr,
    grammar::GrammarLocation,
    scope::{self, Scopes},
    tvs::{
        self,
        Type::{self, Function, Struct},
    },
};

#[derive(Debug, Clone)]
pub enum CompiledConstant {
    Integer(i64),
    String(String),
    Boolean(bool),
    IPAddress(IpAddr),
}

#[derive(Debug, Clone)]
pub struct Analyzed {
    pub tipe: Type,
    pub constant: Option<CompiledConstant>,
}

impl Expr<Analyzed> {
    pub fn tipe(&self) -> Type {
        match self {
            Expr::FunctionCall(function_call) => function_call.aug.tipe.clone(),
            Expr::BinaryExpr(binary_expr) => binary_expr.aug.tipe.clone(),
            Expr::TernaryExpr(ternary_expr) => ternary_expr.aug.tipe.clone(),
            Expr::Identifier(identifier) => identifier.aug.tipe.clone(),
            Expr::ArgumentList(argument_list) => argument_list.aug.tipe.clone(),
            Expr::Argument(argument) => argument.aug.tipe.clone(),
            Expr::MemberAccess(member) => member.aug.tipe.clone(),
            Expr::Literal(_, _, aug) => aug.tipe.clone(),
        }
    }

    pub fn constant(&self) -> Option<CompiledConstant> {
        let analyzed = match self {
            Expr::FunctionCall(function_call) => &function_call.aug,
            Expr::BinaryExpr(binary_expr) => &binary_expr.aug,
            Expr::TernaryExpr(ternary_expr) => &ternary_expr.aug,
            Expr::Identifier(identifier) => &identifier.aug,
            Expr::ArgumentList(argument_list) => &argument_list.aug,
            Expr::Argument(argument) => &argument.aug,
            Expr::MemberAccess(member) => &member.aug,
            Expr::Literal(_, _, aug) => aug,
        };
        analyzed.constant.clone()
    }
}

impl<A: Clone + Debug> Display for Expr<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::FunctionCall(_) => write!(f, "FunctionCall"),
            Expr::BinaryExpr(_) => write!(f, "BinaryExpr"),
            Expr::TernaryExpr(_) => write!(f, "TernaryExpr"),
            Expr::Identifier(_) => write!(f, "Identifier"),
            Expr::ArgumentList(_) => write!(f, "ArgumentList"),
            Expr::Argument(_) => write!(f, "Argument"),
            Expr::MemberAccess(_) => write!(f, "MemberAccess"),
            Expr::Literal(_, _, _) => write!(f, "Literal"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MelAnalysisAssertions {
    ContextMissing,
    ContextMissingExpr(String),
    ContextWrongExprType(String, String, String),
    ContextMissingParams,
    InvalidOperator(String, String),
}

impl Display for MelAnalysisAssertions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MelAnalysisAssertions::ContextMissing => write!(f, "Missing analysis context"),
            ContextWrongExprType(expected, actual, whre) => write!(
                f,
                "Wrong expression type in analysis context; expected {} but found {} in {}",
                expected, actual, whre
            ),
            MelAnalysisAssertions::ContextMissingParams => {
                write!(f, "Missing parameters in analysis context")
            }
            MelAnalysisAssertions::ContextMissingExpr(s) => {
                write!(f, "Missing expression in {}", s)
            }
            MelAnalysisAssertions::InvalidOperator(o, l) => {
                write!(f, "Invalid operator {o} in {l}")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum MelAnalysisPreconditions {
    ContextMissingExpr(String),
}

impl Display for MelAnalysisPreconditions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MelAnalysisPreconditions::ContextMissingExpr(s) => {
                write!(f, "Missing expression in {}", s)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum MelAnalysisError {
    CompilerError(CompilerError),
    Mismatch(Type, Type),
    RegexSame,
    InvalidType(Vec<Type>, Type),
    Miscount(usize, usize),
    InvalidRegex(String),
    UnknownField(String, String),
    UnknownIdentifier(String),
    AssertionFailure(MelAnalysisAssertions),
    PreconditionFailure(MelAnalysisPreconditions),
    OptimizationNotSupported(String),
    Incalculable,
}

impl Display for MelAnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CompilerError(e) => write!(f, "Compiler error: {:?}", e),
            Mismatch(expected, found) => write!(f, "Expected {:?}, found {:?}", expected, found),
            Miscount(expected, found) => write!(f, "Expected {expected} arguments, found {found}"),
            UnknownIdentifier(i) => write!(f, "Unknown identifier {:?}", i),
            AssertionFailure(c) => write!(f, "Missing compiler context: {:?}", c),
            PreconditionFailure(c) => {
                write!(f, "Precondition not satisfied during analysis: {:?}", c)
            }
            UnknownField(strct, member) => write!(f, "No field named {member} in {strct}"),
            Incalculable => write!(f, "Incalculable expression analysis"),
            OptimizationNotSupported(o) => write!(f, "Optimization not supported: {o}"),
            InvalidRegex(i) => write!(f, "Regular expression literal not valid: {i}"),
            RegexSame => write!(
                f,
                "The operands to the regular expression match operator must be a string and a regular expression"
            ),
            InvalidType(valids, actual) => write!(
                f,
                "{} is not one of the expected types ({})",
                actual.to_string(),
                valids
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MelAnalysisLocatableError {
    pub error: MelAnalysisError,
    pub location: GrammarLocation,
}

impl Display for MelAnalysisLocatableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.error, self.location)
    }
}

#[derive(Debug, Clone, Default)]
pub struct MelAnalysisContext {
    pub expr: Option<Expr<Analyzed>>,
    pub params: Option<Vec<Type>>,
    pub scopes: scope::Scopes<Type>,
}

impl MelAnalysisContext {
    pub fn update_expr(&self, new: Expr<Analyzed>) -> Self {
        MelAnalysisContext {
            expr: Some(new),
            params: self.params.clone(),
            scopes: self.scopes.clone(),
        }
    }
    pub fn update_scopes(&self, new: scope::Scopes<Type>) -> Self {
        MelAnalysisContext {
            expr: self.expr.clone(),
            params: self.params.clone(),
            scopes: new,
        }
    }
    pub fn update_params(&self, new: Vec<Type>) -> Self {
        MelAnalysisContext {
            expr: self.expr.clone(),
            params: Some(new),
            scopes: self.scopes.clone(),
        }
    }
}

pub struct MelTypeChecker {}

impl AstVisitor<MelAnalysisContext, (), MelAnalysisLocatableError> for MelTypeChecker {
    fn visit_function_call(
        &self,
        ast: &FunctionCall<()>,
        context: MelAnalysisContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        let callee = driver.visit(&ast.callee, self, context.clone())?;

        let callee = if let Some(callee) = callee.expr {
            callee
        } else {
            return Err(MelAnalysisLocatableError {
                error: AssertionFailure(MelAnalysisAssertions::ContextMissingExpr(
                    "visit_function_call".to_string(),
                )),
                location: ast.location.clone(),
            });
        };

        let fn_params = match callee.tipe() {
            Type::Function(return_type, tvs::Params { args }) => (return_type, args),
            t => {
                return Err(MelAnalysisLocatableError {
                    error: Mismatch(
                        Function(Arc::new(Type::None), tvs::Params { args: vec![] }),
                        t,
                    ),
                    location: ast.location.clone(),
                });
            }
        };

        let context_with_params = context.update_params(fn_params.1.clone());
        let args = self.visit_argument_list(&ast.arguments, context_with_params, driver)?;
        let args = match args.expr.unwrap() {
            Expr::ArgumentList(argument_list) => Ok(argument_list),
            e => Err(AssertionFailure(ContextWrongExprType(
                "ArgumentList".to_string(),
                e.to_string(),
                "visit_function_call".to_string(),
            ))),
        }
        .map_err(|e| MelAnalysisLocatableError {
            error: e,
            location: ast.location.clone(),
        })?;

        Ok(
            context.update_expr(Expr::FunctionCall(Arc::new(FunctionCall {
                callee: callee.clone(),
                location: ast.location.clone(),
                arguments: (*args).clone(),
                aug: Analyzed {
                    tipe: (*fn_params.0).clone(),
                    constant: None,
                },
            }))),
        )
    }

    fn visit_identifier(
        &self,
        ast: &Identifier<()>,
        context: MelAnalysisContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        let found_id = context
            .scopes
            .lookup(&ast.identifier)
            .ok_or(MelAnalysisLocatableError {
                error: UnknownIdentifier(ast.identifier.clone()),
                location: ast.location.clone(),
            })?;

        Ok(context.update_expr(Expr::Identifier(Arc::new(Identifier {
            identifier: ast.identifier.clone(),
            location: ast.location.clone(),
            aug: Analyzed {
                tipe: found_id,
                constant: None,
            },
        }))))
    }

    fn visit_argument_list(
        &self,
        ast: &ArgumentList<()>,
        context: MelAnalysisContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        let params = context.params.as_ref().ok_or(MelAnalysisLocatableError {
            error: AssertionFailure(ContextMissingParams),
            location: ast.location.clone(),
        })?;

        if params.len() != ast.arguments.len() {
            return Err(MelAnalysisLocatableError {
                error: MelAnalysisError::Miscount(params.len(), ast.arguments.len()),
                location: ast.location.clone(),
            });
        }

        let mut arg_types: Vec<Argument<Analyzed>> = vec![];
        for arg in ast.arguments.iter().zip(params) {
            let arg = self
                .visit_argument(arg.0, context.update_params(vec![arg.1.clone()]), driver)?
                .expr
                .ok_or(MelAnalysisLocatableError {
                    error: MelAnalysisError::AssertionFailure(ContextMissingExpr(
                        "visit_argument_list".into(),
                    )),
                    location: arg.0.location.clone(),
                })?;
            let arg = match arg {
                Expr::Argument(argument) => Ok(argument),
                e => Err(AssertionFailure(ContextWrongExprType(
                    "Argument".to_string(),
                    e.to_string(),
                    "visit_argument_list".to_string(),
                ))),
            }
            .map_err(|e| MelAnalysisLocatableError {
                error: e,
                location: ast.location.clone(),
            })?;
            arg_types.push((*arg).clone());
        }

        Ok(
            context.update_expr(Expr::ArgumentList(Arc::new(ArgumentList {
                arguments: arg_types,
                location: ast.location.clone(),
                aug: Analyzed {
                    tipe: Type::None,
                    constant: None,
                },
            }))),
        )
    }

    fn visit_argument(
        &self,
        ast: &Argument<()>,
        context: MelAnalysisContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        let params = context.params.as_ref().ok_or(MelAnalysisLocatableError {
            error: AssertionFailure(ContextMissingParams),
            location: ast.location.clone(),
        })?;

        let arg = driver.visit(&ast.expr, self, context.clone())?.expr.ok_or(
            MelAnalysisLocatableError {
                error: MelAnalysisError::AssertionFailure(ContextMissingExpr(
                    "visit_argument".into(),
                )),
                location: ast.location.clone(),
            },
        )?;

        let arg_type = arg.tipe();
        if arg_type != params[0] {
            return Err(MelAnalysisLocatableError {
                error: Mismatch(params[0].clone(), arg_type),
                location: arg.location(),
            });
        }
        Ok(context.update_expr(Expr::Argument(Arc::new(Argument {
            expr: arg,
            location: ast.location.clone(),
            aug: Analyzed {
                tipe: arg_type,
                constant: None,
            },
        }))))
    }

    fn visit_binary_expr(
        &self,
        ast: &BinaryExpr<()>,
        context: MelAnalysisContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        let left = driver.visit(&ast.left, self, context.clone())?.expr.ok_or(
            MelAnalysisLocatableError {
                error: MelAnalysisError::AssertionFailure(ContextMissingExpr(
                    "visit_binary_expr".into(),
                )),
                location: ast.left.location(),
            },
        )?;
        let right = driver
            .visit(&ast.right, self, context.clone())?
            .expr
            .ok_or(MelAnalysisLocatableError {
                error: MelAnalysisError::AssertionFailure(ContextMissingExpr(
                    "visit_binary_expr".into(),
                )),
                location: ast.right.location(),
            })?;

        let left_type = left.tipe();
        let right_type = right.tipe();

        let (valid_types, result_type) = match &ast.op {
            BinaryInfixOperator::Logic(_) => (vec![Type::Boolean], Type::Boolean),
            BinaryInfixOperator::Comparison(Re) => (vec![Type::Regex, Type::String], Type::Boolean),
            BinaryInfixOperator::Comparison(IP) => (vec![Type::IPAddress], Type::Boolean),
            BinaryInfixOperator::Comparison(_) => (
                vec![Type::Boolean, Type::String, Type::Integer],
                Type::Boolean,
            ),
            BinaryInfixOperator::Math(_) => (vec![Type::Integer], Type::Integer),
            BinaryInfixOperator::Concat(_) => (vec![Type::String], Type::String),
            BinaryInfixOperator::MemberAccess(e) => {
                return Err(MelAnalysisLocatableError {
                    error: MelAnalysisError::AssertionFailure(
                        MelAnalysisAssertions::InvalidOperator(
                            e.to_string(),
                            "visit_binary_expr".to_string(),
                        ),
                    ),
                    location: ast.right.location(),
                });
            }
        };

        if !valid_types.contains(&left_type) {
            return Err(MelAnalysisLocatableError {
                error: InvalidType(valid_types, left_type),
                location: left.location(),
            });
        }
        if !valid_types.contains(&right_type) {
            return Err(MelAnalysisLocatableError {
                error: InvalidType(valid_types, right_type),
                location: right.location(),
            });
        }

        // If the operator is the regex operator, then there is special handling.
        if let BinaryInfixOperator::Comparison(Re) = ast.op {
            if left_type != Type::String {
                return Err(MelAnalysisLocatableError {
                    error: Mismatch(Type::String, left_type),
                    location: left.location(),
                });
            }
            if right_type != Type::Regex {
                return Err(MelAnalysisLocatableError {
                    error: Mismatch(Type::Regex, right_type),
                    location: right.location(),
                });
            }
            return Ok(context.update_expr(Expr::BinaryExpr(Arc::new(BinaryExpr {
                left,
                right,
                op: ast.op.clone(),
                location: ast.location.clone(),
                aug: Analyzed {
                    tipe: result_type,
                    constant: None,
                },
            }))));
        }

        // Otherwise, the types just need to be equal!
        if left_type != right_type {
            return Err(MelAnalysisLocatableError {
                error: Mismatch(left_type, right_type),
                location: right.location(),
            });
        }

        Ok(context.update_expr(Expr::BinaryExpr(Arc::new(BinaryExpr {
            left,
            right,
            op: ast.op.clone(),
            location: ast.location.clone(),
            aug: Analyzed {
                tipe: result_type,
                constant: None,
            },
        }))))
    }

    fn visit_literal(
        &self,
        ast: (&ast::Literal, &GrammarLocation, &()),
        context: MelAnalysisContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        match ast {
            (b @ ast::Literal::Boolean(_), location, _) => Ok(context.update_expr(Expr::Literal(
                b.clone(),
                location.clone(),
                Analyzed {
                    tipe: Type::Boolean,
                    constant: None,
                },
            ))),
            (n @ ast::Literal::Number(_), location, _) => Ok(context.update_expr(Expr::Literal(
                n.clone(),
                location.clone(),
                Analyzed {
                    tipe: Type::Integer,
                    constant: None,
                },
            ))),
            (s @ ast::Literal::String(_), location, _) => Ok(context.update_expr(Expr::Literal(
                s.clone(),
                location.clone(),
                Analyzed {
                    tipe: Type::String,
                    constant: None,
                },
            ))),
            (r @ ast::Literal::Regex(_), location, _) => Ok(context.update_expr(Expr::Literal(
                r.clone(),
                location.clone(),
                Analyzed {
                    tipe: Type::Regex,
                    constant: None,
                },
            ))),
            (ip @ ast::Literal::IPAddress(_), location, _) => {
                Ok(context.update_expr(Expr::Literal(
                    ip.clone(),
                    location.clone(),
                    Analyzed {
                        tipe: Type::IPAddress,
                        constant: None,
                    },
                )))
            }
        }
    }

    fn visit_ternary_expr(
        &self,
        ast: &TernaryExpr<()>,
        context: MelAnalysisContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        let condition = driver
            .visit(&ast.condition, self, context.clone())?
            .expr
            .ok_or(MelAnalysisLocatableError {
                error: MelAnalysisError::AssertionFailure(ContextMissingExpr(
                    "visit_ternary_expr".into(),
                )),
                location: ast.condition.location(),
            })?;
        let yes = driver.visit(&ast.yes, self, context.clone())?.expr.ok_or(
            MelAnalysisLocatableError {
                error: MelAnalysisError::AssertionFailure(ContextMissingExpr(
                    "visit_ternary_expr".into(),
                )),
                location: ast.yes.location(),
            },
        )?;

        let no = driver.visit(&ast.no, self, context.clone())?.expr.ok_or(
            MelAnalysisLocatableError {
                error: MelAnalysisError::AssertionFailure(ContextMissingExpr(
                    "visit_ternary_expr".into(),
                )),
                location: ast.no.location(),
            },
        )?;

        let condition_type = condition.tipe();
        let yes_type = yes.tipe();
        let no_type = no.tipe();

        if condition_type != Type::Boolean {
            return Err(MelAnalysisLocatableError {
                error: Mismatch(Type::Boolean, condition_type),
                location: ast.location.clone(),
            });
        }

        if yes_type != no_type {
            return Err(MelAnalysisLocatableError {
                error: Mismatch(yes_type, no_type),
                location: ast.location.clone(),
            });
        }

        Ok(context.update_expr(Expr::TernaryExpr(Arc::new(TernaryExpr {
            condition,
            yes,
            no,
            location: ast.location.clone(),
            aug: Analyzed {
                tipe: yes_type,
                constant: None,
            },
        }))))
    }

    fn visit_member_access_expr(
        &self,
        ast: &ast::MemberAccessExpression<()>,
        context: MelAnalysisContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        let base = driver.visit(&ast.base, self, context.clone())?.expr.ok_or(
            MelAnalysisLocatableError {
                error: MelAnalysisError::AssertionFailure(ContextMissingExpr(
                    "visit_member_access_expr".into(),
                )),
                location: ast.base.location(),
            },
        )?;

        let struct_type = match base.tipe() {
            Type::Struct(struct_type) => struct_type,
            t => {
                return Err(MelAnalysisLocatableError {
                    error: MelAnalysisError::Mismatch(
                        Struct(tvs::Struct {
                            name: "TODO".to_string(),
                            ..Default::default()
                        }),
                        t,
                    ),
                    location: ast.base.location(),
                });
            }
        };

        let member_type = struct_type.type_for_field(&ast.member.identifier).ok_or(
            MelAnalysisLocatableError {
                error: MelAnalysisError::UnknownField(
                    struct_type.name,
                    ast.member.identifier.clone(),
                ),
                location: ast.base.location(),
            },
        )?;

        Ok(
            context.update_expr(Expr::MemberAccess(Arc::new(MemberAccessExpression {
                base,
                oper: ast.oper.clone(),
                member: Identifier {
                    identifier: ast.member.identifier.clone(),
                    aug: Analyzed {
                        tipe: tvs::Type::None,
                        constant: None,
                    },
                    location: ast.location.clone(),
                },
                location: ast.location.clone(),
                aug: Analyzed {
                    tipe: member_type,
                    constant: None,
                },
            }))),
        )
    }
}

#[cfg(test)]
mod type_check_tests {
    use crate::{
        analysis::{
            Analyzed, MelAnalysisContext, MelAnalysisError, MelAnalysisLocatableError,
            MelTypeChecker,
        },
        ast::{AstVisitorDriver, BinaryExpr, Expr, FunctionCall, Identifier},
        compiler::{CompilerError, MELCompilerContext, SyntaxError::EmptyContext, compile},
        expect_expr,
        grammar::GrammarLocation,
        tvs::{
            self,
            Type::{self, Boolean, Function, IPAddress, Integer},
        },
    };
    use std::{assert_matches, sync::Arc};

    #[test]
    fn test_type_check_literal() {
        let expr = "5";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let aug = match result {
            Expr::Literal(_, _, c) => c,
            _ => todo!(),
        };

        assert_matches!(
            aug,
            Analyzed {
                tipe: Type::Integer,
                constant: None,
            }
        )
    }

    #[test]
    fn test_type_check_invalid_regex_expr() {
        let expr = "\"testing\" ~= /[tsting/";

        let compile_result = compile(expr);
        let _ =
            compile_result.expect_err("Could compile expression with invalid regular expression.");
    }

    #[test]
    fn test_type_check_binary_regex_expr() {
        let expr = "\"testing\" ~= /testing/";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(binary_expr) => binary_expr,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Boolean,
                    constant: _
                }
            }
        )
    }

    #[test]
    fn test_type_check_binary_regex_expr_error() {
        let expr = "5 ~= /testing/";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect_err("Could analyze expression with type error");

        assert_matches!(
            result,
            MelAnalysisLocatableError {
                error: MelAnalysisError::InvalidType(i, Integer),
                location: GrammarLocation {
                    start: 0,
                    extent: 1
                }
            } if i == vec![Type::Regex, Type::String]
        )
    }

    #[test]
    fn test_type_check_binary_regex_expr_error_reverse() {
        let expr = "\"/testing/\" ~= false";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect_err("Could analyze expression with type error");

        assert_matches!(
            result,
            MelAnalysisLocatableError {
                error: MelAnalysisError::InvalidType(i, Boolean),
                location: GrammarLocation {
                    start: 15,
                    extent: 5
                }
            } if i == vec![Type::Regex, Type::String]
        )
    }

    #[test]
    fn test_type_check_binary_regex_expr_error_two_res() {
        let expr = "/testing/ ~= /[\\W]/";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect_err("Could analyze expression with type error");

        assert_matches!(
            result,
            MelAnalysisLocatableError {
                error: MelAnalysisError::Mismatch(Type::String, Type::Regex),
                location: GrammarLocation {
                    start: 0,
                    extent: 9
                }
            }
        )
    }

    #[test]
    fn test_type_check_binary_regex_expr_error_two_strings() {
        let expr = "\"testing\" ~= \"hello\"";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect_err("Could analyze expression with type error");

        assert_matches!(
            result,
            MelAnalysisLocatableError {
                error: MelAnalysisError::Mismatch(Type::Regex, Type::String),
                location: GrammarLocation {
                    start: 13,
                    extent: 7
                }
            }
        )
    }

    #[test]
    fn test_type_check_binary_expr() {
        let expr = "5 + 3";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(binary_expr) => binary_expr,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Integer,
                    constant: _
                }
            }
        )
    }

    #[test]
    fn test_type_check_binary_expr_nested() {
        let expr = "5 + (3 * 8)";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(binary_expr) => binary_expr,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Integer,
                    constant: _
                }
            }
        );
    }

    #[test]
    fn test_type_check_binary_expr_error() {
        let expr = "5 + \"testing\"";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver.visit(&ast, &visitor, context);

        assert_matches!(
            result,
            Err(MelAnalysisLocatableError {
                error: MelAnalysisError::InvalidType(x, tvs::Type::String),
                location: GrammarLocation {
                    start: 4,
                    extent: 9
                }
            }) if x == vec![Integer]
        );
    }

    #[test]
    fn test_type_check_identifier_missing() {
        let expr = "a";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect_err("Could analyze expression with missing identifier");

        assert_matches!(result, MelAnalysisLocatableError {
            error: MelAnalysisError::UnknownIdentifier(a),
            location: GrammarLocation { start: 0, extent: 1},
        } if a == "a");
    }

    #[test]
    fn test_type_check_identifier_in_expression() {
        let expr = "5 + a";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert("a", Integer));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze expression with missing identifier");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(binary_expr) => binary_expr,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Integer,
                    constant: _
                }
            }
        );
    }

    #[test]
    fn test_type_check_bool_to_math_operator() {
        let expr = "false + true";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert("a", Integer));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect_err("Could analyze expression with type error");

        assert_matches!(result, MelAnalysisLocatableError {
            error: MelAnalysisError::InvalidType(a, Boolean),
            location: GrammarLocation { start: 0, extent: 5},
        } if a == vec![Integer]);
    }

    #[test]
    fn test_type_check_bool_to_math_operator2() {
        let expr = "5 + true";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert("a", Integer));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect_err("Could analyze expression with type error");

        assert_matches!(result, MelAnalysisLocatableError {
            error: MelAnalysisError::InvalidType(a, Boolean),
            location: GrammarLocation { start: 4, extent: 4},
        } if a == vec![Integer]);
    }

    #[test]
    fn test_type_check_identifier() {
        let expr = "a";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert("a", Integer));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze expression with missing identifier");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::Identifier(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            Identifier::<Analyzed> {
                identifier: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Integer,
                    constant: _
                }
            }
        );
    }

    #[test]
    fn test_type_check_function_call() {
        let expr = "testing(5)";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert(
            "testing",
            Function(
                Arc::new(Type::Integer),
                tvs::Params {
                    args: vec![Type::Integer],
                },
            ),
        ));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze expression with missing identifier");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::FunctionCall(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            FunctionCall::<Analyzed> {
                callee: _,
                location: _,
                arguments: _,
                aug: Analyzed {
                    tipe: Integer,
                    constant: _
                }
            }
        );
    }

    #[test]
    fn test_type_check_function_call_error() {
        let expr = "use_me(5)";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert(
            "use",
            Function(
                Arc::new(Type::Integer),
                tvs::Params {
                    args: vec![Type::Integer],
                },
            ),
        ));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect_err("Could analyze expression with missing identifier");
        assert_matches!(result, MelAnalysisLocatableError {
            error: MelAnalysisError::UnknownIdentifier(a),
            location: _,
        } if a == "use_me");
    }

    #[test]
    fn test_type_check_function_call_error_wrong_argument_count() {
        let expr = "use(5)";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert(
            "use",
            Function(
                Arc::new(Type::Integer),
                tvs::Params {
                    args: vec![Type::Integer, Type::String],
                },
            ),
        ));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect_err("Could analyze expression with miscounted params/args");
        assert_matches!(
            result,
            MelAnalysisLocatableError {
                error: MelAnalysisError::Miscount(2, 1),
                location: _,
            }
        )
    }

    #[test]
    fn test_type_check_function_call_in_expression() {
        let expr = "use_me(5)";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert(
            "use_me",
            Function(
                Arc::new(Type::Integer),
                tvs::Params {
                    args: vec![Type::Integer],
                },
            ),
        ));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze expression");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::FunctionCall(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            FunctionCall::<Analyzed> {
                callee: _,
                arguments: _,
                location: _,
                aug: Analyzed {
                    tipe: Integer,
                    constant: _
                }
            }
        );
    }

    #[test]
    fn test_type_check_function_call_in_expression2() {
        let expr = "use_me(5, \"testing\")";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert(
            "use_me",
            Function(
                Arc::new(Type::Integer),
                tvs::Params {
                    args: vec![Type::Integer, Type::String],
                },
            ),
        ));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze expression");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::FunctionCall(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            FunctionCall::<Analyzed> {
                callee: _,
                arguments: _,
                location: _,
                aug: Analyzed {
                    tipe: Integer,
                    constant: _
                }
            }
        );
    }

    #[test]
    fn test_type_check_function_call_in_expression3() {
        let expr = "use_me(5, \"testing\")";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert(
            "use_me",
            Function(
                Arc::new(Type::Integer),
                tvs::Params {
                    args: vec![Type::Integer, Type::Integer],
                },
            ),
        ));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect_err("Could analyze expression with type error");

        assert_matches!(
            result,
            MelAnalysisLocatableError {
                error: MelAnalysisError::Mismatch(Integer, Type::String),
                location: GrammarLocation {
                    start: 10,
                    extent: 9
                }
            }
        );
    }

    #[test]
    fn test_type_check_function_call_in_binary_expression() {
        let expr = "use_me(5) + 10";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert(
            "use_me",
            Function(
                Arc::new(Type::Integer),
                tvs::Params {
                    args: vec![Type::Integer],
                },
            ),
        ));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze expression");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(bin) => bin,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,

                aug: Analyzed {
                    tipe: Integer,
                    constant: _
                }
            }
        );
    }

    #[test]
    fn test_type_check_function_call_in_expression_mismatch() {
        let expr = "5 + (use_me(5) + 10)";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert(
            "use_me",
            Function(
                Arc::new(Type::Boolean),
                tvs::Params {
                    args: vec![Type::Integer],
                },
            ),
        ));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect_err("Could analyze expression with type error");
        assert_matches!(
            result,
            MelAnalysisLocatableError {
                error: MelAnalysisError::InvalidType(i, Boolean),
                location: GrammarLocation {
                    start: 5,
                    extent: 9
                }
            } if i == vec![Integer]
        );
    }

    #[test]
    fn test_type_check_comparison_expr() {
        let expr = "5 < 4";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Boolean,
                    constant: None,
                }
            }
        );
    }

    #[test]
    fn test_type_check_comparison_expr_mismatch() {
        let expr = "(5 < 4) + 5";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();

        let result = driver
            .visit(&ast, &visitor, context)
            .expect_err("Could analyze expression with type error");
        assert_matches!(
            result,
            MelAnalysisLocatableError {
                error: MelAnalysisError::InvalidType(i, Boolean),
                location: GrammarLocation {
                    start: 1,
                    extent: 5
                }
            } if i == vec![Integer]
        );
    }

    #[test]
    fn test_type_check_comparison_expr_ipliteral() {
        let expr = "8.8.8.8 ipmatch 10.1.9.2";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Boolean,
                    constant: None,
                }
            }
        );
    }

    #[test]
    fn test_type_check_comparison_expr_ipliteral2() {
        let expr = "2001:0db8:85a3:0000:0000:8a2e:0370:7334 ipmatch 2001:0db8:85a3:0000:0000:8a2e:0370:7334";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Boolean,
                    constant: None,
                }
            }
        );
    }

    #[test]
    fn test_type_check_comparison_expr_ipliteral3() {
        let expr = "2001:0db8:85a3:0000:0000:8a2e:0370:7334 ipmatch 8.8.8.8";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Boolean,
                    constant: None,
                }
            }
        );
    }

    #[test]
    fn test_type_check_comparison_expr_ipliteral4() {
        let expr = "2001:0db8:85a3:0000:0000:8a2e:0370:7334 ipmatch a";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert("a", IPAddress));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Boolean,
                    constant: None,
                }
            }
        );
    }

    #[test]
    fn test_type_check_comparison_expr_ip() {
        let expr = "a ipmatch b";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert("a", IPAddress));

        context = context.update_scopes(context.scopes.insert("b", IPAddress));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Boolean,
                    constant: None,
                }
            }
        );
    }

    #[test]
    fn test_type_check_comparison_expr_ipliteral_mismatch() {
        let expr = "2001:0db8:85a3:0000:0000:8a2e:0370:7334 ipmatch 8";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();

        let result = driver
            .visit(&ast, &visitor, context)
            .expect_err("Could analyze expression with type error");
        assert_matches!(
            result,
            MelAnalysisLocatableError {
                error: MelAnalysisError::InvalidType(i, Integer),
                location: GrammarLocation {
                    start: 48,
                    extent: 1
                }
            } if i == vec![IPAddress]
        );
    }

    #[test]
    fn test_type_check_comparison_expr_ipliteral_mismatch2() {
        let expr = "2001:0db8:85a3:0000:0000:8a2e:0370:7334 ipmatch a";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert("a", Type::String));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect_err("Could analyze expression with type error");
        assert_matches!(
            result,
            MelAnalysisLocatableError {
                error: MelAnalysisError::InvalidType(i, Type::String),
                location: GrammarLocation {
                    start: 48,
                    extent: 1
                }
            } if i == vec![IPAddress]
        );
    }

    #[test]
    fn test_type_check_comparison_expr_ip_mismatch() {
        let expr = "a ipmatch b";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert("a", IPAddress));

        context = context.update_scopes(context.scopes.insert("b", Type::String));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect_err("Could analyze expression with type error");
        assert_matches!(
            result,
            MelAnalysisLocatableError {
                error: MelAnalysisError::InvalidType(i, Type::String),
                location: GrammarLocation {
                    start: 10,
                    extent: 1
                }
            } if i == vec![IPAddress]
        );
    }
}

#[derive(Debug, Clone)]
pub enum MelOptimizerError {}

impl Display for MelOptimizerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Optimizer error")
    }
}

#[derive(Debug, Clone)]
pub struct MelOptimizerLocatableError {
    pub error: MelOptimizerError,
    pub location: GrammarLocation,
}

impl Display for MelOptimizerLocatableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.error, self.location)
    }
}

pub struct ConstEvaluator {}

impl ConstEvaluator {
    pub fn const_evaluatable(op: &ast::BinaryInfixOperator) -> bool {
        matches!(op, ast::BinaryInfixOperator::Comparison(IP))
            || !matches!(op, ast::BinaryInfixOperator::Comparison(_))
    }

    #[allow(clippy::result_large_err)]
    fn evaluate_binary_expr(
        left: &CompiledConstant,
        left_type: Type,
        right: &CompiledConstant,
        right_type: Type,
        op: ast::BinaryInfixOperator,
    ) -> Result<CompiledConstant, MelAnalysisError> {
        match op {
            ast::BinaryInfixOperator::Comparison(IP) => {
                let left = if let CompiledConstant::IPAddress(l) = left {
                    l
                } else {
                    return Err(MelAnalysisError::Mismatch(Type::IPAddress, left_type));
                };
                let right = if let CompiledConstant::IPAddress(r) = right {
                    r
                } else {
                    return Err(MelAnalysisError::Mismatch(Type::IPAddress, right_type));
                };
                Ok(CompiledConstant::Boolean(left == right))
            }
            ast::BinaryInfixOperator::Comparison(_) => Err(
                MelAnalysisError::OptimizationNotSupported("Comparison operator".into()),
            ),
            ast::BinaryInfixOperator::Logic(logic_operator) => {
                let left = if let CompiledConstant::Boolean(l) = left {
                    l
                } else {
                    return Err(MelAnalysisError::Mismatch(Type::Boolean, left_type));
                };
                let right = if let CompiledConstant::Boolean(r) = right {
                    r
                } else {
                    return Err(MelAnalysisError::Mismatch(Type::Boolean, right_type));
                };
                match logic_operator {
                    ast::LogicOperator::And => Ok(CompiledConstant::Boolean(*left && *right)),
                    ast::LogicOperator::Or => Ok(CompiledConstant::Boolean(*left || *right)),
                }
            }
            ast::BinaryInfixOperator::Math(math_operator) => {
                let left = if let CompiledConstant::Integer(l) = left {
                    l
                } else {
                    return Err(MelAnalysisError::Mismatch(Type::Integer, left_type));
                };
                let right = if let CompiledConstant::Integer(r) = right {
                    r
                } else {
                    return Err(MelAnalysisError::Mismatch(Type::Integer, right_type));
                };
                match math_operator {
                    ast::MathOperator::Plus => Ok(CompiledConstant::Integer(left + right)),
                    ast::MathOperator::Minus => Ok(CompiledConstant::Integer(left - right)),
                    ast::MathOperator::Multiply => Ok(CompiledConstant::Integer(left * right)),
                    ast::MathOperator::Divide => Ok(CompiledConstant::Integer(left / right)),
                    ast::MathOperator::Modulo => Ok(CompiledConstant::Integer(left % right)),
                }
            }
            ast::BinaryInfixOperator::Concat(_) => {
                let left = if let CompiledConstant::String(l) = left {
                    l
                } else {
                    return Err(MelAnalysisError::Mismatch(Type::String, left_type));
                };
                let right = if let CompiledConstant::String(r) = right {
                    r
                } else {
                    return Err(MelAnalysisError::Mismatch(Type::String, right_type));
                };
                Ok(CompiledConstant::String(left.to_owned() + right))
            }
            ast::BinaryInfixOperator::MemberAccess(c) => Err(MelAnalysisError::AssertionFailure(
                MelAnalysisAssertions::InvalidOperator(
                    c.to_string(),
                    "evaluate_binary_expr".to_string(),
                ),
            )),
        }
    }
}
pub struct MelOptimizer {}

impl AstVisitor<MelAnalysisContext, (), MelAnalysisLocatableError> for MelOptimizer {
    fn visit_function_call(
        &self,
        _ast: &FunctionCall<()>,
        context: MelAnalysisContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        Ok(context.clone())
    }

    fn visit_identifier(
        &self,
        _ast: &Identifier<()>,
        context: MelAnalysisContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        Ok(context.clone())
    }

    fn visit_argument_list(
        &self,
        _ast: &ArgumentList<()>,
        context: MelAnalysisContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        Ok(context.clone())
    }

    fn visit_argument(
        &self,
        _: &Argument<()>,
        context: MelAnalysisContext,
        _: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        Ok(context.clone())
    }

    fn visit_binary_expr(
        &self,
        ast: &BinaryExpr<()>,
        context: MelAnalysisContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        let expr = context.expr.as_ref().ok_or(MelAnalysisLocatableError {
            error: MelAnalysisError::PreconditionFailure(
                MelAnalysisPreconditions::ContextMissingExpr("visit_binary_expr".into()),
            ),
            location: ast.location.clone(),
        })?;

        let analyzed_binary_expr = match expr {
            Expr::BinaryExpr(binary_expr) => (**binary_expr).clone(),
            _ => todo!(),
        };

        let left = driver
            .visit(
                &ast.left,
                self,
                context.update_expr(analyzed_binary_expr.left),
            )?
            .expr
            .ok_or(MelAnalysisLocatableError {
                error: MelAnalysisError::AssertionFailure(ContextMissingExpr(
                    "visit_binary_expr".into(),
                )),
                location: ast.location.clone(),
            })?;

        let right = driver
            .visit(
                &ast.right,
                self,
                context.update_expr(analyzed_binary_expr.right),
            )?
            .expr
            .ok_or(MelAnalysisLocatableError {
                error: MelAnalysisError::AssertionFailure(ContextMissingExpr(
                    "visit_binary_expr".into(),
                )),
                location: ast.location.clone(),
            })?;

        let constant = match (left.constant(), right.constant()) {
            (Some(cl), Some(cr)) => {
                if ConstEvaluator::const_evaluatable(&ast.op) {
                    Some(
                        ConstEvaluator::evaluate_binary_expr(
                            &cl,
                            left.tipe(),
                            &cr,
                            right.tipe(),
                            ast.op.clone(),
                        )
                        .map_err(|e| MelAnalysisLocatableError {
                            error: e,
                            location: ast.location.clone(),
                        })?,
                    )
                } else {
                    None
                }
            }
            _ => None,
        };

        Ok(
            context.update_expr(Expr::BinaryExpr(Arc::new(ast::BinaryExpr {
                left,
                right,
                op: ast.op.clone(),
                location: ast.location.clone(),
                aug: Analyzed {
                    tipe: expr.tipe(),
                    constant,
                },
            }))),
        )
    }

    fn visit_literal(
        &self,
        _ast: (&ast::Literal, &GrammarLocation, &()),
        context: MelAnalysisContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        match _ast {
            (b @ ast::Literal::Boolean(v), location, _) => Ok(context.update_expr(Expr::Literal(
                b.clone(),
                location.clone(),
                Analyzed {
                    tipe: Type::Boolean,
                    constant: Some(CompiledConstant::Boolean(*v == BooleanLiteral::True)),
                },
            ))),
            (n @ ast::Literal::Number(NumberLiteral { literal: v }), location, _) => Ok(context
                .update_expr(Expr::Literal(
                    n.clone(),
                    location.clone(),
                    Analyzed {
                        tipe: Type::Integer,
                        constant: Some(CompiledConstant::Integer(*v as i64)),
                    },
                ))),
            (s @ ast::Literal::String(StringLiteral { literal: v }), location, _) => Ok(context
                .update_expr(Expr::Literal(
                    s.clone(),
                    location.clone(),
                    Analyzed {
                        tipe: Type::String,
                        constant: Some(CompiledConstant::String(v.clone())),
                    },
                ))),
            (r @ ast::Literal::Regex(_), location, _) => Ok(context.update_expr(Expr::Literal(
                r.clone(),
                location.clone(),
                Analyzed {
                    tipe: Type::Regex,
                    constant: None,
                },
            ))),
            (ip @ ast::Literal::IPAddress(IPAddressLiteral { literal: ipl }), location, _) => {
                Ok(context.update_expr(Expr::Literal(
                    ip.clone(),
                    location.clone(),
                    Analyzed {
                        tipe: Type::IPAddress,
                        constant: Some(CompiledConstant::IPAddress(*ipl)),
                    },
                )))
            }
        }
    }

    fn visit_ternary_expr(
        &self,
        ast: &ast::TernaryExpr<()>,
        context: MelAnalysisContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        let expr = context.expr.as_ref().ok_or(MelAnalysisLocatableError {
            error: MelAnalysisError::PreconditionFailure(
                MelAnalysisPreconditions::ContextMissingExpr("visit_ternary_expr".into()),
            ),
            location: ast.location.clone(),
        })?;

        let analyzed_binary_expr = match expr {
            Expr::TernaryExpr(ternary_expr) => (**ternary_expr).clone(),
            _ => todo!(),
        };

        let condition = driver
            .visit(
                &ast.condition,
                self,
                context.update_expr(analyzed_binary_expr.condition),
            )?
            .expr
            .ok_or(MelAnalysisLocatableError {
                error: MelAnalysisError::AssertionFailure(ContextMissingExpr(
                    "visit_ternary_expr".into(),
                )),
                location: ast.location.clone(),
            })?;
        let yes = driver
            .visit(
                &ast.yes,
                self,
                context.update_expr(analyzed_binary_expr.yes),
            )?
            .expr
            .ok_or(MelAnalysisLocatableError {
                error: MelAnalysisError::AssertionFailure(ContextMissingExpr(
                    "visit_ternary_expr".into(),
                )),
                location: ast.location.clone(),
            })?;

        let no = driver
            .visit(&ast.no, self, context.update_expr(analyzed_binary_expr.no))?
            .expr
            .ok_or(MelAnalysisLocatableError {
                error: MelAnalysisError::AssertionFailure(ContextMissingExpr(
                    "visit_ternary_expr".into(),
                )),
                location: ast.location.clone(),
            })?;

        let condition_c = match condition.constant() {
            Some(condition_c) => condition_c,
            None => {
                return Ok(
                    context.update_expr(Expr::TernaryExpr(Arc::new(ast::TernaryExpr {
                        condition,
                        yes,
                        no,
                        location: ast.location.clone(),
                        aug: Analyzed {
                            tipe: expr.tipe(),
                            constant: None,
                        },
                    }))),
                );
            }
        };

        let condition_c = match condition_c {
            CompiledConstant::Boolean(b) => b,
            e => {
                return Err(MelAnalysisLocatableError {
                    error: MelAnalysisError::AssertionFailure(
                        MelAnalysisAssertions::ContextWrongExprType(
                            "Boolean".to_string(),
                            e.to_string(),
                            "visit_ternary_expr".to_string(),
                        ),
                    ),
                    location: ast.location.clone(),
                });
            }
        };

        let constant_result = if condition_c { &yes } else { &no }.constant();

        Ok(
            context.update_expr(Expr::TernaryExpr(Arc::new(ast::TernaryExpr {
                condition,
                yes,
                no,
                location: ast.location.clone(),
                aug: Analyzed {
                    tipe: expr.tipe(),
                    constant: constant_result,
                },
            }))),
        )
    }

    fn visit_member_access_expr(
        &self,
        _ast: &ast::MemberAccessExpression<()>,
        context: MelAnalysisContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        Ok(context.clone())
    }
}

#[cfg(test)]
mod optimizer_tests {
    use crate::{
        analysis::{
            Analyzed, CompiledConstant, MelAnalysisContext, MelOptimizer, MelTypeChecker, ast::Expr,
        },
        ast::{AstVisitorDriver, BinaryExpr},
        compiler::{CompilerError, MELCompilerContext, SyntaxError::EmptyContext, compile},
        expect_expr,
        tvs::Type,
    };
    use std::assert_matches;

    #[test]
    fn test_optimize_binary_expr() {
        let expr = "5 + 3";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Integer,
                    constant: Some(CompiledConstant::Integer(8)),
                }
            }
        );
    }

    #[test]
    fn test_optimize_nested_binary_expr() {
        let expr = "5 + 3 + (8 * 11)";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Integer,
                    constant: Some(CompiledConstant::Integer(96)),
                }
            }
        );
    }

    #[test]
    fn test_optimize_nested_binary_expr2() {
        let expr = "(8 * 11) / 8";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Integer,
                    constant: Some(CompiledConstant::Integer(11)),
                }
            }
        );
    }

    #[test]
    fn test_optimize_nested_binary_expr3() {
        let expr = "(8 * 11) % 2";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Integer,
                    constant: Some(CompiledConstant::Integer(0)),
                }
            }
        );
    }

    #[test]
    fn test_optimize_nested_binary_expr4() {
        let expr = "\"testing\" . \"one\"";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            &*binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::String,
                    constant: Some(CompiledConstant::String(a)),
                }
            } if a == "testingone"
        );
    }

    #[test]
    fn test_optimize_nested_binary_expr5() {
        let expr = "(\"testing\" . \"one\") . \"two\"";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            &*binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::String,
                    constant: Some(CompiledConstant::String(a)),
                }
            } if a == "testingonetwo"
        );
    }

    #[test]
    fn test_optimize_logical_binary_expr() {
        let expr = "true or false";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Boolean,
                    constant: Some(CompiledConstant::Boolean(true)),
                }
            }
        );
    }

    #[test]
    fn test_optimize_logical_binary_expr2() {
        let expr = "true and false";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Boolean,
                    constant: Some(CompiledConstant::Boolean(false)),
                }
            }
        );
    }

    #[test]
    fn test_optimize_comparison_expr() {
        let expr = "5 < 4";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Boolean,
                    constant: None,
                }
            }
        );
    }

    #[test]
    fn test_optimize_ipmatch_comparison_expr() {
        let expr = "8.8.8.8 ipmatch 5.5.5.5";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Boolean,
                    constant: Some(CompiledConstant::Boolean(false)),
                }
            }
        );
    }

    #[test]
    fn test_optimize_ipmatch_comparison_expr2() {
        let expr = "5.5.5.5 ipmatch 5.5.5.5";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Boolean,
                    constant: Some(CompiledConstant::Boolean(true)),
                }
            }
        );
    }

    #[test]
    fn test_optimize_ipmatch_comparison_expr3() {
        let expr = "2001:0db8:85a3:0000:0000:8a2e:0370:7334 ipmatch 5.5.5.5";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Boolean,
                    constant: Some(CompiledConstant::Boolean(false)),
                }
            }
        );
    }

    #[test]
    fn test_optimize_ipmatch_comparison_expr4() {
        let expr = "2001:0db8:85a3:0000:0000:8a2e:0370:7334 ipmatch 2001:0db8:85a3:0000:0000:8a2e:0370:7334";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");
        let binary_expr = match result {
            Expr::BinaryExpr(id) => id,
            _ => todo!(),
        };

        assert_matches!(
            *binary_expr,
            BinaryExpr::<Analyzed> {
                left: _,
                right: _,
                op: _,
                location: _,
                aug: Analyzed {
                    tipe: Type::Boolean,
                    constant: Some(CompiledConstant::Boolean(true)),
                }
            }
        );
    }
}

#[cfg(test)]
mod analysis_error_tests {
    use crate::{
        analysis::{
            MelAnalysisContext, MelAnalysisError, MelAnalysisLocatableError,
            MelAnalysisPreconditions, MelOptimizer,
        },
        ast::AstVisitorDriver,
        compiler::{CompilerError, MELCompilerContext, SyntaxError::EmptyContext, compile},
        expect_expr,
    };
    use std::assert_matches;

    #[test]
    fn test_invalid_context_error_binary_expr() {
        let expr = "5 < 4";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(MELCompilerContext, compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let context = MelAnalysisContext::default();
        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};

        assert_matches!(
            driver.visit(&ast, &visitor, context),
            Err(MelAnalysisLocatableError {
                error: MelAnalysisError::PreconditionFailure(
                    MelAnalysisPreconditions::ContextMissingExpr(_)
                ),
                location: _
            })
        );
    }
}

pub type MelAnalysisResult = Result<Expr<Analyzed>, MelAnalysisLocatableError>;
#[allow(clippy::result_large_err)]
pub fn compile_and_analyze(source: &str, scopes: Scopes<Type>) -> MelAnalysisResult {
    let compile_result = compile(source);
    let compiled = compile_result.map_err(|e| MelAnalysisLocatableError {
        error: MelAnalysisError::CompilerError(e),
        location: GrammarLocation {
            start: 0,
            extent: source.len(),
        },
    })?;

    let ast = expect_expr!(MELCompilerContext, compiled).ok_or(MelAnalysisLocatableError {
        error: MelAnalysisError::CompilerError(CompilerError::SyntaxError(EmptyContext)),
        location: GrammarLocation {
            start: 0,
            extent: source.len(),
        },
    })?;

    let driver = AstVisitorDriver {};
    let visitor = MelTypeChecker {};
    let mut context = MelAnalysisContext::default();

    context = context.update_scopes(scopes);
    let result = driver.visit(&ast, &visitor, context)?;

    let driver = AstVisitorDriver {};
    let visitor = MelOptimizer {};
    let result = driver.visit(&ast, &visitor, result)?;

    let result = result.expr.ok_or(MelAnalysisLocatableError {
        error: MelAnalysisError::Incalculable,
        location: GrammarLocation {
            start: 0,
            extent: source.len(),
        },
    })?;

    Ok(result)
}
