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

use std::{collections::HashMap, fmt::Display};

use crate::{
    analysis::MelTypeCheckerError::{Incalculable, Mismatch, MissingContext, UnknownIdentifier},
    ast::{
        self, Argument, ArgumentList, AstVisitor, AstVisitorDriver, AstVisitorResult, BinaryExpr,
        Expr, FunctionCall, Identifier,
        Type::{self, Function},
    }, grammar::GrammarLocation,
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

pub struct MelTypeChecker {}

#[derive(Debug, Clone)]
pub struct TypedExpr {
    expr: Expr,
    tipe: Type,
}

impl PartialEq for TypedExpr {
    fn eq(&self, other: &Self) -> bool {
        self.tipe == other.tipe
    }
}

#[derive(Debug, Clone)]
pub enum MelTypeCheckerError {
    Mismatch(Type, Type),
    UnknownIdentifier(String),
    MissingContext(String),
    Incalculable,
}

impl Display for MelTypeCheckerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mismatch(expected, found) => write!(f, "Expected {:?}, found {:?}", expected, found),
            UnknownIdentifier(i) => write!(f, "Unknown identifier {:?}", i),
            MissingContext(c) => write!(f, "Missing compiler context: {:?}", c),
            Incalculable => write!(f, "Incalculable type in expression")
        }
    }
}

#[derive(Debug, Clone)]
pub struct MelTypeCheckerLocatableError {
    pub error: MelTypeCheckerError,
    pub location: GrammarLocation,
}

impl Display for MelTypeCheckerLocatableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.error, self.location)
    }
}

#[derive(Debug, Clone, Default)]
pub struct MelTypeCheckerContext {
    expr: Option<TypedExpr>,
    params: Option<Vec<Type>>,
    scopes: Scopes<Type>,
}

impl MelTypeCheckerContext {
    pub fn update_expr(&self, new: TypedExpr) -> Self {
        MelTypeCheckerContext {
            expr: Some(new),
            params: self.params.clone(),
            scopes: self.scopes.clone(),
        }
    }
    pub fn update_scopes(&self, new: Scopes<Type>) -> Self {
        MelTypeCheckerContext {
            expr: self.expr.clone(),
            params: self.params.clone(),
            scopes: new,
        }
    }
    pub fn update_params(&self, new: Vec<Type>) -> Self {
        MelTypeCheckerContext {
            expr: self.expr.clone(),
            params: Some(new),
            scopes: self.scopes.clone(),
        }
    }
}

impl AstVisitor<MelTypeCheckerContext, MelTypeCheckerLocatableError> for MelTypeChecker {
    fn visit_function_call(
        &self,
        ast: FunctionCall,
        context: MelTypeCheckerContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelTypeCheckerContext, MelTypeCheckerLocatableError> {
        let found_callee =
            context
                .scopes
                .lookup(&ast.callee.identifier)
                .ok_or(MelTypeCheckerLocatableError {
                    error: UnknownIdentifier(ast.callee.identifier.clone()),
                    location: ast.location.clone(),
                })?;
        let fn_params = match found_callee {
            Type::Function(return_type, args) => (return_type, args),
            _ => {
                return Err(MelTypeCheckerLocatableError {
                    error: Mismatch(Function(Box::new(Type::None), vec![]), found_callee),
                    location: ast.location.clone()
                });
            }
        };

        let context_with_params = context.update_params(fn_params.1);

        self.visit_argument_list(ast.arguments.clone(), context_with_params, driver)?;

        Ok(context.update_expr(TypedExpr {
            expr: Expr::FunctionCall(Box::new(ast)),
            tipe: *fn_params.0,
        }))
    }

    fn visit_identifier(
        &self,
        ast: Identifier,
        context: MelTypeCheckerContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelTypeCheckerContext, MelTypeCheckerLocatableError> {
        let found_id =
            context
                .scopes
                .lookup(&ast.identifier)
                .ok_or(MelTypeCheckerLocatableError {
                    error: UnknownIdentifier(ast.identifier.clone()),
                    location: ast.location.clone(),
                })?;

        Ok(context.update_expr(TypedExpr {
            expr: Expr::Identifier(Box::new(ast)),
            tipe: found_id,
        }))
    }

    fn visit_argument_list(
        &self,
        ast: ArgumentList,
        context: MelTypeCheckerContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelTypeCheckerContext, MelTypeCheckerLocatableError> {
        let params = context
            .params
            .as_ref()
            .ok_or(MelTypeCheckerLocatableError {
                error: MissingContext("Parameters expected".to_string()),
                location: ast.location.clone(),
            })?;

        let mut arg_types: Vec<Type> = vec![];
        for arg in ast.arguments.iter().zip(params) {
            let arg = self
                .visit_argument(
                    arg.0.clone(),
                    context.update_params(vec![arg.1.clone()]),
                    driver,
                )?
                .expr
                .ok_or(MelTypeCheckerLocatableError {
                    error: Incalculable,
                    location: arg.0.location.clone(),
                })?;
            arg_types.push(arg.tipe);
        }

        Ok(context.update_expr(TypedExpr {
            expr: Expr::ArgumentList(Box::new(ast)),
            tipe: Type::Params(arg_types),
        }))
    }

    fn visit_argument(
        &self,
        ast: Argument,
        context: MelTypeCheckerContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelTypeCheckerContext, MelTypeCheckerLocatableError> {
        let params = context
            .params
            .as_ref()
            .ok_or(MelTypeCheckerLocatableError {
                error: MissingContext("Parameters expected".to_string()),
                location: ast.location.clone()
            })?;
        let arg = driver
            .visit(ast.expr.clone(), self, context.clone())?
            .expr
            .ok_or(MelTypeCheckerLocatableError {
                error: Incalculable,
                location: ast.location.clone()
            })?;
        if arg.tipe != params[0] {
            return Err(MelTypeCheckerLocatableError {
                error: Mismatch(arg.tipe, params[0].clone()),
                location: arg.expr.location(),
            });
        }

        Ok(context.update_expr(TypedExpr {
            expr: Expr::Argument(Box::new(ast)),
            tipe: arg.tipe,
        }))
    }

    fn visit_binary_expr(
        &self,
        ast: BinaryExpr,
        context: MelTypeCheckerContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelTypeCheckerContext, MelTypeCheckerLocatableError> {
        let left = driver
            .visit(ast.left.clone(), self, context.clone())?
            .expr
            .ok_or(MelTypeCheckerLocatableError {
                error: Incalculable,
                location: ast.left.location(),
            })?;
        let right = driver
            .visit(ast.right.clone(), self, context.clone())?
            .expr
            .ok_or(MelTypeCheckerLocatableError {
                error: Incalculable,
                location: ast.right.location(),
            })?;

        if left != right {
            return Err(MelTypeCheckerLocatableError {
                error: Mismatch(left.tipe, right.tipe),
                location: ast.location.clone()
            });
        }
        Ok(context.update_expr(TypedExpr {
            expr: Expr::BinaryExpr(Box::new(ast)),
            tipe: left.tipe,
        }))
    }

    fn visit_literal(
        &self,
        ast: (ast::Literal, GrammarLocation),
        context: MelTypeCheckerContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelTypeCheckerContext, MelTypeCheckerLocatableError> {
        match ast {
            (b @ ast::Literal::Boolean(_), location) => Ok(context.update_expr(TypedExpr {
                expr: Expr::Literal(Box::new(b), location),
                tipe: Type::Boolean,
            })),
            (n @ ast::Literal::Number(_), location) => Ok(context.update_expr(TypedExpr {
                expr: Expr::Literal(Box::new(n), location),
                tipe: Type::Integer,
            })),
            (s @ ast::Literal::String(_), location) => Ok(context.update_expr(TypedExpr {
                expr: Expr::Literal(Box::new(s), location),
                tipe: Type::String,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        analysis::{
            MelTypeChecker, MelTypeCheckerContext, MelTypeCheckerError,
            MelTypeCheckerLocatableError, TypedExpr,
        },
        ast::{
            self, AstVisitorDriver,
            Type::{self, Boolean, Function, Integer},
        },
        compiler::{CompilerError, MELCompilerContext, SyntaxError::EmptyContext, compile},
        expect_expr, grammar::GrammarLocation,
    };
    use std::assert_matches;

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
        let context = MelTypeCheckerContext::default();
        let result = driver
            .visit(ast, &visitor, context)
            .expect("Could not analyze");

        assert_matches!(
            result.expr,
            Some(TypedExpr {
                expr: ast::Expr::Literal(_, _),
                tipe: Type::Integer,
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
        let context = MelTypeCheckerContext::default();
        let result = driver
            .visit(ast, &visitor, context)
            .expect("Could not analyze");

        assert_matches!(
            result.expr,
            Some(TypedExpr {
                expr: ast::Expr::BinaryExpr(_),
                tipe: Type::Integer,
            })
        );
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
        let context = MelTypeCheckerContext::default();
        let result = driver
            .visit(ast, &visitor, context)
            .expect("Could not analyze");

        assert_matches!(
            result.expr,
            Some(TypedExpr {
                expr: ast::Expr::BinaryExpr(_),
                tipe: Type::Integer,
            })
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
        let context = MelTypeCheckerContext::default();
        let result = driver.visit(ast, &visitor, context);

        assert_matches!(
            result,
            Err(MelTypeCheckerLocatableError {
                error: MelTypeCheckerError::Mismatch(Integer, ast::Type::String),
                location: GrammarLocation { start: 0, extent:  13} 
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
        let context = MelTypeCheckerContext::default();
        let result = driver
            .visit(ast, &visitor, context)
            .expect_err("Could analyze expression with missing identifier");

        assert_matches!(result, MelTypeCheckerLocatableError {
            error: MelTypeCheckerError::UnknownIdentifier(a),
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
        let mut context = MelTypeCheckerContext::default();

        context = context.update_scopes(context.scopes.insert("a", Integer));

        let result = driver
            .visit(ast, &visitor, context)
            .expect("Could not analyze expression with missing identifier");
        assert_matches!(
            result.expr,
            Some(TypedExpr {
                expr: ast::Expr::BinaryExpr(_),
                tipe: Type::Integer,
            })
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
        let mut context = MelTypeCheckerContext::default();

        context = context.update_scopes(context.scopes.insert("a", Integer));

        let result = driver
            .visit(ast, &visitor, context)
            .expect("Could not analyze expression with missing identifier");
        assert_matches!(
            result.expr,
            Some(TypedExpr {
                expr: ast::Expr::Identifier(_),
                tipe: Type::Integer,
            })
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
        let mut context = MelTypeCheckerContext::default();

        context = context.update_scopes(context.scopes.insert(
            "testing",
            Function(Box::new(Type::Integer), vec![Type::Integer]),
        ));

        let result = driver
            .visit(ast, &visitor, context)
            .expect("Could not analyze expression with missing identifier");

        assert_matches!(
            result.expr,
            Some(TypedExpr {
                expr: ast::Expr::FunctionCall(_),
                tipe: Type::Integer,
            })
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
        let mut context = MelTypeCheckerContext::default();

        context = context.update_scopes(context.scopes.insert(
            "use",
            Function(Box::new(Type::Integer), vec![Type::Integer]),
        ));

        let result = driver
            .visit(ast, &visitor, context)
            .expect_err("Could analyze expression with missing identifier");
        assert_matches!(result, MelTypeCheckerLocatableError {
            error: MelTypeCheckerError::UnknownIdentifier(a),
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
        let mut context = MelTypeCheckerContext::default();

        context = context.update_scopes(context.scopes.insert(
            "use_me",
            Function(Box::new(Type::Integer), vec![Type::Integer]),
        ));

        let result = driver
            .visit(ast, &visitor, context)
            .expect("Could not analyze expression");
        assert_matches!(
            result.expr,
            Some(TypedExpr {
                expr: ast::Expr::FunctionCall(_),
                tipe: Type::Integer,
            })
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
        let mut context = MelTypeCheckerContext::default();

        context = context.update_scopes(context.scopes.insert(
            "use_me",
            Function(Box::new(Type::Boolean), vec![Type::Integer]),
        ));

        let result = driver
            .visit(ast, &visitor, context)
            .expect_err("Could analyze expression with type error");
        assert_matches!(
            result,
            MelTypeCheckerLocatableError {
                error: MelTypeCheckerError::Mismatch(Boolean, Integer),
                location: GrammarLocation{start: 5, extent: 14}
            }
        );
    }
}
