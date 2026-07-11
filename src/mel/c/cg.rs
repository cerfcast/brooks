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

use crate::common::GrammarLocation;
use crate::logging::{LogLevel, LogMsg, LogMsgs};

use crate::mel::ast::{ComparisonOperator, LogicOperator, MathOperator, StringConcatOperator};
use crate::mel::{
    analysis::Analyzed,
    ast::{
        self, Argument, ArgumentList, AstVisitor, AstVisitorDriver, AstVisitorResult, BinaryExpr,
        FunctionCall, IPAddressLiteral, Identifier, NumberLiteral, RegexLiteral, StringLiteral,
        TernaryExpr,
    },
    scope,
    tvs::Type,
};

use std::error::Error;

#[derive(Debug)]
pub enum MelCodegenError {
    WriteFailed(Box<dyn Error>),
    UnknownIdentifier(String),
}

#[derive(Debug)]
pub struct MelCodegenLocatableError {
    pub location: GrammarLocation,
    pub error: MelCodegenError,
    pub context: MelCodegenContext,
}

pub type MelCodegenResult = Result<MelCodegenContext, MelCodegenLocatableError>;

#[derive(Clone, Debug)]
pub struct LocatableString {
    pub s: String,
    pub l: GrammarLocation,
}

#[derive(Clone, Debug, Default)]
pub struct SSA {
    next: usize,
    handle: String,
}

impl SSA {
    pub fn new(handle: &str) -> Self {
        SSA {
            next: 0usize,
            handle: handle.to_string(),
        }
    }

    pub fn usse(&self) -> (String, Self) {
        let mut next = self.clone();
        let next_handle = format!("{}_{}", self.handle, self.next);
        next.next += 1;

        (next_handle, next)
    }
}

#[derive(Clone, Debug, Default)]
pub struct MelCodegenContext {
    pub scopes: scope::Scopes<Type>,
    pub code: Vec<LocatableString>,
    pub ssa_gen: SSA,
    pub ssa: String,
    pub log: LogMsgs,
}

impl MelCodegenContext {
    pub fn update_log(&self, new: LogMsgs) -> Self {
        MelCodegenContext {
            code: self.code.clone(),
            scopes: self.scopes.clone(),
            ssa: self.ssa.clone(),
            ssa_gen: self.ssa_gen.clone(),
            log: new,
        }
    }

    pub fn append_code(&self, code: LocatableString) -> Self {
        let mut nc = self.code.clone();
        nc.push(code);
        MelCodegenContext {
            code: nc,
            scopes: self.scopes.clone(),
            log: self.log.clone(),
            ssa: self.ssa.clone(),
            ssa_gen: self.ssa_gen.clone(),
        }
    }

    pub fn used_ssa(&self, ssa: String) -> Self {
        MelCodegenContext {
            code: self.code.clone(),
            scopes: self.scopes.clone(),
            log: self.log.clone(),
            ssa,
            ssa_gen: self.ssa_gen.clone(),
        }
    }

    pub fn next_ssa(&self) -> (String, Self) {
        let (next_handle, next_ssa) = self.ssa_gen.usse();
        (
            next_handle,
            MelCodegenContext {
                code: self.code.clone(),
                scopes: self.scopes.clone(),
                log: self.log.clone(),
                ssa: self.ssa.clone(),
                ssa_gen: next_ssa,
            },
        )
    }
}

pub(crate) fn mel_type_to_c_type(tipe: &Type) -> String {
    match tipe {
        Type::Boolean => "bool".to_string(),
        Type::Integer => "int".to_string(),
        Type::String => "std::string".to_string(),
        Type::Regex => "std::regex".to_string(),
        Type::Struct(s) => s.name.clone(),
        Type::IPAddress => todo!(),
        Type::Params(_) => todo!(),
        Type::Function(_, _) => todo!(),
        Type::None => todo!(),
    }
}

fn mel_regex_to_c_regex(re: &str) -> String {
    re.trim_start_matches("/").trim_end_matches("/").to_string()
}

fn mel_logic_operator_to_c_logic_operator(op: &LogicOperator, left: &str, right: &str) -> String {
    let op = match op {
        LogicOperator::And => "&&".to_string(),
        LogicOperator::Or => "||".to_string(),
    };

    format!("{left} {op} {right}")
}

fn mel_comparison_operator_to_c_comparison_operator(
    op: &ComparisonOperator,
    left: &str,
    right: &str,
) -> String {
    match op {
        ComparisonOperator::Eq => format!("{left} == {right}"),
        ComparisonOperator::Ne => format!("{left} != {right}"),
        ComparisonOperator::Lt => format!("{left} < {right}"),
        ComparisonOperator::Lte => format!("{left} <= {right}"),
        ComparisonOperator::Gt => format!("{left} > {right}"),
        ComparisonOperator::Gte => format!("{left} >= {right}"),
        ComparisonOperator::Re => format!("std::regex_match({left}, {right})"),
        ComparisonOperator::IP => todo!(),
    }
}

fn mel_math_operator_to_c_math_operator(op: &MathOperator, left: &str, right: &str) -> String {
    let op = match op {
        MathOperator::Plus => "+".to_string(),
        MathOperator::Minus => "-".to_string(),
        MathOperator::Multiply => "*".to_string(),
        MathOperator::Divide => "/".to_string(),
        MathOperator::Modulo => "%".to_string(),
    };
    format!("{left} {op} {right}")
}

fn mel_concat_operator_to_c_concat_operator(
    _: &StringConcatOperator,
    left: &str,
    right: &str,
) -> String {
    format!("{left} + {right}")
}

macro_rules! decl {
    ($tpe:expr, $ssa:expr, $val:expr) => {
        format!("{} {} = {};", mel_type_to_c_type(&$tpe), $ssa, $val)
    };
}

pub struct MelCodegen {}

impl AstVisitor<MelCodegenContext, Analyzed, MelCodegenLocatableError> for MelCodegen {
    fn visit_function_call(
        &self,
        ast: &FunctionCall<Analyzed>,
        context: MelCodegenContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelCodegenContext, MelCodegenLocatableError> {
        let _context = context.update_log(trace_with_loc!(
            context.log,
            ast.location.clone(),
            "Generating code for function call expression"
        ));

        todo!()
    }

    fn visit_identifier(
        &self,
        ast: &Identifier<Analyzed>,
        context: MelCodegenContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelCodegenContext, MelCodegenLocatableError> {
        let context = context.update_log(trace_with_loc!(
            context.log,
            ast.location.clone(),
            "Generating code for identifier expression"
        ));

        let (ssa, mut context) = context.next_ssa();

        let found_id = context
            .scopes
            .lookup(&ast.identifier)
            .as_ref()
            .ok_or(MelCodegenLocatableError {
                error: MelCodegenError::UnknownIdentifier(ast.identifier.clone()),
                location: ast.location.clone(),
                context: context.clone(),
            })?
            .clone();

        let decl = decl!(found_id, ssa, ast.identifier);

        context = context.used_ssa(ssa);

        context = context.append_code(LocatableString {
            s: decl,
            l: ast.location.clone(),
        });

        Ok(context)
    }

    fn visit_argument_list(
        &self,
        ast: &ArgumentList<Analyzed>,
        context: MelCodegenContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelCodegenContext, MelCodegenLocatableError> {
        let _context = context.update_log(trace_with_loc!(
            context.log,
            ast.location.clone(),
            "Generating code for argument list expression"
        ));

        todo!()
    }

    fn visit_argument(
        &self,
        ast: &Argument<Analyzed>,
        context: MelCodegenContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelCodegenContext, MelCodegenLocatableError> {
        let _context = context.update_log(trace_with_loc!(
            context.log,
            ast.location.clone(),
            "Generating code for argument list expression"
        ));

        todo!()
    }

    fn visit_binary_expr(
        &self,
        ast: &BinaryExpr<Analyzed>,
        context: MelCodegenContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelCodegenContext, MelCodegenLocatableError> {
        let mut context = context.update_log(trace_with_loc!(
            context.log,
            ast.location.clone(),
            "Generating code for binary expression"
        ));

        context = driver.visit(&ast.left, self, context.clone())?;
        let left_ssa = context.ssa.clone();

        context = driver.visit(&ast.right, self, context.clone())?;
        let right_ssa = context.ssa.clone();

        let (ssa, mut context) = context.next_ssa();

        let value = match &ast.op {
            ast::BinaryInfixOperator::Logic(logic_operator) => {
                mel_logic_operator_to_c_logic_operator(logic_operator, &left_ssa, &right_ssa)
            }
            ast::BinaryInfixOperator::Comparison(comparison_operator) => {
                mel_comparison_operator_to_c_comparison_operator(
                    comparison_operator,
                    &left_ssa,
                    &right_ssa,
                )
            }
            ast::BinaryInfixOperator::Math(math_operator) => {
                mel_math_operator_to_c_math_operator(math_operator, &left_ssa, &right_ssa)
            }
            ast::BinaryInfixOperator::Concat(string_concat_operator) => {
                mel_concat_operator_to_c_concat_operator(
                    string_concat_operator,
                    &left_ssa,
                    &right_ssa,
                )
            }
            ast::BinaryInfixOperator::MemberAccess(_) => todo!(),
        };

        let decl = decl!(ast.aug.tipe, ssa, value);

        context = context.used_ssa(ssa);

        context = context.append_code(LocatableString {
            s: decl,
            l: ast.location.clone(),
        });

        Ok(context)
    }

    fn visit_literal(
        &self,
        ast: (&ast::Literal, &GrammarLocation, &Analyzed),
        context: MelCodegenContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelCodegenContext, MelCodegenLocatableError> {
        let context = context.update_log(trace_with_loc!(
            context.log,
            ast.1.clone(),
            "Generating code for literal expression"
        ));

        let (ssa, mut context) = context.next_ssa();

        let lit = match ast {
            (ast::Literal::Boolean(b), _, _) => LocatableString {
                s: decl!(Type::Boolean, ssa, b.to_string()),
                l: ast.1.clone(),
            },
            (ast::Literal::Number(NumberLiteral { literal: l }), _, _) => LocatableString {
                s: decl!(Type::Integer, ssa, l.to_string()),
                l: ast.1.clone(),
            },
            (ast::Literal::String(StringLiteral { literal: s }), _, _) => LocatableString {
                s: decl!(Type::String, ssa, format!("\"{s}\"")),
                l: ast.1.clone(),
            },
            (ast::Literal::Regex(RegexLiteral { literal: re }), _, _) => LocatableString {
                s: decl!(
                    Type::Regex,
                    ssa,
                    format!("std::regex(\"{}\")", mel_regex_to_c_regex(&re.to_string()))
                ),
                l: ast.1.clone(),
            },
            (ast::Literal::IPAddress(IPAddressLiteral { literal: _ }), _, _) => todo!(),
        };

        context = context.used_ssa(ssa);

        Ok(context.append_code(lit))
    }

    fn visit_ternary_expr(
        &self,
        ast: &TernaryExpr<Analyzed>,
        context: MelCodegenContext,
        driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelCodegenContext, MelCodegenLocatableError> {
        let mut context = context.update_log(trace_with_loc!(
            context.log,
            ast.location.clone(),
            "Generating code for ternary expression"
        ));

        context = driver.visit(&ast.condition, self, context.clone())?;
        let condition_ssa = context.ssa.clone();

        context = driver.visit(&ast.yes, self, context.clone())?;
        let yes_ssa = context.ssa.clone();

        context = driver.visit(&ast.no, self, context.clone())?;
        let no_ssa = context.ssa.clone();

        let (ssa, mut context) = context.next_ssa();

        let value = format!("{} ? {} : {}", condition_ssa, yes_ssa, no_ssa);
        let decl = decl!(ast.aug.tipe, ssa, value);

        context = context.used_ssa(ssa);

        context = context.append_code(LocatableString {
            s: decl,
            l: ast.location.clone(),
        });

        Ok(context)
    }

    fn visit_member_access_expr(
        &self,
        ast: &ast::MemberAccessExpression<Analyzed>,
        context: MelCodegenContext,
        _driver: &AstVisitorDriver,
    ) -> AstVisitorResult<MelCodegenContext, MelCodegenLocatableError> {
        let _context = context.update_log(trace_with_loc!(
            context.log,
            ast.location.clone(),
            "Generating code for member access expression"
        ));

        todo!()
    }
}
