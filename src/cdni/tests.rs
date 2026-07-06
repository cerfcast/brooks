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
pub mod test_helpers {
    use std::{fs::OpenOptions, io::Read, path::Path};

    use crate::cdni::spec::{
        ExpressionMatch, StageMetadata, StageRules, TypedExpressionMatch, TypedStageMetadata,
        TypedStageRules,
    };

    pub fn read_test_file(path: &Path) -> String {
        let mut contents: Vec<u8> = vec![];
        OpenOptions::new()
            .read(true)
            .open(path)
            .expect("Could not open test file.")
            .read_to_end(&mut contents)
            .expect("Could not read string from test file");

        String::from_utf8(contents).expect("Could not convert source to string")
    }

    pub fn expression_match(expression: &str) -> TypedExpressionMatch<()> {
        TypedExpressionMatch::<()> {
            tpe: "MI.ExpressionMatch".to_string(),
            value: ExpressionMatch::<()> {
                expression: expression.to_string(),
                aug: (),
            },
        }
    }

    pub fn stage_metadata() -> TypedStageMetadata<()> {
        TypedStageMetadata {
            tpe: "MI.StageMetadata".to_string(),
            value: StageMetadata::<()> {
                generic: None,
                request_xform: None,
                response_xform: None,
                aug: (),
            },
        }
    }

    pub fn stage_rule(
        expression: Option<TypedExpressionMatch<()>>,
        md: TypedStageMetadata<()>,
    ) -> TypedStageRules<()> {
        TypedStageRules {
            tpe: "MI.StageRules".to_string(),
            value: StageRules::<()> {
                mtch: expression,
                stage_metadata: md,
                aug: (),
            },
        }
    }
}
