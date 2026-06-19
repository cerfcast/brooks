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

use crate::{
    analysis::{self, Analyzed},
    ast::{
        self, Argument, ArgumentList, AstVisitor, AstVisitorDriver, AstVisitorResult, BinaryExpr,
        FunctionCall, Identifier,
    },
    grammar::GrammarLocation,
};

#[allow(clippy::to_string_trait_impl)]
impl ToString for ast::BinaryInfixOperator {
    fn to_string(&self) -> String {
        match self {
            ast::BinaryInfixOperator::Logic(lo) => lo.to_string(),
            ast::BinaryInfixOperator::Math(m) => m.to_string(),
            ast::BinaryInfixOperator::Concat(_) => "concat".to_string(),
            ast::BinaryInfixOperator::Comparison(c) => c.to_string(),
        }
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ast::LogicOperator {
    fn to_string(&self) -> String {
        match self {
            ast::LogicOperator::And => "and".into(),
            ast::LogicOperator::Or => "or".into(),
        }
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ast::ComparisonOperator {
    fn to_string(&self) -> String {
        match self {
            ast::ComparisonOperator::Eq => "==".into(),
            ast::ComparisonOperator::Lt => "<".into(),
            ast::ComparisonOperator::Lte => "<=".into(),
            ast::ComparisonOperator::Gt => ">".into(),
            ast::ComparisonOperator::Gte => ">=".into(),
            ast::ComparisonOperator::Ne => "!=".into(),
        }
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ast::MathOperator {
    fn to_string(&self) -> String {
        match self {
            ast::MathOperator::Plus => "plus".into(),
            ast::MathOperator::Minus => "minus".into(),
            ast::MathOperator::Multiply => "multiply".into(),
            ast::MathOperator::Divide => "divide".into(),
            ast::MathOperator::Modulo => "modulo".into(),
        }
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ast::Literal {
    fn to_string(&self) -> String {
        match self {
            ast::Literal::Boolean(bl) => bl.to_string(),
            ast::Literal::Number(nl) => nl.to_string(),
            ast::Literal::String(sl) => sl.to_string(),
        }
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ast::BooleanLiteral {
    fn to_string(&self) -> String {
        match self {
            ast::BooleanLiteral::True => "true".into(),
            ast::BooleanLiteral::False => "false".into(),
        }
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ast::NumberLiteral {
    fn to_string(&self) -> String {
        self.literal.to_string()
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ast::StringLiteral {
    fn to_string(&self) -> String {
        self.literal.to_string()
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ast::Type {
    fn to_string(&self) -> String {
        match self {
            ast::Type::Boolean => "Bool".into(),
            ast::Type::Integer => "Integer".into(),
            ast::Type::String => "String".into(),
            ast::Type::Params(items) => format!(
                "Parameters: {}",
                items.iter().fold("".to_string(), |o, n| o + &n.to_string())
            ),
            ast::Type::Function(result, args) => format!(
                "Return Type: {}, Argument Types: {}",
                result.to_string(),
                args.iter().fold("".to_string(), |o, n| o + &n.to_string())
            ),
            ast::Type::None => "None".to_string(),
        }
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for analysis::CompiledConstant {
    fn to_string(&self) -> String {
        match self {
            analysis::CompiledConstant::Integer(i) => i.to_string(),
            analysis::CompiledConstant::String(s) => s.clone(),
            analysis::CompiledConstant::Boolean(b) => b.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AstTextSerializerError {}

pub struct AstTextSerializer {}

pub struct AstTextSerializerContext {
    pub serialized: String,
    pub indent: usize,
}

impl AstTextSerializerContext {
    fn indent(&self) -> Self {
        AstTextSerializerContext {
            serialized: self.serialized.clone(),
            indent: self.indent + 1,
        }
    }
    fn unindent(&self) -> Self {
        AstTextSerializerContext {
            serialized: self.serialized.clone(),
            indent: self.indent - 1,
        }
    }
    fn append(&self, addition: String) -> Self {
        AstTextSerializerContext {
            serialized: self.serialized.clone() + &addition,
            indent: self.indent,
        }
    }
}

impl AstVisitor<AstTextSerializerContext, (), AstTextSerializerError> for AstTextSerializer {
    fn visit_function_call(
        &self,
        ast: &FunctionCall<()>,
        context: AstTextSerializerContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<AstTextSerializerContext, AstTextSerializerError> {
        let mut context = context.append(
            ("\t".repeat(context.indent) + "Function Call:\n").to_string()
                + &("\t".repeat(context.indent + 1) + "Callee: " + &ast.callee.identifier + "\n")
                    .to_string(),
        );
        context = self.visit_argument_list(&ast.arguments, context.indent(), driver)?;
        Ok(context.unindent())
    }

    fn visit_identifier(
        &self,
        ast: &Identifier<()>,
        context: AstTextSerializerContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<AstTextSerializerContext, AstTextSerializerError> {
        let output = "\t".repeat(context.indent) + "Identifier: " + &ast.identifier;
        Ok(context.append(output))
    }

    fn visit_argument_list(
        &self,
        ast: &ArgumentList<()>,
        context: AstTextSerializerContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<AstTextSerializerContext, AstTextSerializerError> {
        let mut context = context.append("\t".repeat(context.indent) + "Arguments:\n");
        context = context.indent();
        let mut first = true;
        for arg in &ast.arguments {
            if !first {
                context = context.append("\n".into())
            }
            first = false;
            context = self.visit_argument(arg, context, driver)?;
        }
        Ok(context.unindent())
    }

    fn visit_argument(
        &self,
        ast: &Argument<()>,
        context: AstTextSerializerContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<AstTextSerializerContext, AstTextSerializerError> {
        driver.visit(&ast.expr, self, context)
    }

    fn visit_binary_expr(
        &self,
        ast: &BinaryExpr<()>,
        context: AstTextSerializerContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<AstTextSerializerContext, AstTextSerializerError> {
        let mut context = context.append("\t".repeat(context.indent) + "Binary Expression:\n");

        context = context.indent();

        context = context.append("\t".repeat(context.indent) + "Left:\n");
        context = driver.visit(&ast.left, self, context.indent())?;
        context = context.unindent();

        context = context.append("\n".into());

        context = context
            .append("\t".repeat(context.indent) + &format!("Operation: {}\n", ast.op.to_string()));

        context = context.append("\t".repeat(context.indent) + "Right:\n");
        context = driver.visit(&ast.right, self, context.indent())?;
        context = context.unindent();

        context = context.unindent();
        Ok(context)
    }

    fn visit_literal(
        &self,
        ast: (&ast::Literal, &GrammarLocation, &()),
        context: AstTextSerializerContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<AstTextSerializerContext, AstTextSerializerError> {
        Ok(context.append("\t".repeat(context.indent) + "Literal: " + &ast.0.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        compiler::{CompilerError, MELCompilerContext, SyntaxError::EmptyContext, compile},
        expect_expr,
        serializer::{AstTextSerializer, AstTextSerializerContext, AstVisitorDriver},
    };

    #[test]
    fn serialize_function_call() {
        let code = "testing(one(hello),b)";
        let expected = "Function Call:
\tCallee: testing
\tArguments:
\t\tFunction Call:
\t\t\tCallee: one
\t\t\tArguments:
\t\t\t\tIdentifier: hello
\t\tIdentifier: b";

        let compile_result = compile(code);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = AstTextSerializer {};
        let context = AstTextSerializerContext {
            serialized: "".to_string(),
            indent: 0,
        };
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not serialize");
        pretty_assertions::assert_eq!(result.serialized, expected);
    }

    #[test]
    fn serialize_binary_expr() {
        let code = "a and b";
        let expected = "Binary Expression:
\tLeft:
\t\tIdentifier: a
\tOperation: and
\tRight:
\t\tIdentifier: b";

        let compile_result = compile(code);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = AstTextSerializer {};
        let context = AstTextSerializerContext {
            serialized: "".to_string(),
            indent: 0,
        };
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not serialize");
        pretty_assertions::assert_eq!(result.serialized, expected);
    }

    #[test]
    fn serialize_binary_math_expr() {
        let code = "a + b";
        let expected = "Binary Expression:
\tLeft:
\t\tIdentifier: a
\tOperation: plus
\tRight:
\t\tIdentifier: b";

        let compile_result = compile(code);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = AstTextSerializer {};
        let context = AstTextSerializerContext {
            serialized: "".to_string(),
            indent: 0,
        };
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not serialize");
        pretty_assertions::assert_eq!(result.serialized, expected);
    }

    #[test]
    fn serialize_binary_math_expr_associative() {
        let code = "a + b - c";
        let expected = "Binary Expression:
\tLeft:
\t\tBinary Expression:
\t\t\tLeft:
\t\t\t\tIdentifier: a
\t\t\tOperation: plus
\t\t\tRight:
\t\t\t\tIdentifier: b
\tOperation: minus
\tRight:
\t\tIdentifier: c";

        let compile_result = compile(code);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");
        let driver = AstVisitorDriver {};
        let visitor = AstTextSerializer {};
        let context = AstTextSerializerContext {
            serialized: "".to_string(),
            indent: 0,
        };
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not serialize");
        pretty_assertions::assert_eq!(result.serialized, expected);
    }

    #[test]
    fn serialize_binary_math_expr_grouped() {
        let code = "(a + b) - c";
        let expected = "Binary Expression:
\tLeft:
\t\tBinary Expression:
\t\t\tLeft:
\t\t\t\tIdentifier: a
\t\t\tOperation: plus
\t\t\tRight:
\t\t\t\tIdentifier: b
\tOperation: minus
\tRight:
\t\tIdentifier: c";

        let compile_result = compile(code);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = AstTextSerializer {};
        let context = AstTextSerializerContext {
            serialized: "".to_string(),
            indent: 0,
        };
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not serialize");
        pretty_assertions::assert_eq!(result.serialized, expected);
    }

    #[test]
    fn serialize_binary_math_expr_grouped_right() {
        let code = "a + (b - c)";
        let expected = "Binary Expression:
\tLeft:
\t\tIdentifier: a
\tOperation: plus
\tRight:
\t\tBinary Expression:
\t\t\tLeft:
\t\t\t\tIdentifier: b
\t\t\tOperation: minus
\t\t\tRight:
\t\t\t\tIdentifier: c";

        let compile_result = compile(code);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = AstTextSerializer {};
        let context = AstTextSerializerContext {
            serialized: "".to_string(),
            indent: 0,
        };
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not serialize");
        pretty_assertions::assert_eq!(result.serialized, expected);
    }

    #[test]
    fn serialize_binary_math_expr_grouped_right2() {
        let code = "a * (b % c)";
        let expected = "Binary Expression:
\tLeft:
\t\tIdentifier: a
\tOperation: multiply
\tRight:
\t\tBinary Expression:
\t\t\tLeft:
\t\t\t\tIdentifier: b
\t\t\tOperation: modulo
\t\t\tRight:
\t\t\t\tIdentifier: c";

        let compile_result = compile(code);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = AstTextSerializer {};
        let context = AstTextSerializerContext {
            serialized: "".to_string(),
            indent: 0,
        };
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not serialize");
        pretty_assertions::assert_eq!(result.serialized, expected);
    }

    #[test]
    fn serialize_boolean_expression() {
        let code = "true or false";
        let expected = "Binary Expression:
\tLeft:
\t\tLiteral: true
\tOperation: or
\tRight:
\t\tLiteral: false";

        let compile_result = compile(code);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = AstTextSerializer {};
        let context = AstTextSerializerContext {
            serialized: "".to_string(),
            indent: 0,
        };
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not serialize");
        pretty_assertions::assert_eq!(result.serialized, expected);
    }

    #[test]
    fn serialize_boolean_expression2() {
        let code = "true < false";
        let expected = "Binary Expression:
\tLeft:
\t\tLiteral: true
\tOperation: <
\tRight:
\t\tLiteral: false";

        let compile_result = compile(code);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = AstTextSerializer {};
        let context = AstTextSerializerContext {
            serialized: "".to_string(),
            indent: 0,
        };
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not serialize");
        pretty_assertions::assert_eq!(result.serialized, expected);
    }

    #[test]
    fn serialize_boolean_expression3() {
        let code = "(1 < 2) != true";
        let expected = "Binary Expression:
\tLeft:
\t\tBinary Expression:
\t\t\tLeft:
\t\t\t\tLiteral: 1
\t\t\tOperation: <
\t\t\tRight:
\t\t\t\tLiteral: 2
\tOperation: !=
\tRight:
\t\tLiteral: true";
        let compile_result = compile(code);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = AstTextSerializer {};
        let context = AstTextSerializerContext {
            serialized: "".to_string(),
            indent: 0,
        };
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not serialize");
        pretty_assertions::assert_eq!(result.serialized, expected);
    }

    #[test]
    fn serialize_number_expression() {
        let code = "5 + 4";
        let expected = "Binary Expression:
\tLeft:
\t\tLiteral: 5
\tOperation: plus
\tRight:
\t\tLiteral: 4";

        let compile_result = compile(code);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = AstTextSerializer {};
        let context = AstTextSerializerContext {
            serialized: "".to_string(),
            indent: 0,
        };
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not serialize");
        pretty_assertions::assert_eq!(result.serialized, expected);
    }

    #[test]
    fn serialize_string_expression() {
        let code = "\"testing\" . \"one\"";
        let expected = "Binary Expression:
\tLeft:
\t\tLiteral: testing
\tOperation: concat
\tRight:
\t\tLiteral: one";

        let compile_result = compile(code);
        let compiled = compile_result.expect("Compilation error");
        let ast = expect_expr!(compiled)
            .ok_or(CompilerError::SyntaxError(EmptyContext))
            .expect("Missing AST");

        let driver = AstVisitorDriver {};
        let visitor = AstTextSerializer {};
        let context = AstTextSerializerContext {
            serialized: "".to_string(),
            indent: 0,
        };
        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not serialize");
        pretty_assertions::assert_eq!(result.serialized, expected);
    }
}

impl AstVisitor<AstTextSerializerContext, Analyzed, AstTextSerializerError> for AstTextSerializer {
    fn visit_function_call(
        &self,
        ast: &FunctionCall<Analyzed>,
        context: AstTextSerializerContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<AstTextSerializerContext, AstTextSerializerError> {
        let mut context = context.append(
            ("\t".repeat(context.indent) + "Function Call:\n").to_string()
                + &("\t".repeat(context.indent + 1) + "Callee: " + &ast.callee.identifier + "\n")
                    .to_string(),
        );
        context = self.visit_argument_list(&ast.arguments, context.indent(), driver)?;

        context = context.append("\n".into());
        context = context
            .append("\t".repeat(context.indent) + &format!("Type: {}", ast.aug.tipe.to_string()));

        Ok(context.unindent())
    }

    fn visit_identifier(
        &self,
        ast: &Identifier<Analyzed>,
        context: AstTextSerializerContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<AstTextSerializerContext, AstTextSerializerError> {
        let output = "\t".repeat(context.indent) + "Identifier: " + &ast.identifier;
        Ok(context.append(output))
    }

    fn visit_argument_list(
        &self,
        ast: &ArgumentList<Analyzed>,
        context: AstTextSerializerContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<AstTextSerializerContext, AstTextSerializerError> {
        let mut context = context.append("\t".repeat(context.indent) + "Arguments:\n");
        context = context.indent();
        let mut first = true;
        for arg in &ast.arguments {
            if !first {
                context = context.append("\n".into())
            }
            first = false;
            context = self.visit_argument(arg, context, driver)?;
        }
        Ok(context.unindent())
    }

    fn visit_argument(
        &self,
        ast: &Argument<Analyzed>,
        context: AstTextSerializerContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<AstTextSerializerContext, AstTextSerializerError> {
        driver.visit(&ast.expr, self, context)
    }

    fn visit_binary_expr(
        &self,
        ast: &BinaryExpr<Analyzed>,
        context: AstTextSerializerContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<AstTextSerializerContext, AstTextSerializerError> {
        let mut context = context.append("\t".repeat(context.indent) + "Binary Expression:\n");

        context = context.indent();

        context = context.append("\t".repeat(context.indent) + "Left:\n");
        context = driver.visit(&ast.left, self, context.indent())?;
        context = context.unindent();

        context = context.append("\n".into());

        context = context
            .append("\t".repeat(context.indent) + &format!("Operation: {}\n", ast.op.to_string()));

        context = context.append("\t".repeat(context.indent) + "Right:\n");
        context = driver.visit(&ast.right, self, context.indent())?;
        context = context.unindent();

        context = context.append("\n".into());
        context = context
            .append("\t".repeat(context.indent) + &format!("Type: {}", ast.aug.tipe.to_string()));
        context = context.append("\n".into());

        context = context.append(
            "\t".repeat(context.indent)
                + &ast
                    .aug
                    .constant
                    .as_ref()
                    .map(|c| format!("Constant value: {}", c.to_string()))
                    .unwrap_or("Not a constant".to_string()),
        );

        context = context.unindent();
        Ok(context)
    }

    fn visit_literal(
        &self,
        ast: (&ast::Literal, &GrammarLocation, &Analyzed),
        context: AstTextSerializerContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<AstTextSerializerContext, AstTextSerializerError> {
        let mut context =
            context.append("\t".repeat(context.indent) + "Literal: " + &ast.0.to_string());
        context = context.append("\n".into());

        context = context.indent();

        context = context
            .append("\t".repeat(context.indent) + &format!("Type: {}", ast.2.tipe.to_string()));
        context = context.append("\n".into());

        context = context.append(
            "\t".repeat(context.indent)
                + &ast
                    .2
                    .constant
                    .as_ref()
                    .map(|c| format!("Constant value: {}", c.to_string()))
                    .unwrap_or("Not a constant".to_string()),
        );
        Ok(context.unindent())
    }
}

#[cfg(test)]
mod analyzed_serializer_tests {
    use std::sync::Arc;

    use crate::{
        analysis::{MelAnalysisContext, MelOptimizer, MelTypeChecker},
        ast::Type::{self, Function},
        compiler::{CompilerError, MELCompilerContext, SyntaxError::EmptyContext, compile},
        expect_expr,
        serializer::{AstTextSerializer, AstTextSerializerContext, AstVisitorDriver},
    };

    #[test]
    fn serialize_analyzed_string_expression() {
        let code = "\"testing\" . \"one\"";
        let expected = "Binary Expression:
\tLeft:
\t\tLiteral: testing
\t\t\tType: String
\t\t\tConstant value: testing
\tOperation: concat
\tRight:
\t\tLiteral: one
\t\t\tType: String
\t\t\tConstant value: one
\tType: String
\tConstant value: testingone";

        let compile_result = compile(code);
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

        let driver = AstVisitorDriver {};
        let visitor = AstTextSerializer {};
        let context = AstTextSerializerContext {
            serialized: "".to_string(),
            indent: 0,
        };
        let result = driver
            .visit(&result, &visitor, context)
            .expect("Could not serialize");
        pretty_assertions::assert_eq!(result.serialized, expected);
    }

    #[test]
    fn serialize_analyzed_function_call() {
        let code = "use(5)";
        let expected = "Function Call:
\tCallee: use
\tArguments:
\t\tLiteral: 5
\t\t\tType: Integer
\t\t\tNot a constant
\tType: Return Type: Integer, Argument Types: Integer";

        let compile_result = compile(code);
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
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let result = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = AstTextSerializer {};
        let context = AstTextSerializerContext {
            serialized: "".to_string(),
            indent: 0,
        };
        let result = driver
            .visit(&result, &visitor, context)
            .expect("Could not serialize");
        pretty_assertions::assert_eq!(result.serialized, expected);
    }

    #[test]
    fn serialize_analyzed_comparison_expr() {
        let code = "5 < 4";
        let expected = "Binary Expression:
\tLeft:
\t\tLiteral: 5
\t\t\tType: Integer
\t\t\tConstant value: 5
\tOperation: <
\tRight:
\t\tLiteral: 4
\t\t\tType: Integer
\t\t\tConstant value: 4
\tType: Bool
\tNot a constant";

        let compile_result = compile(code);
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

        let driver = AstVisitorDriver {};
        let visitor = AstTextSerializer {};
        let context = AstTextSerializerContext {
            serialized: "".to_string(),
            indent: 0,
        };
        let result = driver
            .visit(&result, &visitor, context)
            .expect("Could not serialize");
        pretty_assertions::assert_eq!(result.serialized, expected);
    }
}
