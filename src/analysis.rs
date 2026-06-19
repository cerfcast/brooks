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

use std::{collections::HashMap, fmt::Display, sync::Arc};

use crate::{
    analysis::MelAnalysisError::{
        Incalculable, Mismatch, MissingContext, OptimizationNotSupported, UnknownIdentifier,
    },
    ast::{
        self, Argument, ArgumentList, AstVisitor, AstVisitorDriver, AstVisitorResult, BinaryExpr,
        BinaryInfixOperator, BooleanLiteral, Expr, FunctionCall, Identifier, NumberLiteral,
        StringLiteral,
        Type::{self, Function},
    },
    grammar::GrammarLocation,
};

#[derive(Debug, Clone, Default)]
pub struct Scope<I: Clone + Default> {
    pub items: HashMap<String, I>,
}

impl<I: Clone + Default> Scope<I> {
    pub fn lookup(&self, id: &str) -> Option<I> {
        self.items.get(id).cloned()
    }
    pub fn insert(&self, id: &str, value: I) -> Self {
        let mut next = self.items.clone();
        next.insert(id.to_string(), value);
        Self { items: next }
    }
}

#[derive(Debug, Clone)]
pub struct Scopes<I: Clone + Default> {
    pub scopes: Vec<Scope<I>>,
}

impl<I: Clone + Default> Scopes<I> {
    pub fn lookup(&self, id: &str) -> Option<I> {
        self.scopes[0].lookup(id)
    }

    pub fn insert(&self, id: &str, value: I) -> Self {
        let updated_scope = self.scopes[0].insert(id, value);

        let mut next = self.scopes.clone();
        next[0] = updated_scope;

        Self { scopes: next }
    }

    pub fn enter(&self) -> Scopes<I> {
        let mut next = self.scopes.clone();
        next.extend([Scope::default()]);
        Self { scopes: next }
    }
}

impl<I: Clone + Default> Default for Scopes<I> {
    fn default() -> Self {
        Self {
            scopes: vec![Scope::default()],
        }
    }
}

#[derive(Debug, Clone)]
pub enum CompiledConstant {
    Integer(i64),
    String(String),
    Boolean(bool),
}

#[derive(Debug, Clone)]
pub struct Analyzed {
    pub tipe: Type,
    pub constant: Option<CompiledConstant>,
}

impl Expr<Analyzed> {
    pub fn tipe(&self) -> Option<Type> {
        match self {
            Expr::FunctionCall(function_call) => function_call.aug.as_ref().map(|a| a.tipe.clone()),
            Expr::BinaryExpr(binary_expr) => binary_expr.aug.as_ref().map(|a| a.tipe.clone()),
            Expr::Identifier(identifier) => identifier.aug.as_ref().map(|a| a.tipe.clone()),
            Expr::ArgumentList(argument_list) => argument_list.aug.as_ref().map(|a| a.tipe.clone()),
            Expr::Argument(argument) => argument.aug.as_ref().map(|a| a.tipe.clone()),
            Expr::Literal(_, _, aug) => aug.as_ref().map(|a| a.tipe.clone()),
        }
    }

    pub fn constant(&self) -> Option<CompiledConstant> {
        let analyzed = match self {
            Expr::FunctionCall(function_call) => &function_call.aug,
            Expr::BinaryExpr(binary_expr) => &binary_expr.aug,
            Expr::Identifier(identifier) => &identifier.aug,
            Expr::ArgumentList(argument_list) => &argument_list.aug,
            Expr::Argument(argument) => &argument.aug,
            Expr::Literal(_, _, aug) => aug,
        };

        if let Some(analyzed) = analyzed {
            analyzed.constant.clone()
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub enum MelAnalysisError {
    Mismatch(Type, Type),
    UnknownIdentifier(String),
    MissingContext(String),
    OptimizationNotSupported(String),
    Incalculable,
}

impl Display for MelAnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mismatch(expected, found) => write!(f, "Expected {:?}, found {:?}", expected, found),
            UnknownIdentifier(i) => write!(f, "Unknown identifier {:?}", i),
            MissingContext(c) => write!(f, "Missing compiler context: {:?}", c),
            Incalculable => write!(f, "Incalculable type in expression"),
            OptimizationNotSupported(o) => write!(f, "Optimization not supported: {o}"),
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
    pub scopes: Scopes<Type>,
}

impl MelAnalysisContext {
    pub fn update_expr(&self, new: Expr<Analyzed>) -> Self {
        MelAnalysisContext {
            expr: Some(new),
            params: self.params.clone(),
            scopes: self.scopes.clone(),
        }
    }
    pub fn update_scopes(&self, new: Scopes<Type>) -> Self {
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
        let callee = self.visit_identifier(&ast.callee, context.clone(), driver)?;
        let callee = match callee.expr.unwrap() {
            Expr::Identifier(id) => Ok(id),
            _ => Err(MissingContext("TODO".into())),
        }
        .map_err(|e| MelAnalysisLocatableError {
            error: e,
            location: ast.location.clone(),
        })?;

        let found_callee =
            context
                .scopes
                .lookup(&ast.callee.identifier)
                .ok_or(MelAnalysisLocatableError {
                    error: UnknownIdentifier(ast.callee.identifier.clone()),
                    location: ast.location.clone(),
                })?;

        let fn_params = match found_callee {
            Type::Function(return_type, args) => (return_type, args),
            _ => {
                return Err(MelAnalysisLocatableError {
                    error: Mismatch(Function(Arc::new(Type::None), vec![]), found_callee),
                    location: ast.location.clone(),
                });
            }
        };

        let context_with_params = context.update_params(fn_params.1.clone());
        let args = self.visit_argument_list(&ast.arguments, context_with_params, driver)?;
        let args = match args.expr.unwrap() {
            Expr::ArgumentList(argument_list) => Ok(argument_list),
            _ => Err(MissingContext("TODO".into())),
        }
        .map_err(|e| MelAnalysisLocatableError {
            error: e,
            location: ast.location.clone(),
        })?;

        Ok(
            context.update_expr(Expr::FunctionCall(Arc::new(FunctionCall {
                callee: (*callee).clone(),
                location: ast.location.clone(),
                arguments: (*args).clone(),
                aug: Some(Analyzed {
                    tipe: Type::Function(fn_params.0.clone(), fn_params.1.clone()),
                    constant: None,
                }),
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
            aug: Some(Analyzed {
                tipe: found_id,
                constant: None,
            }),
        }))))
    }

    fn visit_argument_list(
        &self,
        ast: &ArgumentList<()>,
        context: MelAnalysisContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        let params = context.params.as_ref().ok_or(MelAnalysisLocatableError {
            error: MissingContext("Parameters expected".to_string()),
            location: ast.location.clone(),
        })?;

        let mut arg_types: Vec<Argument<Analyzed>> = vec![];
        for arg in ast.arguments.iter().zip(params) {
            let arg = self
                .visit_argument(arg.0, context.update_params(vec![arg.1.clone()]), driver)?
                .expr
                .ok_or(MelAnalysisLocatableError {
                    error: Incalculable,
                    location: arg.0.location.clone(),
                })?;
            let arg = match arg {
                Expr::Argument(argument) => Ok(argument),
                _ => Err(MissingContext("TODO".into())),
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
                aug: Some(Analyzed {
                    tipe: Type::None,
                    constant: None,
                }),
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
            error: MissingContext("Parameters expected".to_string()),
            location: ast.location.clone(),
        })?;
        let arg = driver.visit(&ast.expr, self, context.clone())?.expr.ok_or(
            MelAnalysisLocatableError {
                error: Incalculable,
                location: ast.location.clone(),
            },
        )?;

        let arg_type = arg
            .tipe()
            .ok_or(MissingContext("Could not get type of argument".into()))
            .map_err(|e| MelAnalysisLocatableError {
                error: e,
                location: ast.location.clone(),
            })?;
        if arg_type != params[0] {
            return Err(MelAnalysisLocatableError {
                error: Mismatch(arg_type, params[0].clone()),
                location: arg.location(),
            });
        }
        Ok(context.update_expr(Expr::Argument(Arc::new(Argument {
            expr: arg,
            location: ast.location.clone(),
            aug: Some(Analyzed {
                tipe: arg_type,
                constant: None,
            }),
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
                error: Incalculable,
                location: ast.left.location(),
            },
        )?;
        let right = driver
            .visit(&ast.right, self, context.clone())?
            .expr
            .ok_or(MelAnalysisLocatableError {
                error: Incalculable,
                location: ast.right.location(),
            })?;

        let left_type = left
            .tipe()
            .ok_or(MissingContext(
                "Could not get type of left expression in binary expression".into(),
            ))
            .map_err(|e| MelAnalysisLocatableError {
                error: e,
                location: ast.location.clone(),
            })?;

        let right_type = right
            .tipe()
            .ok_or(MissingContext(
                "Could not get type of left expression in binary expression".into(),
            ))
            .map_err(|e| MelAnalysisLocatableError {
                error: e,
                location: ast.location.clone(),
            })?;

        if left_type != right_type {
            return Err(MelAnalysisLocatableError {
                error: Mismatch(left_type, right_type),
                location: ast.location.clone(),
            });
        }

        let result_type = match &ast.op {
            BinaryInfixOperator::Logic(_) => Type::Boolean,
            BinaryInfixOperator::Comparison(_) => Type::Boolean,
            BinaryInfixOperator::Math(_) => Type::Integer,
            BinaryInfixOperator::Concat(_) => Type::String,
        };

        Ok(context.update_expr(Expr::BinaryExpr(Arc::new(BinaryExpr {
            left,
            right,
            op: ast.op.clone(),
            location: ast.location.clone(),
            aug: Some(Analyzed {
                tipe: result_type,
                constant: None,
            }),
        }))))
    }

    fn visit_literal(
        &self,
        ast: (&ast::Literal, &GrammarLocation, &Option<()>),
        context: MelAnalysisContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        match ast {
            (b @ ast::Literal::Boolean(_), location, _) => Ok(context.update_expr(Expr::Literal(
                b.clone(),
                location.clone(),
                Some(Analyzed {
                    tipe: Type::Boolean,
                    constant: None,
                }),
            ))),
            (n @ ast::Literal::Number(_), location, _) => Ok(context.update_expr(Expr::Literal(
                n.clone(),
                location.clone(),
                Some(Analyzed {
                    tipe: Type::Integer,
                    constant: None,
                }),
            ))),
            (s @ ast::Literal::String(_), location, _) => Ok(context.update_expr(Expr::Literal(
                s.clone(),
                location.clone(),
                Some(Analyzed {
                    tipe: Type::String,
                    constant: None,
                }),
            ))),
        }
    }
}

#[cfg(test)]
mod type_check_tests {
    use crate::{
        analysis::{
            Analyzed, MelAnalysisContext, MelAnalysisError, MelAnalysisLocatableError,
            MelTypeChecker,
        },
        ast::{
            self, AstVisitorDriver, BinaryExpr, Expr, FunctionCall, Identifier,
            Type::{self, Function, Integer, Boolean},
        },
        compiler::{CompilerError, MELCompilerContext, SyntaxError::EmptyContext, compile},
        expect_expr,
        grammar::GrammarLocation,
    };
    use std::{assert_matches, sync::Arc};

    #[test]
    fn test_type_check_literal() {
        let expr = "5";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
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
            Some(Analyzed {
                tipe: Type::Integer,
                constant: None,
            })
        )
    }

    #[test]
    fn test_type_check_binary_expr() {
        let expr = "5 + 3";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
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
                aug: Some(Analyzed {
                    tipe: Type::Integer,
                    constant: _
                })
            }
        )
    }

    #[test]
    fn test_type_check_binary_expr_nested() {
        let expr = "5 + (3 * 8)";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
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
                aug: Some(Analyzed {
                    tipe: Type::Integer,
                    constant: _
                })
            }
        );
    }

    #[test]
    fn test_type_check_binary_expr_error() {
        let expr = "5 + \"testing\"";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let context = MelAnalysisContext::default();
        let result = driver.visit(&ast, &visitor, context);

        assert_matches!(
            result,
            Err(MelAnalysisLocatableError {
                error: MelAnalysisError::Mismatch(Integer, ast::Type::String),
                location: GrammarLocation {
                    start: 0,
                    extent: 13
                }
            })
        );
    }

    #[test]
    fn test_type_check_identifier_missing() {
        let expr = "a";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
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
        let ast = expect_expr!(compiled)
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
                aug: Some(Analyzed {
                    tipe: Type::Integer,
                    constant: _
                })
            }
        );
    }

    #[test]
    fn test_type_check_identifier() {
        let expr = "a";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
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
                aug: Some(Analyzed {
                    tipe: Type::Integer,
                    constant: _
                })
            }
        );
    }

    #[test]
    fn test_type_check_function_call() {
        let expr = "testing(5)";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert(
            "testing",
            Function(Arc::new(Type::Integer), vec![Type::Integer]),
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
                aug: Some(Analyzed {
                    tipe: Function(_, _),
                    constant: _
                })
            }
        );
    }

    #[test]
    fn test_type_check_function_call_error() {
        let expr = "use_me(5)";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert(
            "use",
            Function(Arc::new(Type::Integer), vec![Type::Integer]),
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
    fn test_type_check_function_call_in_expression() {
        let expr = "use_me(5)";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert(
            "use_me",
            Function(Arc::new(Type::Integer), vec![Type::Integer]),
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
                aug: Some(Analyzed {
                    tipe: Function(_, _),
                    constant: _
                })
            }
        );
    }

    #[test]
    fn test_type_check_function_call_in_expression2() {
        let expr = "use_me(5, \"testing\")";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert(
            "use_me",
            Function(Arc::new(Type::Integer), vec![Type::Integer, Type::String]),
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
                aug: Some(Analyzed {
                    tipe: Function(_, _),
                    constant: _
                })
            }
        );
    }

    #[test]
    fn test_type_check_function_call_in_expression3() {
        let expr = "use_me(5, \"testing\")";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert(
            "use_me",
            Function(Arc::new(Type::Integer), vec![Type::Integer, Type::Integer]),
        ));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect_err("Could analyze expression with type error");

        assert_matches!(
            result,
            MelAnalysisLocatableError {
                error: MelAnalysisError::Mismatch(Type::String, Integer),
                location: GrammarLocation {
                    start: 10,
                    extent: 9
                }
            }
        );
    }

    #[test]
    fn test_type_check_function_call_in_expression_mismatch() {
        let expr = "5 + (use_me(5) + 10)";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(context.scopes.insert(
            "use_me",
            Function(Arc::new(Type::Boolean), vec![Type::Integer]),
        ));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect_err("Could analyze expression with type error");
        assert_matches!(
            result,
            MelAnalysisLocatableError {
                error: MelAnalysisError::Mismatch(Function(_, _), Integer),
                location: GrammarLocation {
                    start: 5,
                    extent: 14
                }
            }
        );
    }

    #[test]
    fn test_type_check_comparison_expr() {
        let expr = "5 < 4";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
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
                aug: Some(Analyzed {
                    tipe: Type::Boolean,
                    constant: None,
                })
            }
        );
    }

    #[test]
    fn test_type_check_comparison_expr_mismatch() {
        let expr = "5 < 4 + 5";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
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
                error: MelAnalysisError::Mismatch(Boolean, Integer),
                location: GrammarLocation {
                    start: 0,
                    extent: 9 
                }
            }
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

pub struct MelOptimizer {}

impl MelOptimizer {
    fn evaluate_binary_expr(
        left: &CompiledConstant,
        left_type: Type,
        right: &CompiledConstant,
        right_type: Type,
        op: ast::BinaryInfixOperator,
    ) -> Result<CompiledConstant, MelAnalysisError> {
        match op {
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
        }
    }
}

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
            error: MissingContext("Visiting binary expression in optimizer analysis".into()),
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
                error: MissingContext(
                    "Left operand of binary expression in optimizer analysis".into(),
                ),
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
                error: MissingContext(
                    "Right operand of binary expression in optimizer analysis".into(),
                ),
                location: ast.location.clone(),
            })?;

        let constant = match (left.constant(), right.constant()) {
            (Some(cl), Some(cr)) => {
                if !matches!(ast.op, BinaryInfixOperator::Comparison(_)) {
                    Some(
                        Self::evaluate_binary_expr(
                            &cl,
                            left.tipe().unwrap(),
                            &cr,
                            right.tipe().unwrap(),
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
                aug: Some(Analyzed {
                    tipe: expr.tipe().unwrap(),
                    constant,
                }),
            }))),
        )
    }

    fn visit_literal(
        &self,
        _ast: (&ast::Literal, &GrammarLocation, &Option<()>),
        context: MelAnalysisContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelAnalysisContext, MelAnalysisLocatableError> {
        match _ast {
            (b @ ast::Literal::Boolean(v), location, _) => Ok(context.update_expr(Expr::Literal(
                b.clone(),
                location.clone(),
                Some(Analyzed {
                    tipe: Type::Boolean,
                    constant: Some(CompiledConstant::Boolean(*v == BooleanLiteral::True)),
                }),
            ))),
            (n @ ast::Literal::Number(NumberLiteral { literal: v }), location, _) => Ok(context
                .update_expr(Expr::Literal(
                    n.clone(),
                    location.clone(),
                    Some(Analyzed {
                        tipe: Type::Integer,
                        constant: Some(CompiledConstant::Integer(*v as i64)),
                    }),
                ))),
            (s @ ast::Literal::String(StringLiteral { literal: v }), location, _) => Ok(context
                .update_expr(Expr::Literal(
                    s.clone(),
                    location.clone(),
                    Some(Analyzed {
                        tipe: Type::String,
                        constant: Some(CompiledConstant::String(v.clone())),
                    }),
                ))),
        }
    }
}

#[cfg(test)]
mod optimizer_tests {
    use crate::{
        analysis::{
            Analyzed, CompiledConstant, MelAnalysisContext, MelOptimizer, MelTypeChecker, ast::Expr,
        },
        ast::{AstVisitorDriver, BinaryExpr, Type},
        compiler::{CompilerError, MELCompilerContext, SyntaxError::EmptyContext, compile},
        expect_expr,
    };
    use std::assert_matches;

    #[test]
    fn test_optimize_binary_expr() {
        let expr = "5 + 3";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
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
                aug: Some(Analyzed {
                    tipe: Type::Integer,
                    constant: Some(CompiledConstant::Integer(8)),
                })
            }
        );
    }

    #[test]
    fn test_optimize_nested_binary_expr() {
        let expr = "5 + 3 + (8 * 11)";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
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
                aug: Some(Analyzed {
                    tipe: Type::Integer,
                    constant: Some(CompiledConstant::Integer(96)),
                })
            }
        );
    }

    #[test]
    fn test_optimize_nested_binary_expr2() {
        let expr = "(8 * 11) / 8";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
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
                aug: Some(Analyzed {
                    tipe: Type::Integer,
                    constant: Some(CompiledConstant::Integer(11)),
                })
            }
        );
    }

    #[test]
    fn test_optimize_nested_binary_expr3() {
        let expr = "(8 * 11) % 2";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
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
                aug: Some(Analyzed {
                    tipe: Type::Integer,
                    constant: Some(CompiledConstant::Integer(0)),
                })
            }
        );
    }

    #[test]
    fn test_optimize_nested_binary_expr4() {
        let expr = "\"testing\" . \"one\"";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
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
                aug: Some(Analyzed {
                    tipe: Type::String,
                    constant: Some(CompiledConstant::String(a)),
                })
            } if a == "testingone"
        );
    }

    #[test]
    fn test_optimize_nested_binary_expr5() {
        let expr = "(\"testing\" . \"one\") . \"two\"";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
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
                aug: Some(Analyzed {
                    tipe: Type::String,
                    constant: Some(CompiledConstant::String(a)),
                })
            } if a == "testingonetwo"
        );
    }

    #[test]
    fn test_optimize_logical_binary_expr() {
        let expr = "true or false";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
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
                aug: Some(Analyzed {
                    tipe: Type::Boolean,
                    constant: Some(CompiledConstant::Boolean(true)),
                })
            }
        );
    }

    #[test]
    fn test_optimize_logical_binary_expr2() {
        let expr = "true and false";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
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
                aug: Some(Analyzed {
                    tipe: Type::Boolean,
                    constant: Some(CompiledConstant::Boolean(false)),
                })
            }
        );
    }

    #[test]
    fn test_optimize_comparison_expr() {
        let expr = "5 < 4";

        let compile_result = compile(expr);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
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
                aug: Some(Analyzed {
                    tipe: Type::Boolean,
                    constant: None,
                })
            }
        );
    }
}
