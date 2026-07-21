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

use std::fmt::Display;

use crate::common::GrammarLocation;
use crate::mel::ast::{
    BinaryInfixOperator, ComparisonOperator, LogicOperator, MathOperator, MemberAccessOperator,
    StringConcatOperator, TernaryOperator,
};
use crate::mel::{
    analysis::{self, Analyzed},
    ast::{
        self, Argument, ArgumentList, AstVisitor, AstVisitorDriver, AstVisitorResult, BinaryExpr,
        FunctionCall, Identifier,
    },
};

impl Display for MemberAccessOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, " . ")
    }
}

impl Display for BinaryInfixOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryInfixOperator::Logic(logic_operator) => write!(f, "{logic_operator}"),
            BinaryInfixOperator::Comparison(comparison_operator) => {
                write!(f, "{comparison_operator}")
            }
            BinaryInfixOperator::Math(math_operator) => write!(f, "{math_operator}"),
            BinaryInfixOperator::Concat(string_concat_operator) => {
                write!(f, "{string_concat_operator}")
            }
            BinaryInfixOperator::MemberAccess(member_access_operator) => {
                write!(f, "{member_access_operator}")
            }
        }
    }
}

impl Display for ComparisonOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComparisonOperator::Eq => write!(f, "=="),
            ComparisonOperator::Ne => write!(f, "!="),
            ComparisonOperator::Lt => write!(f, "<"),
            ComparisonOperator::Lte => write!(f, "<="),
            ComparisonOperator::Gt => write!(f, ">"),
            ComparisonOperator::Gte => write!(f, ">="),
            ComparisonOperator::Re => write!(f, "~="),
            ComparisonOperator::IP => write!(f, "ipmatch"),
        }
    }
}

impl Display for LogicOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogicOperator::And => write!(f, "and"),
            LogicOperator::Or => write!(f, "or"),
        }
    }
}

impl Display for StringConcatOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "concat")
    }
}

impl Display for MathOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MathOperator::Plus => write!(f, "plus"),
            MathOperator::Minus => write!(f, "minus"),
            MathOperator::Multiply => write!(f, "multiply"),
            MathOperator::Divide => write!(f, "divide"),
            MathOperator::Modulo => write!(f, "modulo"),
        }
    }
}

impl Display for TernaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TernaryOperator::Question => write!(f, "?"),
            TernaryOperator::Colon => write!(f, ":"),
        }
    }
}

impl Display for ast::Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ast::Literal::Boolean(bl) => write!(f, "{bl}"),
            ast::Literal::Number(nl) => write!(f, "{nl}"),
            ast::Literal::String(sl) => write!(f, "{sl}"),
            ast::Literal::Regex(rl) => write!(f, "{rl}"),
            ast::Literal::IPAddress(ip) => write!(f, "{ip}"),
        }
    }
}

impl Display for ast::BooleanLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ast::BooleanLiteral::True => write!(f, "true"),
            ast::BooleanLiteral::False => write!(f, "false"),
        }
    }
}

impl Display for ast::NumberLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.literal)
    }
}

impl Display for ast::StringLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.literal)
    }
}

impl Display for ast::RegexLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.literal)
    }
}

impl Display for ast::IPAddressLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.literal)
    }
}

impl Display for analysis::CompiledConstant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            analysis::CompiledConstant::Integer(i) => write!(f, "{i}"),
            analysis::CompiledConstant::String(s) => write!(f, "{s}"),
            analysis::CompiledConstant::Boolean(b) => write!(f, "{b}"),
            analysis::CompiledConstant::IPAddress(ip) => write!(f, "{ip}"),
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
        let mut context =
            context.append(("\t".repeat(context.indent) + "Function Call:\n").to_string());

        context = context.indent();
        context = context.append("\t".repeat(context.indent) + "Callee:\n");

        context = driver.visit(&ast.callee, self, context.indent())?;
        context = context.append("\n".into());
        context = context.unindent();
        context = context.unindent();

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

        context = context.append("\t".repeat(context.indent) + &format!("Operation: {}\n", ast.op));

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

    fn visit_ternary_expr(
        &self,
        ast: &ast::TernaryExpr<()>,
        context: AstTextSerializerContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<AstTextSerializerContext, AstTextSerializerError> {
        let mut context = context.append("\t".repeat(context.indent) + "Ternary Expression:\n");

        context = context.indent();

        context = context.append("\t".repeat(context.indent) + "Condition:\n");
        context = driver.visit(&ast.condition, self, context.indent())?;
        context = context.unindent();

        context = context.append("\n".into());

        context = context.append("\t".repeat(context.indent) + "Yes:\n");
        context = driver.visit(&ast.yes, self, context.indent())?;
        context = context.unindent();

        context = context.append("\n".into());

        context = context.append("\t".repeat(context.indent) + "No:\n");
        context = driver.visit(&ast.no, self, context.indent())?;
        context = context.unindent();

        context = context.unindent();
        Ok(context)
    }

    fn visit_member_access_expr(
        &self,
        ast: &ast::MemberAccessExpression<()>,
        context: AstTextSerializerContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<AstTextSerializerContext, AstTextSerializerError> {
        let mut context =
            context.append("\t".repeat(context.indent) + "Member Access Expression:\n");

        context = context.indent();
        context = context.append("\t".repeat(context.indent) + "Base:\n");
        context = driver.visit(&ast.base, self, context.indent())?;
        context = context.append("\n".into());
        context = context.unindent();

        context = context.append("\t".repeat(context.indent) + "Member:\n");
        context = self.visit_identifier(&ast.member, context.indent(), driver)?;
        context = context.unindent();

        context = context.unindent();
        Ok(context)
    }
}

#[cfg(test)]
mod tests {
    use crate::mel::{
        compiler::compile,
        serializer::{AstTextSerializer, AstTextSerializerContext, AstVisitorDriver},
    };

    #[test]
    fn serialize_function_call() {
        let code = "testing(one(hello),b)";
        let expected = "Function Call:
\tCallee:
\t\tIdentifier: testing
\tArguments:
\t\tFunction Call:
\t\t\tCallee:
\t\t\t\tIdentifier: one
\t\t\tArguments:
\t\t\t\tIdentifier: hello
\t\tIdentifier: b";

        let compile_result = compile(code);
        let ast = compile_result.expect("Compilation error");

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
        let ast = compile_result.expect("Compilation error");

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
    fn serialize_binary_regex_expr() {
        let code = "a ~= \"/[0-9]/\"";
        let expected = "Binary Expression:
\tLeft:
\t\tIdentifier: a
\tOperation: ~=
\tRight:
\t\tLiteral: /[0-9]/";

        let compile_result = compile(code);
        let ast = compile_result.expect("Compilation error");

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
        let ast = compile_result.expect("Compilation error");

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
        let ast = compile_result.expect("Compilation error");
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
        let ast = compile_result.expect("Compilation error");

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
        let ast = compile_result.expect("Compilation error");

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
        let ast = compile_result.expect("Compilation error");

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
        let ast = compile_result.expect("Compilation error");

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
        let ast = compile_result.expect("Compilation error");

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
        let ast = compile_result.expect("Compilation error");

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
        let ast = compile_result.expect("Compilation error");

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
        let ast = compile_result.expect("Compilation error");

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
    fn serialize_ternary_expr() {
        let code = "true ? 1 : 5";
        let expected = "Ternary Expression:
\tCondition:
\t\tLiteral: true
\tYes:
\t\tLiteral: 1
\tNo:
\t\tLiteral: 5";

        let compile_result = compile(code);
        let ast = compile_result.expect("Compilation error");

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
    fn serialize_ternary_expr_proper_associativity() {
        let code = "true ? a : false ? \"b\" : 10";
        let expected = "Ternary Expression:
\tCondition:
\t\tLiteral: true
\tYes:
\t\tIdentifier: a
\tNo:
\t\tTernary Expression:
\t\t\tCondition:
\t\t\t\tLiteral: false
\t\t\tYes:
\t\t\t\tLiteral: b
\t\t\tNo:
\t\t\t\tLiteral: 10";

        let compile_result = compile(code);
        let ast = compile_result.expect("Compilation error");

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
    fn serialize_member_access_expression() {
        let code = "a^b^c";
        let expected = "Member Access Expression:
\tBase:
\t\tMember Access Expression:
\t\t\tBase:
\t\t\t\tIdentifier: a
\t\t\tMember:
\t\t\t\tIdentifier: b
\tMember:
\t\tIdentifier: c";

        let compile_result = compile(code);
        let ast = compile_result.expect("Compilation error");

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
        let mut context =
            context.append(("\t".repeat(context.indent) + "Function Call:\n").to_string());

        context = context.indent();
        context = context.append("\t".repeat(context.indent) + "Callee:\n");

        context = driver.visit(&ast.callee, self, context.indent())?;
        context = context.append("\n".into());
        context = context.unindent();
        context = context.unindent();

        context = self.visit_argument_list(&ast.arguments, context.indent(), driver)?;

        context = context.append("\n".into());
        context = context.append("\t".repeat(context.indent) + &format!("Type: {}", ast.aug.tipe));

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

        context = context.append("\t".repeat(context.indent) + &format!("Operation: {}\n", ast.op));

        context = context.append("\t".repeat(context.indent) + "Right:\n");
        context = driver.visit(&ast.right, self, context.indent())?;
        context = context.unindent();

        context = context.append("\n".into());
        context = context.append("\t".repeat(context.indent) + &format!("Type: {}", ast.aug.tipe));
        context = context.append("\n".into());

        context = context.append(
            "\t".repeat(context.indent)
                + &ast
                    .aug
                    .constant
                    .as_ref()
                    .map(|c| format!("Constant value: {c}"))
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

        context = context.append("\t".repeat(context.indent) + &format!("Type: {}", ast.2.tipe));
        context = context.append("\n".into());

        context = context.append(
            "\t".repeat(context.indent)
                + &ast
                    .2
                    .constant
                    .as_ref()
                    .map(|c| format!("Constant value: {c}"))
                    .unwrap_or("Not a constant".to_string()),
        );
        Ok(context.unindent())
    }

    fn visit_ternary_expr(
        &self,
        ast: &ast::TernaryExpr<Analyzed>,
        context: AstTextSerializerContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<AstTextSerializerContext, AstTextSerializerError> {
        let mut context = context.append("\t".repeat(context.indent) + "Ternary Expression:\n");

        context = context.indent();

        context = context.append("\t".repeat(context.indent) + "Condition:\n");
        context = driver.visit(&ast.condition, self, context.indent())?;
        context = context.unindent();

        context = context.append("\n".into());

        context = context.append("\t".repeat(context.indent) + "Yes:\n");
        context = driver.visit(&ast.yes, self, context.indent())?;
        context = context.unindent();

        context = context.append("\n".into());

        context = context.append("\t".repeat(context.indent) + "No:\n");
        context = driver.visit(&ast.no, self, context.indent())?;
        context = context.unindent();

        context = context.append("\n".into());

        context = context.append("\t".repeat(context.indent) + &format!("Type: {}", ast.aug.tipe));
        context = context.append("\n".into());

        context = context.append(
            "\t".repeat(context.indent)
                + &ast
                    .aug
                    .constant
                    .as_ref()
                    .map(|c| format!("Constant value: {c}"))
                    .unwrap_or("Not a constant".to_string()),
        );

        context = context.unindent();
        Ok(context)
    }

    fn visit_member_access_expr(
        &self,
        ast: &ast::MemberAccessExpression<Analyzed>,
        context: AstTextSerializerContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<AstTextSerializerContext, AstTextSerializerError> {
        let mut context =
            context.append("\t".repeat(context.indent) + "Member Access Expression:\n");

        context = context.indent();
        context = context.append("\t".repeat(context.indent) + "Base:\n");
        context = driver.visit(&ast.base, self, context.indent())?;
        context = context.append("\n".into());
        context = context.unindent();

        context = context.append("\t".repeat(context.indent) + "Member:\n");
        context = self.visit_identifier(&ast.member, context.indent(), driver)?;
        context = context.append("\n".into());
        context = context.unindent();

        context = context.append("\t".repeat(context.indent) + &format!("Type: {}", ast.aug.tipe));
        context = context.append("\n".into());

        context = context.append(
            "\t".repeat(context.indent)
                + &ast
                    .aug
                    .constant
                    .as_ref()
                    .map(|c| format!("Constant value: {c}"))
                    .unwrap_or("Not a constant".to_string()),
        );

        context = context.unindent();

        Ok(context)
    }
}

#[cfg(test)]
mod analyzed_serializer_tests {
    use std::sync::Arc;

    use crate::mel::tvs::{
        self, Struct,
        Type::{self, Function},
    };
    use crate::mel::{
        analysis::{MelAnalysisContext, MelOptimizer, MelTypeChecker},
        compiler::compile,
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
        let ast = compile_result.expect("Compilation error");

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
\tCallee:
\t\tIdentifier: use
\tArguments:
\t\tLiteral: 5
\t\t\tType: Integer
\t\t\tNot a constant
\tType: Integer";

        let compile_result = compile(code);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
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
        let ast = compile_result.expect("Compilation error");

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
    fn serialize_analyzed_ternary_expr() {
        let code = "true ? 1 : 5";
        let expected = "Ternary Expression:
\tCondition:
\t\tLiteral: true
\t\t\tType: Bool
\t\t\tConstant value: true
\tYes:
\t\tLiteral: 1
\t\t\tType: Integer
\t\t\tConstant value: 1
\tNo:
\t\tLiteral: 5
\t\t\tType: Integer
\t\t\tConstant value: 5
\tType: Integer
\tConstant value: 1";

        let compile_result = compile(code);
        let ast = compile_result.expect("Compilation error");

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
    fn serialize_analyzed_ternary_expr2() {
        let code = "false ? 1 : (5 + 2 + 3)";
        let expected = "Ternary Expression:
\tCondition:
\t\tLiteral: false
\t\t\tType: Bool
\t\t\tConstant value: false
\tYes:
\t\tLiteral: 1
\t\t\tType: Integer
\t\t\tConstant value: 1
\tNo:
\t\tBinary Expression:
\t\t\tLeft:
\t\t\t\tBinary Expression:
\t\t\t\t\tLeft:
\t\t\t\t\t\tLiteral: 5
\t\t\t\t\t\t\tType: Integer
\t\t\t\t\t\t\tConstant value: 5
\t\t\t\t\tOperation: plus
\t\t\t\t\tRight:
\t\t\t\t\t\tLiteral: 2
\t\t\t\t\t\t\tType: Integer
\t\t\t\t\t\t\tConstant value: 2
\t\t\t\t\tType: Integer
\t\t\t\t\tConstant value: 7
\t\t\tOperation: plus
\t\t\tRight:
\t\t\t\tLiteral: 3
\t\t\t\t\tType: Integer
\t\t\t\t\tConstant value: 3
\t\t\tType: Integer
\t\t\tConstant value: 10
\tType: Integer
\tConstant value: 10";

        let compile_result = compile(code);
        let ast = compile_result.expect("Compilation error");

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
    fn serialize_analyzed_member_access_binary_expr_nested() {
        let code = "req^incoming^headers . \"testing\"";
        let expected = "Binary Expression:
\tLeft:
\t\tMember Access Expression:
\t\t\tBase:
\t\t\t\tMember Access Expression:
\t\t\t\t\tBase:
\t\t\t\t\t\tIdentifier: req
\t\t\t\t\tMember:
\t\t\t\t\t\tIdentifier: incoming
\t\t\t\t\tType: Struct: Name: headers, Fields: headers
\t\t\t\t\tNot a constant
\t\t\tMember:
\t\t\t\tIdentifier: headers
\t\t\tType: String
\t\t\tNot a constant
\tOperation: concat
\tRight:
\t\tLiteral: testing
\t\t\tType: String
\t\t\tNot a constant
\tType: String
\tNot a constant";

        let compile_result = compile(code);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        let mut headers = Struct::new("headers");
        headers.insert_field("headers", Type::String);

        let mut reqs = Struct::new("req");
        reqs.insert_field("incoming", Type::Struct(headers));

        context = context.update_scopes(&context.scopes.insert("req", Type::Struct(reqs)));

        let result = driver
            .visit(&ast, &visitor, context)
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
    fn serialize_analyzed_member_access_binary_expr() {
        let code = "req^incoming and true";
        let expected = "Binary Expression:
\tLeft:
\t\tMember Access Expression:
\t\t\tBase:
\t\t\t\tIdentifier: req
\t\t\tMember:
\t\t\t\tIdentifier: incoming
\t\t\tType: Bool
\t\t\tNot a constant
\tOperation: and
\tRight:
\t\tLiteral: true
\t\t\tType: Bool
\t\t\tNot a constant
\tType: Bool
\tNot a constant";

        let compile_result = compile(code);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        let mut reqs = Struct::new("req");
        reqs.insert_field("incoming", Type::Boolean);

        context = context.update_scopes(&context.scopes.insert("req", Type::Struct(reqs)));

        let result = driver
            .visit(&ast, &visitor, context)
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
    fn serialize_analyzed_member_access_function_call_expr() {
        let code = "req^callable(5)";
        let expected = "Function Call:
\tCallee:
\t\tMember Access Expression:
\t\t\tBase:
\t\t\t\tIdentifier: req
\t\t\tMember:
\t\t\t\tIdentifier: callable
\t\t\tType: Return Type: Bool, Argument Types: Integer
\t\t\tNot a constant
\tArguments:
\t\tLiteral: 5
\t\t\tType: Integer
\t\t\tNot a constant
\tType: Bool";

        let compile_result = compile(code);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        let mut reqs = Struct::new("req");

        reqs.insert_field(
            "callable",
            Function(
                Arc::new(Type::Boolean),
                tvs::Params {
                    args: vec![Type::Integer],
                },
            ),
        );
        context = context.update_scopes(&context.scopes.insert("req", Type::Struct(reqs)));

        let result = driver
            .visit(&ast, &visitor, context)
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
