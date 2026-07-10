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
mod codegen_tests {
    use std::{io::BufWriter, path::Path};

    use crate::{
        logging::LogMsgs,
        mel::{
            analysis::analyze,
            c::{
                cg::{MelCodegenContext, SSA},
                codegen, codegen_function, codegen_project,
            },
            compiler::compile,
            scope::Scopes,
            tvs::Type,
        },
        tests::read_test_file,
    };

    #[test]
    fn test_codegen_literal() {
        let expected = "// 0 to 1
int _var_0 = 5;
";

        let expr = "5";

        let expr = compile(expr).expect("Could not compile");

        let scopes = Scopes::<Type>::default();
        let expr = analyze(&expr, &scopes).expect("Could not analyze");

        let context = MelCodegenContext {
            scopes,
            code: vec![],
            log: LogMsgs::new(crate::logging::LogLevel::Trace),
            ssa_gen: SSA::new("_var"),
            ssa: String::new(),
        };
        let actual: Vec<u8> = vec![];
        let output = BufWriter::new(actual);
        let result = codegen(&expr, context, output).expect("");
        let actual = String::from_utf8_lossy(result.buffer());

        pretty_assertions::assert_eq!(expected, actual);
    }

    #[test]
    fn test_codegen_literal_regex() {
        let expected = "// 0 to 5
std::regex _var_0 = std::regex(\"[a]\");
";

        let expr = "/[a]/";

        let expr = compile(expr).expect("Could not compile");

        let scopes = Scopes::<Type>::default();
        let expr = analyze(&expr, &scopes).expect("Could not analyze");

        let context = MelCodegenContext {
            scopes,
            code: vec![],
            log: LogMsgs::new(crate::logging::LogLevel::Trace),
            ssa_gen: SSA::new("_var"),
            ssa: String::new(),
        };
        let actual: Vec<u8> = vec![];
        let output = BufWriter::new(actual);
        let result = codegen(&expr, context, output).expect("");
        let actual = String::from_utf8_lossy(result.buffer());

        pretty_assertions::assert_eq!(expected, actual);
    }

    #[test]
    fn test_codegen_identifier() {
        let expected = "// 0 to 13
int _var_0 = user_variable;
";
        let expr = "user_variable";

        let expr = compile(expr).expect("Could not compile");
        let mut scopes = Scopes::<Type>::default();
        scopes = scopes.insert("user_variable", Type::Integer);

        let expr = analyze(&expr, &scopes).expect("Could not analyze");

        let context = MelCodegenContext {
            scopes,
            code: vec![],
            log: LogMsgs::new(crate::logging::LogLevel::Trace),
            ssa_gen: SSA::new("_var"),
            ssa: String::new(),
        };

        let actual: Vec<u8> = vec![];
        let output = BufWriter::new(actual);
        let result = codegen(&expr, context, output).expect("");
        let actual = String::from_utf8_lossy(result.buffer());

        pretty_assertions::assert_eq!(expected, actual);
    }

    #[test]
    fn test_codegen_binary_expr() {
        let expected = "// 0 to 1
int _var_0 = 5;
// 4 to 5
int _var_1 = 4;
// 0 to 5
int _var_2 = _var_0 + _var_1;
";
        let expr = "5 + 4";

        let expr = compile(expr).expect("Could not compile");
        let scopes = Scopes::<Type>::default();
        let expr = analyze(&expr, &scopes).expect("Could not analyze");

        let context = MelCodegenContext {
            scopes,
            code: vec![],
            log: LogMsgs::new(crate::logging::LogLevel::Trace),
            ssa_gen: SSA::new("_var"),
            ssa: String::new(),
        };

        let actual: Vec<u8> = vec![];
        let output = BufWriter::new(actual);
        let result = codegen(&expr, context, output).expect("");
        let actual = String::from_utf8_lossy(result.buffer());

        pretty_assertions::assert_eq!(expected, actual);
    }

    #[test]
    fn test_codegen_binary_expr_regex() {
        let expected = "// 0 to 9
std::string _var_0 = \"testing\";
// 13 to 17
std::regex _var_1 = std::regex(\".*\");
// 0 to 17
bool _var_2 = std::regex_match(_var_0, _var_1);
";
        let expr = "\"testing\" ~= /.*/";

        let expr = compile(expr).expect("Could not compile");
        let scopes = Scopes::<Type>::default();
        let expr = analyze(&expr, &scopes).expect("Could not analyze");

        let context = MelCodegenContext {
            scopes,
            code: vec![],
            log: LogMsgs::new(crate::logging::LogLevel::Trace),
            ssa_gen: SSA::new("_var"),
            ssa: String::new(),
        };

        let actual: Vec<u8> = vec![];
        let output = BufWriter::new(actual);
        let result = codegen(&expr, context, output).expect("");
        let actual = String::from_utf8_lossy(result.buffer());

        pretty_assertions::assert_eq!(expected, actual);
    }

    #[test]
    fn test_codegen_function_binary_expr() {
        let expected = "int interpret() {
// 0 to 1
int _var_0 = 5;
// 4 to 5
int _var_1 = 4;
// 0 to 5
int _var_2 = _var_0 + _var_1;
// 0 to 5
return _var_2;
}
";
        let expr = "5 + 4";

        let expr = compile(expr).expect("Could not compile");
        let scopes = Scopes::<Type>::default();
        let expr = analyze(&expr, &scopes).expect("Could not analyze");

        let context = MelCodegenContext {
            scopes,
            code: vec![],
            log: LogMsgs::new(crate::logging::LogLevel::Trace),
            ssa_gen: SSA::new("_var"),
            ssa: String::new(),
        };

        let actual: Vec<u8> = vec![];
        let output = BufWriter::new(actual);
        let result = codegen_function(&expr, context, "interpret", vec![], output).expect("");
        let actual = String::from_utf8_lossy(result.buffer());

        pretty_assertions::assert_eq!(expected, actual);
    }

    #[test]
    fn test_codegen_project_binary_expr() {
        let expected = read_test_file(Path::new("src/mel/c/tests/mel1.cpp"));
        let expr = "5 + 4";

        let expr = compile(expr).expect("Could not compile");
        let scopes = Scopes::<Type>::default();
        let expr = analyze(&expr, &scopes).expect("Could not analyze");

        let context = MelCodegenContext {
            scopes,
            code: vec![],
            log: LogMsgs::new(crate::logging::LogLevel::Trace),
            ssa_gen: SSA::new("_var"),
            ssa: String::new(),
        };

        let t = tempfile::tempdir().expect("Could not create a temporary directory");
        let m = Path::new("./cpp");
        codegen_project(&expr, context, "interpret", vec![], t.path(), m).expect("");

        let mut output_path = t.path().to_path_buf();
        output_path.push("mel.cpp");
        let actual = read_test_file(&output_path);

        pretty_assertions::assert_eq!(expected, actual);
    }

    #[test]
    fn test_codegen_project_binary_expr_regex_match() {
        let expected = read_test_file(Path::new("src/mel/c/tests/mel2.cpp"));
        let expr = "\"testing\" ~= /.*/";

        let expr = compile(expr).expect("Could not compile");
        let scopes = Scopes::<Type>::default();
        let expr = analyze(&expr, &scopes).expect("Could not analyze");

        let context = MelCodegenContext {
            scopes,
            code: vec![],
            log: LogMsgs::new(crate::logging::LogLevel::Trace),
            ssa_gen: SSA::new("_var"),
            ssa: String::new(),
        };

        let t = tempfile::tempdir().expect("Could not create a temporary directory");
        let m = Path::new("./cpp");
        codegen_project(&expr, context, "interpret", vec![], t.path(), m).expect("");

        let mut output_path = t.path().to_path_buf();
        output_path.push("mel.cpp");
        let actual = read_test_file(&output_path);

        pretty_assertions::assert_eq!(expected, actual);
    }

    #[test]
    fn test_codegen_project_binary_expr_regex_no_match() {
        let expected = read_test_file(Path::new("src/mel/c/tests/mel3.cpp"));
        let expr = "\"testing\" ~= /t3st.*/";

        let expr = compile(expr).expect("Could not compile");
        let scopes = Scopes::<Type>::default();
        let expr = analyze(&expr, &scopes).expect("Could not analyze");

        let context = MelCodegenContext {
            scopes,
            code: vec![],
            log: LogMsgs::new(crate::logging::LogLevel::Trace),
            ssa_gen: SSA::new("_var"),
            ssa: String::new(),
        };

        let t = tempfile::tempdir().expect("Could not create a temporary directory");
        let m = Path::new("./cpp");
        codegen_project(&expr, context, "interpret", vec![], t.path(), m).expect("");

        let mut output_path = t.path().to_path_buf();
        output_path.push("mel.cpp");
        let actual = read_test_file(&output_path);

        pretty_assertions::assert_eq!(expected, actual);
    }
}
