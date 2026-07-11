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
    use crate::{
        logging::LogMsgs,
        mel::{
            analysis::analyze,
            c::{
                cg::{MelCodegenContext, SSA},
                codegen, codegen_project,
            },
            compiler::compile,
            scope::Scopes,
            tvs::Type,
        },
        tests::read_test_file,
    };
    use std::{io::BufWriter, path::Path};

    macro_rules! codegen_test {
        ($name:ident) => {
            #[test]
            fn $name() {
                test_codegen(
                    Path::new(&format!("src/mel/c/tests/{}.exp", stringify!($name))),
                    Path::new(&format!("src/mel/c/tests/{}.mel", stringify!($name))),
                );
            }
        };
    }

    fn test_codegen(expected: &Path, mel: &Path) {
        let expected = read_test_file(expected);
        let expr = read_test_file(mel);

        let expr = compile(&expr).expect("Could not compile");

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

    codegen_test! {test_codegen_literal}
    codegen_test! {test_codegen_literal_regex}
    codegen_test! {test_codegen_identifier}
    codegen_test! {test_codegen_binary_expr}
    codegen_test! {test_codegen_binary_expr_regex}
    codegen_test! {test_codegen_string_concat_expr}
    codegen_test! {test_codegen_ternary_expr}

    fn test_codegen_project(expected: &Path, mel: &Path) {
        let expected = read_test_file(expected);
        let expr = read_test_file(mel);

        let expr = compile(&expr).expect("Could not compile");
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
    fn test_codegen_project_binary_expr() {
        test_codegen_project(
            Path::new("src/mel/c/tests/mel1.cpp"),
            Path::new("src/mel/c/tests/mel1.mel"),
        );
    }

    #[test]
    fn test_codegen_project_binary_expr_regex_match() {
        test_codegen_project(
            Path::new("src/mel/c/tests/mel2.cpp"),
            Path::new("src/mel/c/tests/mel2.mel"),
        );
    }

    #[test]
    fn test_codegen_project_binary_expr_regex_no_match() {
        test_codegen_project(
            Path::new("src/mel/c/tests/mel3.cpp"),
            Path::new("src/mel/c/tests/mel3.mel"),
        );
    }
}
