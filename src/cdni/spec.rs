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

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticResponse {}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedSyntheticResponse {
    #[serde(rename = "generic-metadata-type")]
    tpe: String,
    #[serde(rename = "generic-metadata-value")]
    value: SyntheticResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    name: String,
    value: String,
    #[serde(rename = "value-is-expression")]
    #[serde(skip_serializing_if = "Option::is_none")]
    value_expr: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedHeader {
    #[serde(rename = "generic-metadata-type")]
    tpe: String,
    #[serde(rename = "generic-metadata-value")]
    value: Header,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderTransform {
    #[serde(skip_serializing_if = "Option::is_none")]
    delete: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    add: Option<Vec<TypedHeader>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replace: Option<Vec<TypedHeader>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedHeaderTransform {
    #[serde(rename = "generic-metadata-type")]
    tpe: String,
    #[serde(rename = "generic-metadata-value")]
    value: HeaderTransform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericMetadata {}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedGenericMetadata {
    #[serde(rename = "generic-metadata-type")]
    tpe: String,
    #[serde(rename = "generic-metadata-value")]
    value: GenericMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestTransform {
    #[serde(rename = "header-transform")]
    #[serde(skip_serializing_if = "Option::is_none")]
    xform: Option<TypedHeaderTransform>,

    #[serde(skip_serializing_if = "Option::is_none")]
    uri: Option<String>,

    #[serde(rename = "uri-is-expression")]
    #[serde(skip_serializing_if = "Option::is_none")]
    uri_is_expr: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedRequestTransform {
    #[serde(rename = "generic-metadata-type")]
    tpe: String,
    #[serde(rename = "generic-metadata-value")]
    value: RequestTransform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseTransform {
    #[serde(rename = "header-transform")]
    #[serde(skip_serializing_if = "Option::is_none")]
    xform: Option<TypedHeaderTransform>,

    #[serde(rename = "response-status")]
    #[serde(skip_serializing_if = "Option::is_none")]
    response_status: Option<String>,

    #[serde(rename = "status-is-expression")]
    #[serde(skip_serializing_if = "Option::is_none")]
    response_status_expr: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    synthetic: Option<TypedSyntheticResponse>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedResponseTransform {
    #[serde(rename = "generic-metadata-type")]
    tpe: String,
    #[serde(rename = "generic-metadata-value")]
    value: ResponseTransform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageMetadata {
    #[serde(rename = "generic-metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    generic: Option<Vec<TypedGenericMetadata>>,

    #[serde(rename = "request-transform")]
    #[serde(skip_serializing_if = "Option::is_none")]
    request_xform: Option<TypedRequestTransform>,

    #[serde(rename = "response-transform")]
    #[serde(skip_serializing_if = "Option::is_none")]
    response_xform: Option<TypedResponseTransform>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedStageMetadata {
    #[serde(rename = "generic-metadata-type")]
    tpe: String,
    #[serde(rename = "generic-metadata-value")]
    value: StageMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpressionMatch {
    expression: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedExpressionMatch {
    #[serde(rename = "generic-metadata-type")]
    tpe: String,
    #[serde(rename = "generic-metadata-value")]
    value: ExpressionMatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageRules {
    #[serde(rename = "match")]
    #[serde(skip_serializing_if = "Option::is_none")]
    mtch: Option<TypedExpressionMatch>,
    #[serde(rename = "stage-metadata")]
    stage_metadata: TypedStageMetadata,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedStageRules {
    #[serde(rename = "generic-metadata-type")]
    tpe: String,
    #[serde(rename = "generic-metadata-value")]
    value: StageRules,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStages {
    #[serde(rename = "client-request")]
    client_req: Vec<TypedStageRules>,
    #[serde(rename = "origin-request")]
    origin_req: Vec<TypedStageRules>,
    #[serde(rename = "origin-response")]
    origin_res: Vec<TypedStageRules>,
    #[serde(rename = "client-response")]
    client_res: Vec<TypedStageRules>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedProcessingStages {
    #[serde(rename = "generic-metadata-type")]
    tpe: String,
    #[serde(rename = "generic-metadata-value")]
    value: ProcessingStages,
}

#[cfg(test)]
mod test_spec {
    use std::{fs::OpenOptions, io::Read, path::Path};

    use crate::cdni::spec::{
        ExpressionMatch, ProcessingStages, StageMetadata, StageRules, TypedExpressionMatch,
        TypedProcessingStages, TypedStageMetadata, TypedStageRules,
    };

    fn read_test_file(path: &Path) -> String {
        let mut contents: Vec<u8> = vec![];
        OpenOptions::new()
            .read(true)
            .open(path)
            .expect("Could not open test file.")
            .read_to_end(&mut contents)
            .expect("Could not read string from test file");

        String::from_utf8(contents).expect("Could not convert source to string")
    }

    fn expression_match(expression: &str) -> TypedExpressionMatch {
        TypedExpressionMatch {
            tpe: "MI.ExpressionMatch".to_string(),
            value: ExpressionMatch {
                expression: expression.to_string(),
            },
        }
    }

    fn stage_metadata() -> TypedStageMetadata {
        TypedStageMetadata {
            tpe: "MI.StageMetadata".to_string(),
            value: StageMetadata {
                generic: None,
                request_xform: None,
                response_xform: None,
            },
        }
    }

    fn stage_rule(
        expression: Option<TypedExpressionMatch>,
        md: TypedStageMetadata,
    ) -> TypedStageRules {
        TypedStageRules {
            tpe: "MI.StageRules".to_string(),
            value: StageRules {
                mtch: expression,
                stage_metadata: md,
            },
        }
    }

    #[test]
    fn test_simple_serialize() {
        let x = TypedProcessingStages {
            tpe: "MI.ProcessingStages".to_string(),
            value: ProcessingStages {
                client_req: vec![stage_rule(
                    Some(expression_match("5 + 4")),
                    stage_metadata(),
                )],
                origin_req: vec![stage_rule(
                    Some(expression_match("5 + 4")),
                    stage_metadata(),
                )],
                origin_res: vec![stage_rule(
                    Some(expression_match("5 + 4")),
                    stage_metadata(),
                )],
                client_res: vec![stage_rule(
                    Some(expression_match("5 + 4")),
                    stage_metadata(),
                )],
            },
        };
        let expected = read_test_file(Path::new("./src/cdni/tests/simple_serialize.json"));
        let actual = serde_json::to_string_pretty(&x).expect("Could not serialize");
        pretty_assertions::assert_eq!(expected, actual);
    }

    #[test]
    fn test_deserialize_example8() {
        let json = read_test_file(Path::new("./src/cdni/tests/example8.json"));
        assert!(serde_json::from_str::<TypedProcessingStages>(&json).is_ok())
    }
}
