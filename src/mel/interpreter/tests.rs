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
        interpreter::{
            builtins::BuiltinInterpError,
            interpret::{
                MelInterp, MelInterpAssertion, MelInterpContext, MelInterpError, StructValue,
                TypedValue,
                Value::{self, Struct},
            },
        },
        tvs::{
            self, Add_Query_MultiBuiltin, Add_QueryBuiltin, BooleanBuiltin, BuiltinFunctionType,
            Keep_Query_MultiBuiltin, LowerBuiltin, Match_ReplaceBuiltin, MatchBuiltin,
            Path_ElementBuiltin, Path_ElementsBuiltin, Remove_Query_MultiBuiltin,
            Remove_QueryBuiltin,
            Type::{self, Function},
            UpperBuiltin,
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
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
    fn test_interp_function_call_path_element() {
        let expr = "path_element(\"one/two/three\", 1)";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Path_ElementBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
    fn test_interp_function_call_path_element_out_of_bounds() {
        let expr = "path_element(\"one/two/three\", 4)";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Path_ElementBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
            },
        ));

        let result = driver
            .visit(&expr, &visitor, context)
            .expect_err("Could interpret call to path_elements with invalid arguments");

        let runtime_error = match *result.error {
            MelInterpError::BuiltinError(be) => *be,
            _ => panic!("Expected BuiltinError, but didn't get one"),
        };

        assert_matches!(runtime_error,
            BuiltinInterpError::RuntimeError(s)
            if s == "Index 4 is out of bounds (max 3)")
    }

    #[test]
    fn test_interp_function_call_path_elements_all() {
        let expr = "path_elements(\"one/two/three\", 0, 3)";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Path_ElementsBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "one/two/three"
        );
    }

    #[test]
    fn test_interp_function_call_path_elements_some() {
        let expr = "path_elements(\"one/two/three\", 1, 2)";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Path_ElementsBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "two/three"
        );
    }

    #[test]
    fn test_interp_function_call_path_elements_last() {
        let expr = "path_elements(\"one/two/three\", 2, 2)";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Path_ElementsBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "three"
        );
    }

    #[test]
    fn test_interp_function_call_path_elements_error() {
        let expr = "path_elements(\"one/two/three\", 3, 2)";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Path_ElementsBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
            },
        ));

        let result = driver
            .visit(&expr, &visitor, context)
            .expect_err("Could interpret call to path_elements with invalid arguments");

        let runtime_error = match *result.error {
            MelInterpError::BuiltinError(be) => *be,
            _ => panic!("Expected BuiltinError, but didn't get one"),
        };

        assert_matches!(runtime_error,
            BuiltinInterpError::RuntimeError(s)
            if s == "Cannot access elements from 3 to 2 -- out of order")
    }

    #[test]
    fn test_interp_function_call_match_found() {
        let expr = "match(\"testingable\", \"ti.*able\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = MatchBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "tingable"
        );
    }

    #[test]
    fn test_interp_function_call_match_not_found() {
        let expr = "match(\"testingable\", \"ta.*able\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = MatchBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s.is_empty()
        );
    }

    #[test]
    fn test_interp_function_call_match_invalid_regular_expression() {
        let expr = "match(\"testingable\", \"*able\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = MatchBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
            },
        ));

        let result = driver
            .visit(&expr, &visitor, context)
            .expect_err("Could interpret call to path_elements with invalid arguments");

        let runtime_error = match *result.error {
            MelInterpError::BuiltinError(be) => *be,
            _ => panic!("Expected BuiltinError, but didn't get one"),
        };

        assert_matches!(runtime_error,
            BuiltinInterpError::RuntimeError(s)
            if s.ends_with("is not a valid regular expression"))
    }

    #[test]
    fn test_interp_function_call_match_replace_found() {
        let expr = "match_replace(\"testingable\", \"ti.*able\", \"REPLACE\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Match_ReplaceBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "tesREPLACE"
        );
    }

    #[test]
    fn test_interp_function_call_match_replace_not_found() {
        let expr = "match_replace(\"testingable\", \"ta.*able\", \"REPLACE\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Match_ReplaceBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "testingable"
        );
    }

    #[test]
    fn test_interp_function_call_match_replace_invalid_regular_expression() {
        let expr = "match_replace(\"testingable\", \"*able\", \"REPLACE\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Match_ReplaceBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
            },
        ));

        let result = driver
            .visit(&expr, &visitor, context)
            .expect_err("Could interpret call to path_elements with invalid arguments");

        let runtime_error = match *result.error {
            MelInterpError::BuiltinError(be) => *be,
            _ => panic!("Expected BuiltinError, but didn't get one"),
        };

        assert_matches!(runtime_error,
            BuiltinInterpError::RuntimeError(s)
            if s.ends_with("is not a valid regular expression"))
    }

    #[test]
    fn test_interp_function_call_add_query_add() {
        let expr = "add_query(\"a=b&c\", \"d\", \"e\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Add_QueryBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "a=b&c&d=e"
        );
    }

    #[test]
    fn test_interp_function_call_add_query_update() {
        let expr = "add_query(\"a=b&c&d=dee\", \"c\", \"xx\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Add_QueryBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "a=b&c=xx&d=dee"
        );
    }

    #[test]
    fn test_interp_function_call_add_query_update_empty() {
        let expr = "add_query(\"a=b&c&d=dee\", \"a\", \"\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Add_QueryBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "a&c&d=dee"
        );
    }

    #[test]
    fn test_interp_function_call_add_query_multi_add() {
        let expr = "add_query_multi(\"a=b&c&d=d\", \"a,c=,e=eee,f=ffff\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Add_Query_MultiBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "a=b&c&d=d&e=eee&f=ffff"
        );
    }

    #[test]
    fn test_interp_function_call_add_query_multi_update() {
        let expr = "add_query_multi(\"a=b&c&d=d\", \"a=aa,c=cc,d=\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Add_Query_MultiBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "a=aa&c=cc&d=d"
        );
    }

    #[test]
    fn test_interp_function_call_remove_query() {
        let expr = "remove_query(\"a=b&c&d=d\", \"a\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Remove_QueryBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "c&d=d"
        );
    }

    #[test]
    fn test_interp_function_call_remove_query_middle() {
        let expr = "remove_query(\"a=b&c&d=d\", \"c\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Remove_QueryBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "a=b&d=d"
        );
    }

    #[test]
    fn test_interp_function_call_remove_query_not_found() {
        let expr = "remove_query(\"a=b&c&d=d\", \"f\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Remove_QueryBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "a=b&c&d=d"
        );
    }

    #[test]
    fn test_interp_function_call_remove_query_multi() {
        let expr = "remove_query_multi(\"a=b&c&d=d\", \"a,c\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Remove_Query_MultiBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "d=d"
        );
    }

    #[test]
    fn test_interp_function_call_remove_query_multi_missing() {
        let expr = "remove_query_multi(\"a=b&c&d=d\", \"a,c,e\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Remove_Query_MultiBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "d=d"
        );
    }

    #[test]
    fn test_interp_function_call_remove_query_multi_all() {
        let expr = "remove_query_multi(\"a=b&c&d=d\", \"a,c,d\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Remove_Query_MultiBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s.is_empty()
        );
    }

    #[test]
    fn test_interp_function_call_keep_query_multi() {
        let expr = "keep_query_multi(\"a=b&c&d=d\", \"a,c,d\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Keep_Query_MultiBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "a=b&c&d=d"
        );
    }

    #[test]
    fn test_interp_function_call_keep_query_multi_some() {
        let expr = "keep_query_multi(\"a=b&c&d=d\", \"d,a\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Keep_Query_MultiBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "a=b&d=d"
        );
    }

    #[test]
    fn test_interp_function_call_keep_query_multi_none() {
        let expr = "keep_query_multi(\"a=b&c&d=d\", \"\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = Keep_Query_MultiBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s.is_empty()
        );
    }

    #[test]
    fn test_interp_function_call_upper() {
        let expr = "upper(\"abcd\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = UpperBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "ABCD"
        );
    }

    #[test]
    fn test_interp_function_call_lower() {
        let expr = "lower(\"AbCd\")";

        let compile_result = compile(expr);
        let ast = compile_result.expect("Compilation error");

        let driver = AstVisitorDriver {};
        let visitor = MelTypeChecker {};

        let b = LowerBuiltin {};

        let mut context = MelAnalysisContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            Function(
                b.name(),
                b.return_type_calculator(),
                b.params_type_checker(),
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

        let expr = result.expr.expect("Could not get the analyzed expression");

        let driver = AstVisitorDriver {};
        let visitor = MelInterp {};
        let mut context = MelInterpContext::default();

        context = context.update_scopes(&context.scopes.insert(
            &b.name(),
            TypedValue {
                value: Value::Function(Arc::new(b.clone())),
                tipe: Function(
                    b.name(),
                    b.return_type_calculator(),
                    b.params_type_checker(),
                ),
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
            }) if s == "abcd"
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
