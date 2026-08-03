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

#[cfg(test)]
mod interpreter_tests {
    use std::{assert_matches, collections::HashMap, sync::Arc};

    use crate::mel::{
        analysis::{MelAnalysisContext, MelOptimizer, MelTypeChecker},
        ast::AstVisitorDriver,
        compiler::compile,
        interpreter::interpret::{
            MelInterp, MelInterpAssertion, MelInterpContext, MelInterpError, StructValue,
            TypedValue,
            Value::{self, Struct},
        },
        tvs::{
            self, BooleanBuiltin, BuiltinFunctionType, Path_ElementBuiltin,
            Type::{self, Function},
        },
    };

    #[test]
    fn test_interp_binary_comparison_expr() {
        let expr = "5 < 5";

        let compile_result = compile(expr);
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

        let expr = result.expr.expect("Could not get analysed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let context = MelInterpContext::default();
        let result = driver
            .visit(&expr, &visitor, context)
            .expect("Could not interpret");

        assert_matches!(
            result.val,
            Some(TypedValue {
                value: Value::Boolean(false),
                tipe: Type::Boolean
            })
        );
    }

    #[test]
    fn test_interp_function_call_boolean() {
        let expr = "boolean(1)";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = BooleanBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(Arc::new(b.return_type()), b.parameters()),
        ));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Type::Function(Arc::new(b.return_type()), b.parameters()),
            },
        ));

        let result = driver
            .visit(&expr, &visitor, context)
            .expect("Could not interpret");

        assert_matches!(
            result.val,
            Some(TypedValue {
                value: Value::Boolean(true),
                tipe: Type::Boolean
            })
        );
    }

    #[test]
    fn test_interp_function_call_boolean2() {
        let expr = "boolean(0)";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = BooleanBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(Arc::new(b.return_type()), b.parameters()),
        ));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Type::Function(Arc::new(b.return_type()), b.parameters()),
            },
        ));

        let result = driver
            .visit(&expr, &visitor, context)
            .expect("Could not interpret");

        assert_matches!(
            result.val,
            Some(TypedValue {
                value: Value::Boolean(false),
                tipe: Type::Boolean
            })
        );
    }

    #[test]
    fn test_interp_function_call_path_elemnt() {
        let expr = "path_element(\"one/two/three\", 1)";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Path_ElementBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(Arc::new(b.return_type()), b.parameters()),
        ));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Type::Function(Arc::new(b.return_type()), b.parameters()),
            },
        ));

        let result = driver
            .visit(&expr, &visitor, context)
            .expect("Could not interpret");

        assert_matches!(
            result.val,
            Some(TypedValue {
                value: Value::String(s),
                tipe: Type::String
            }) if s == "two"
        );
    }

    #[test]
    fn test_interp_ternary_expression() {
        let expr = "true ? 5: 4";

        let compile_result = compile(expr);
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

        let expr = result.expr.expect("Could not get analysed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let context = MelInterpContext::default();
        let result = driver
            .visit(&expr, &visitor, context)
            .expect("Could not interpret");

        assert_matches!(
            result.val,
            Some(TypedValue {
                value: Value::Integer(5),
                tipe: Type::Integer
            })
        );
    }

    #[test]
    fn test_interp_identifier() {
        let expr = "a";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert("a", Type::String));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let expr = result.expr.expect("Could not get analysed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            "a",
            TypedValue {
                value: Value::String("Hello".to_string()),
                tipe: Type::String,
            },
        ));

        let result = driver
            .visit(&expr, &visitor, context)
            .expect("Could not interpret");

        assert_matches!(
            result.val,
            Some(TypedValue {
                value: Value::String(s),
                tipe: Type::String
            }) if s == "Hello"
        );
    }

    #[test]
    fn test_interp_member_access() {
        let code = "req^incoming";

        let compile_result = compile(code);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        let mut reqs = tvs::Struct::new("req");

        reqs.insert_field("incoming", Type::String);
        context = context.update_scopes(&context.scopes.insert("req", Type::Struct(reqs.clone())));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let expr = result.expr.expect("Could not get analysed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        let mut reqsv = StructValue {
            fields: HashMap::new(),
            tpe: reqs.clone(),
        };

        reqsv.fields.insert(
            "incoming".to_string(),
            TypedValue {
                value: Value::String("X-".to_string()),
                tipe: Type::String,
            },
        );
        context = context.update_scopes(&context.scopes.insert(
            "req",
            TypedValue {
                value: Struct(reqsv),
                tipe: Type::Struct(reqs),
            },
        ));

        let result = driver
            .visit(&expr, &visitor, context)
            .expect("Could not interpret");

        assert_matches!(
            result.val,
            Some(TypedValue {
                value: Value::String(s),
                tipe: Type::String
            }) if s == "X-"
        );
    }

    #[test]
    fn test_interp_member_access2() {
        let code = "req^incoming";

        let compile_result = compile(code);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        let mut reqs = tvs::Struct::new("req");
        reqs.insert_field("incoming", Type::Boolean);
        context = context.update_scopes(&context.scopes.insert("req", Type::Struct(reqs.clone())));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let expr = result.expr.expect("Could not get analysed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        let mut reqsv = StructValue {
            fields: HashMap::new(),
            tpe: reqs.clone(),
        };

        reqsv.fields.insert(
            "incoming".to_string(),
            TypedValue {
                value: Value::String("X-".to_string()),
                tipe: Type::String,
            },
        );
        context = context.update_scopes(&context.scopes.insert(
            "req",
            TypedValue {
                value: Struct(reqsv),
                tipe: Type::Struct(reqs),
            },
        ));

        let result = driver
            .visit(&expr, &visitor, context)
            .expect_err("Could interpret");

        match *result.error {
            MelInterpError::Assertion(e) => assert_matches!(
                *e,
                MelInterpAssertion::TypeMismatch(Type::Boolean, Type::String)
            ),
            _ => panic!("Expected to find an assertion error, but did not"),
        }
    }
}

#[cfg(test)]
mod interpreter_logger_tests {
    use crate::{
        logging::{LogLevel::Trace, LogMsgFormatter},
        mel::{
            analysis::{MelAnalysisContext, MelOptimizer, MelTypeChecker},
            ast::AstVisitorDriver,
            compiler::compile,
            interpreter::interpret::{
                MelInterp, MelInterpContext, StructValue, TypedValue,
                Value::{self, Struct},
            },
            tvs::{self, Type},
        },
    };
    use std::collections::HashMap;

    #[test]
    fn test_trace_logging_binary_expression() {
        let code = "5 + 4";

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

        let expr = result.expr.expect("Could not get analysed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_log(context.log.update_level(Trace));
        let result = driver
            .visit(&expr, &visitor, context)
            .expect("Could not interpret");

        assert_eq!(result.log.count(), 2);
        assert_eq!(
            result.log.msgs(&LogMsgFormatter {
                newline: true,
                show_level: false
            }),
            "0 to 5: Evaluating binary expression\n0 to 5: Using constant"
        );
    }

    #[test]
    fn test_trace_logging() {
        let code = "req^incoming";

        let compile_result = compile(code);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};
        let mut context = MelAnalysisContext::default();

        let mut reqs = tvs::Struct::new("req");

        reqs.insert_field("incoming", Type::Boolean);
        context = context.update_scopes(&context.scopes.insert("req", Type::Struct(reqs.clone())));

        let result = driver
            .visit(&ast, &visitor, context)
            .expect("Could not analyze");

        let driver = AstVisitorDriver {};
        let visitor = MelOptimizer {};
        let result = driver
            .visit(&ast, &visitor, result)
            .expect("Could not analyze");

        let expr = result.expr.expect("Could not get analysed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_log(context.log.update_level(Trace));

        let mut reqsv = StructValue {
            fields: HashMap::new(),
            tpe: reqs.clone(),
        };

        reqsv.fields.insert(
            "incoming".to_string(),
            TypedValue {
                value: Value::Boolean(true),
                tipe: Type::Boolean,
            },
        );

        context = context.update_scopes(&context.scopes.insert(
            "req",
            TypedValue {
                value: Struct(reqsv),
                tipe: Type::Struct(reqs),
            },
        ));

        let result = driver
            .visit(&expr, &visitor, context)
            .expect("Could interpret");

        assert_eq!(result.log.count(), 2);
        assert_eq!(
            result.log.msgs(&LogMsgFormatter {
                newline: true,
                show_level: false
            }),
            "0 to 12: Evaluating member access expression\n0 to 3: Evaluating identifier expression"
        );
    }
}

#[cfg(test)]
mod interpreter_value_tests {

    use std::assert_matches;

    use crate::mel::{
        interpreter::interpret::{MelInterpError, StructValue, TypedValue, Value},
        tvs::{Struct, Type},
    };

    #[test]
    fn test_typed_struct_field() {
        let mut st = Struct::new("st");

        st.insert_field("field1", Type::Integer);

        let mut sv = StructValue::new(st);

        assert_matches!(
            sv.insert_field(
                "field1",
                TypedValue {
                    value: Value::Integer(5),
                    tipe: Type::Integer,
                },
            ),
            Ok(_),
        )
    }

    #[test]
    fn test_mistyped_struct_field() {
        let mut st = Struct::new("st");

        st.insert_field("field1", Type::Integer);

        let mut sv = StructValue::new(st);

        let result = sv
            .insert_field(
                "field1",
                TypedValue {
                    value: Value::Boolean(false),
                    tipe: Type::Boolean,
                },
            )
            .expect_err("Could insert mistyped value into field");

        assert_matches!(
            *result,
            MelInterpError::MistypedField(_, Type::Integer, Type::Boolean)
        )
    }
}
