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

//! Serializer/Deserializer of CDNI JSON:
//!
//! This is an implementation of the [CDNI ProcessingStages
//! Metadata](https://datatracker.ietf.org/doc/draft-ietf-cdni-processing-stages-metadata/)
//! spec.
//!
//! ## Implementation Notes:
//!
//! 1. There should be a `TypedXXX` for every `XXX`.
//! 2. The `TypedXXX` should have `tpe` and `value`, where
//!    `value` has the `XXX` type.
//! 3. Use `TypedGenericMetadata` on the `TypedXXX` version
//!    to get `typed_value` and `typed_generic_metadata_name`
//!    to help the parser verify the JSON.
//! 4. The name of the type is `MI.XXX` (where `XXX` is from above).
//! 5. The `XXX` should have a `pub` `aug` whose type is `A: Debug + Clone + Default`
//!    which will hold any augmentation data generated during
//!    verification. Annotate it as `serde(skip)`.
//! 6. Use `serde(rename=...)` to rename fields to match the spec.
//!

use brooks_macros::TypedGenericMetadata;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::fmt::Debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticResponse<A: Debug + Clone + Default> {
    pub headers: Option<Vec<TypedHeader<A>>>,

    #[serde(rename = "response-status")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_status: Option<String>,

    #[serde(rename = "status-is-expression")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_status_expr: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,

    #[serde(rename = "body-is-expression")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_expr: Option<bool>,

    #[serde(skip)]
    pub aug: A,
}

#[derive(Debug, Clone, Serialize, Deserialize, TypedGenericMetadata)]
pub struct TypedSyntheticResponse<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: SyntheticResponse<A>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header<A: Debug + Clone + Default> {
    pub name: String,
    pub value: String,
    #[serde(rename = "value-is-expression")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_expr: Option<bool>,

    #[serde(skip)]
    pub aug: A,
}
#[derive(Debug, Clone, Serialize, Deserialize, TypedGenericMetadata)]
pub struct TypedHeader<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: Header<A>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HeaderTransform<A: Debug + Clone + Default> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add: Option<Vec<TypedHeader<A>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace: Option<Vec<TypedHeader<A>>>,
    #[serde(skip)]
    pub aug: A,
}
#[derive(Debug, Clone, Serialize, Deserialize, TypedGenericMetadata)]
pub struct TypedHeaderTransform<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: HeaderTransform<A>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedGenericMetadata<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: Value,
    #[serde(skip)]
    pub aug: A,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestTransform<A: Debug + Clone + Default> {
    #[serde(rename = "header-transform")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xform: Option<TypedHeaderTransform<A>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,

    #[serde(rename = "uri-is-expression")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri_is_expr: Option<bool>,

    #[serde(skip_serializing, skip_deserializing)]
    pub aug: A,
}
#[derive(Debug, Clone, Serialize, Deserialize, TypedGenericMetadata)]
pub struct TypedRequestTransform<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: RequestTransform<A>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseTransform<A: Debug + Clone + Default> {
    #[serde(rename = "header-transform")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xform: Option<TypedHeaderTransform<A>>,

    #[serde(rename = "response-status")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_status: Option<String>,

    #[serde(rename = "status-is-expression")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_status_expr: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthetic: Option<TypedSyntheticResponse<A>>,

    #[serde(skip)]
    pub aug: A,
}
#[derive(Debug, Clone, Serialize, Deserialize, TypedGenericMetadata)]
pub struct TypedResponseTransform<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: ResponseTransform<A>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageMetadata<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generic: Option<Vec<TypedGenericMetadata<A>>>,

    #[serde(rename = "request-transform")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_xform: Option<TypedRequestTransform<A>>,

    #[serde(rename = "response-transform")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_xform: Option<TypedResponseTransform<A>>,

    #[serde(skip)]
    pub aug: A,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default, TypedGenericMetadata)]
pub struct TypedStageMetadata<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: StageMetadata<A>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpressionMatch<A: Debug + Clone + Default> {
    pub expression: String,
    #[serde(skip)]
    pub aug: A,
}
#[derive(Debug, Clone, Serialize, Deserialize, TypedGenericMetadata)]
pub struct TypedExpressionMatch<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: ExpressionMatch<A>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageRules<A: Debug + Clone + Default> {
    #[serde(rename = "match")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtch: Option<TypedExpressionMatch<A>>,
    #[serde(rename = "stage-metadata")]
    pub stage_metadata: TypedStageMetadata<A>,

    #[serde(skip)]
    pub aug: A,
}
#[derive(Debug, Clone, Serialize, Deserialize, TypedGenericMetadata)]
pub struct TypedStageRules<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: StageRules<A>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchGroup<A: Debug + Clone + Default> {
    #[serde(rename = "if-rule")]
    pub if_rule: TypedStageRules<A>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "else-if-rules")]
    pub else_ifs: Option<Vec<TypedStageRules<A>>>,
    #[serde(skip)]
    pub aug: A,
}
#[derive(Debug, Clone, Serialize, Deserialize, TypedGenericMetadata)]
pub struct TypedMatchGroup<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: MatchGroup<A>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRequestStage<A: Debug + Clone + Default> {
    #[serde(rename = "match-groups")]
    pub match_groups: Vec<TypedMatchGroup<A>>,
    #[serde(skip)]
    pub aug: A,
}
#[derive(Debug, Clone, Serialize, Deserialize, TypedGenericMetadata)]
pub struct TypedClientRequestStage<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: ClientRequestStage<A>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessingStages<A: Debug + Clone + Default> {
    #[serde(rename = "client-request")]
    pub client_req: Vec<TypedStageRules<A>>,
    #[serde(rename = "origin-request")]
    pub origin_req: Vec<TypedStageRules<A>>,
    #[serde(rename = "origin-response")]
    pub origin_res: Vec<TypedStageRules<A>>,
    #[serde(rename = "client-response")]
    pub client_res: Vec<TypedStageRules<A>>,

    #[serde(skip)]
    pub aug: A,
}
#[derive(Debug, Clone, Serialize, Deserialize, TypedGenericMetadata)]
pub struct TypedProcessingStages<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: ProcessingStages<A>,
}

#[cfg(test)]
mod test_spec {
    use std::path::Path;

    use crate::{
        ps::{
            spec::{ProcessingStages, TypedClientRequestStage, TypedProcessingStages},
            tests::test_helpers::{expression_match, stage_metadata, stage_rule},
        },
        tests::read_test_file,
    };

    #[test]
    fn test_simple_serialize() {
        let x = TypedProcessingStages::<()> {
            tpe: "MI.ProcessingStages".to_string(),
            value: ProcessingStages::<()> {
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
                aug: (),
            },
        };
        let expected = read_test_file(Path::new("./src/ps/tests/simple/serialize.json"));
        let actual = serde_json::to_string_pretty(&x).expect("Could not serialize");
        pretty_assertions::assert_eq!(expected, actual);
    }

    #[test]
    fn test_deserialize_example8() {
        let json = read_test_file(Path::new("./src/ps/tests/from-spec/example8.json"));

        let result = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not deserialize Example 8");

        // There is one client request stage rules ...
        assert_eq!(result.value.client_req.len(), 1);
        let client_req = &result.value.client_req[0];

        // ... and it has a match ...
        assert!(client_req.value.mtch.is_some());
        // ... and it requests application of a piece of metadata ...
        assert!(client_req.value.stage_metadata.value.generic.is_some());
        // ... and, finally, it has a response transform.
        assert!(
            client_req
                .value
                .stage_metadata
                .value
                .response_xform
                .is_some()
        );
    }

    #[test]
    fn test_deserialize_client_request_stage() {
        let json = read_test_file(Path::new("./src/ps/tests/client_request_stage/if.json"));

        let result = serde_json::from_str::<TypedClientRequestStage<()>>(&json)
            .expect("Could not deserialize simple client request stage JSON");

        // There is one client request stage rules ...
        assert_eq!(result.value.match_groups.len(), 1);
        let mg = &result.value.match_groups[0];

        // ... and it has a match ...
        assert!(mg.value.if_rule.value.mtch.is_some());
    }
}
