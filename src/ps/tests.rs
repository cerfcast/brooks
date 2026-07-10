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

    use serde_json::Value;

    use crate::ps::spec::{
        ExpressionMatch, Header, HeaderTransform, ProcessingStages, RequestTransform,
        ResponseTransform, StageMetadata, StageRules, TypedExpressionMatch, TypedGenericMetadata,
        TypedHeader, TypedHeaderTransform, TypedProcessingStages, TypedRequestTransform,
        TypedResponseTransform, TypedStageMetadata, TypedStageRules, TypedSyntheticResponse,
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

    pub fn processing_stages(
        client_req: Vec<TypedStageRules<()>>,
        client_res: Vec<TypedStageRules<()>>,
        origin_req: Vec<TypedStageRules<()>>,
        origin_res: Vec<TypedStageRules<()>>,
    ) -> TypedProcessingStages<()> {
        TypedProcessingStages {
            tpe: "MI.ProcessingStages".to_string(),
            value: ProcessingStages::<()> {
                client_req,
                client_res,
                origin_req,
                origin_res,
                aug: (),
            },
        }
    }

    pub fn generic_metadata(tpe: &str, value: Option<Value>) -> TypedGenericMetadata<()> {
        TypedGenericMetadata {
            tpe: tpe.to_string(),
            value: value.unwrap_or_default(),
            aug: (),
        }
    }

    pub fn typed_header(name: &str, value: &str, value_expr: Option<bool>) -> TypedHeader<()> {
        TypedHeader::<()> {
            tpe: TypedHeader::<()>::typed_generic_metadata_name(),
            value: Header {
                name: name.to_string(),
                value: value.to_string(),
                value_expr,
                aug: (),
            },
        }
    }

    pub fn header_transform(
        delete: Option<Vec<String>>,
        add: Option<Vec<TypedHeader<()>>>,
        replace: Option<Vec<TypedHeader<()>>>,
    ) -> TypedHeaderTransform<()> {
        TypedHeaderTransform::<()> {
            tpe: TypedHeaderTransform::<()>::typed_generic_metadata_name(),
            value: HeaderTransform {
                delete,
                add,
                replace,
                aug: (),
            },
        }
    }

    pub fn response_transform(
        xform: Option<TypedHeaderTransform<()>>,
        response_status: Option<String>,
        response_status_expr: Option<bool>,
        synthetic: Option<TypedSyntheticResponse<()>>,
    ) -> TypedResponseTransform<()> {
        TypedResponseTransform::<()> {
            tpe: TypedResponseTransform::<()>::typed_generic_metadata_name(),
            value: ResponseTransform {
                xform,
                response_status,
                response_status_expr,
                synthetic,
                aug: (),
            },
        }
    }

    pub fn request_transform(
        xform: Option<TypedHeaderTransform<()>>,
        uri: Option<String>,
        uri_is_expr: Option<bool>,
    ) -> TypedRequestTransform<()> {
        TypedRequestTransform::<()> {
            tpe: TypedRequestTransform::<()>::typed_generic_metadata_name(),
            value: RequestTransform {
                xform,
                uri,
                uri_is_expr,
                aug: (),
            },
        }
    }
}
