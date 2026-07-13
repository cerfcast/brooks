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

//! Verification of CDNI JSON

use serde::Serialize;

use crate::{
    mel::{
        analysis::{Analyzed, MelAnalysisLocatableError, analyze},
        ast::Expr,
        compiler::{self, compile::CompilerError},
        scope::Scopes,
        tvs::Type,
    },
    ps::{
        spec::{
            ClientRequestStage, ExpressionMatch, Header, HeaderTransform, MatchGroup,
            ProcessingStages, RequestTransform, ResponseTransform, StageMetadata, StageRules,
            SyntheticResponse, TypedClientRequestStage, TypedExpressionMatch, TypedGenericMetadata,
            TypedHeader, TypedHeaderTransform, TypedMatchGroup, TypedProcessingStages,
            TypedRequestTransform, TypedResponseTransform, TypedStageMetadata, TypedStageRules,
            TypedSyntheticResponse,
        },
        visit::CdniVisitor,
    },
};

use std::fmt::Debug;

type CdniVisitorResult<T, E> = Result<T, E>;

#[derive(Debug, Clone, Default)]
pub enum CdniVerificationError {
    #[default]
    NoError,
    WrongType,
    WrongGenericMetadataTypeName(String, String),
    NoVerifiedValue,
    ExpressionCompile(CompilerError),
    ExpressionAnalyze(MelAnalysisLocatableError),
    ExpressionWrongType(Type, Type),
}

#[derive(Debug, Clone, Default)]
pub enum CdniVerificationKey {
    Expr(Expr<Analyzed>),
    ExprPair(Option<Expr<Analyzed>>, Option<Expr<Analyzed>>),
    #[default]
    None,
}

impl Serialize for CdniVerificationKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_none()
    }
}

#[derive(Debug, Clone, Default)]
struct CdniVerifier {}

impl CdniVerifier {
    #[allow(clippy::result_large_err)]
    pub fn compile_and_analyze_expr(source: &str) -> Result<Expr<Analyzed>, CdniVerificationError> {
        let expr = compiler::compile(source).map_err(CdniVerificationError::ExpressionCompile)?;
        analyze(&expr, &Scopes::default()).map_err(CdniVerificationError::ExpressionAnalyze)
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
enum CdniVerifierContextValue {
    ProcessingStages(TypedProcessingStages<CdniVerificationKey>),
    StageRules(TypedStageRules<CdniVerificationKey>),
    ExpressionMatch(TypedExpressionMatch<CdniVerificationKey>),
    StageMetadata(TypedStageMetadata<CdniVerificationKey>),
    GenericMetadata(TypedGenericMetadata<CdniVerificationKey>),
    RequestTransform(TypedRequestTransform<CdniVerificationKey>),
    ResponseTransform(TypedResponseTransform<CdniVerificationKey>),
    HeaderTransform(TypedHeaderTransform<CdniVerificationKey>),
    Header(TypedHeader<CdniVerificationKey>),
    SyntheticResponse(TypedSyntheticResponse<CdniVerificationKey>),
    MatchGroup(TypedMatchGroup<CdniVerificationKey>),
    ClientRequestStage(TypedClientRequestStage<CdniVerificationKey>),
}

#[derive(Debug, Clone, Default)]
struct CdniVerifierContext {
    value: Option<CdniVerifierContextValue>,
}

type VerifiedCdni = ProcessingStages<CdniVerificationKey>;

macro_rules! expect_maybe_some_value {
    ($name:expr, $value:path) => {
        match &$name {
            Some($value(v)) => Some(v),
            None => None,
            _ => return Err(CdniVerificationError::WrongType),
        }
    };
}

macro_rules! expect_some_value {
    ($name:expr, $value:path) => {
        match &$name {
            Some($value(v)) => v,
            None => return Err(CdniVerificationError::NoVerifiedValue),
            _ => return Err(CdniVerificationError::WrongType),
        }
    };
}

macro_rules! make_context_value {
    ($value:expr, $vt:path, $a:ident, $tvt:ident) => {
        Some($vt($tvt::<$a>::typed_value($value)))
    };
}

macro_rules! check_generic_md_typename {
    ($value:expr, $tn:ident) => {
        if $value.tpe != $tn::<()>::typed_generic_metadata_name() {
            return Err(CdniVerificationError::WrongGenericMetadataTypeName(
                $tn::<()>::typed_generic_metadata_name(),
                $value.tpe.clone(),
            ));
        }
    };
}

impl CdniVisitor<(), CdniVerifierContext, CdniVerificationError> for CdniVerifier {
    fn visit_processing_stages(
        &self,
        v: &TypedProcessingStages<()>,
        c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        check_generic_md_typename!(v, TypedProcessingStages);

        let mut result = ProcessingStages::<CdniVerificationKey>::default();
        let mut rc = c.clone();

        let v = &v.value;

        for csr in &v.client_req {
            rc = self.visit_stage_rules(csr, &rc)?;
            if let Some(tsr) =
                expect_maybe_some_value!(rc.value, CdniVerifierContextValue::StageRules)
            {
                result.client_req.push(tsr.clone());
            };
        }

        for csr in &v.client_res {
            rc = self.visit_stage_rules(csr, &rc)?;
            if let Some(tsr) =
                expect_maybe_some_value!(rc.value, CdniVerifierContextValue::StageRules)
            {
                result.client_res.push(tsr.clone());
            };
        }

        for csr in &v.origin_req {
            rc = self.visit_stage_rules(csr, &rc)?;
            if let Some(tsr) =
                expect_maybe_some_value!(rc.value, CdniVerifierContextValue::StageRules)
            {
                result.origin_req.push(tsr.clone());
            };
        }

        for csr in &v.origin_res {
            rc = self.visit_stage_rules(csr, &rc)?;
            if let Some(tsr) =
                expect_maybe_some_value!(rc.value, CdniVerifierContextValue::StageRules)
            {
                result.origin_res.push(tsr.clone());
            };
        }

        rc.value = make_context_value!(
            result,
            CdniVerifierContextValue::ProcessingStages,
            CdniVerificationKey,
            TypedProcessingStages
        );

        Ok(rc.clone())
    }

    fn visit_stage_rules(
        &self,
        v: &TypedStageRules<()>,
        c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        // Check whether the expression has the right type!

        check_generic_md_typename!(v, TypedStageRules);

        let v = &v.value;

        let mut result = if let Some(mtch) = &v.mtch {
            let expr = self.visit_expression_match(mtch, &c.clone())?;

            let expr =
                expect_maybe_some_value!(&expr.value, CdniVerifierContextValue::ExpressionMatch);
            StageRules::<CdniVerificationKey> {
                mtch: expr.cloned(),
                stage_metadata: TypedStageMetadata::default(),
                aug: CdniVerificationKey::None,
            }
        } else {
            StageRules::<CdniVerificationKey> {
                mtch: None,
                stage_metadata: TypedStageMetadata::default(),
                aug: CdniVerificationKey::None,
            }
        };

        result.stage_metadata = expect_some_value!(
            &self
                .visit_stage_metadata(&v.stage_metadata, &c.clone())?
                .value,
            CdniVerifierContextValue::StageMetadata
        )
        .clone();

        Ok(CdniVerifierContext {
            value: make_context_value!(
                result,
                CdniVerifierContextValue::StageRules,
                CdniVerificationKey,
                TypedStageRules
            ),
        })
    }

    fn visit_expression_match(
        &self,
        v: &TypedExpressionMatch<()>,
        _c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        check_generic_md_typename!(v, TypedExpressionMatch);

        let v = &v.value;

        let expr = Self::compile_and_analyze_expr(&v.expression)?;

        if expr.tipe() != Type::Boolean {
            return Err(CdniVerificationError::ExpressionWrongType(
                Type::Boolean,
                expr.tipe(),
            ));
        }
        Ok(CdniVerifierContext {
            value: make_context_value!(
                ExpressionMatch {
                    expression: v.expression.clone(),
                    aug: CdniVerificationKey::Expr(expr),
                },
                CdniVerifierContextValue::ExpressionMatch,
                CdniVerificationKey,
                TypedExpressionMatch
            ),
        })
    }

    fn visit_stage_metadata(
        &self,
        v: &TypedStageMetadata<()>,
        c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        check_generic_md_typename!(v, TypedStageMetadata);

        let v = &v.value;

        let mut result = if let Some(generic) = &v.generic {
            let mut result: Vec<TypedGenericMetadata<CdniVerificationKey>> = vec![];
            for generics in generic {
                let generic = expect_some_value!(
                    self.visit_generic_metadata(generics, &c.clone())?.value,
                    CdniVerifierContextValue::GenericMetadata
                )
                .clone();
                result.push(generic);
            }

            StageMetadata {
                request_xform: None,
                response_xform: None,
                generic: Some(result),
                aug: CdniVerificationKey::None,
            }
        } else {
            StageMetadata {
                request_xform: None,
                response_xform: None,
                generic: None,
                aug: CdniVerificationKey::None,
            }
        };

        result.request_xform = if let Some(reqt) = &v.request_xform {
            Some(
                expect_some_value!(
                    self.visit_request_transform(reqt, &c.clone())?.value,
                    CdniVerifierContextValue::RequestTransform
                )
                .clone(),
            )
        } else {
            None
        };

        result.response_xform = if let Some(reqt) = &v.response_xform {
            Some(
                expect_some_value!(
                    self.visit_response_transform(reqt, &c.clone())?.value,
                    CdniVerifierContextValue::ResponseTransform
                )
                .clone(),
            )
        } else {
            None
        };

        Ok(CdniVerifierContext {
            value: make_context_value!(
                result,
                CdniVerifierContextValue::StageMetadata,
                CdniVerificationKey,
                TypedStageMetadata
            ),
        })
    }

    fn visit_request_transform(
        &self,
        v: &TypedRequestTransform<()>,
        c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        check_generic_md_typename!(v, TypedRequestTransform);

        let v = &v.value;

        let mut result = if let Some(header_xform) = &v.xform {
            let header = Some(
                expect_some_value!(
                    self.visit_header_transform(header_xform, &c.clone())?.value,
                    CdniVerifierContextValue::HeaderTransform
                )
                .clone(),
            );

            RequestTransform {
                xform: header,
                uri: v.uri.clone(),
                uri_is_expr: v.uri_is_expr,
                aug: CdniVerificationKey::None,
            }
        } else {
            RequestTransform {
                xform: None,
                uri: v.uri.clone(),
                uri_is_expr: v.uri_is_expr,
                aug: CdniVerificationKey::None,
            }
        };

        result.aug = if let Some(uri) = &v.uri {
            if let Some(uri_is_expr) = &v.uri_is_expr
                && *uri_is_expr
            {
                let expr = Self::compile_and_analyze_expr(uri)?;

                if expr.tipe() != Type::String {
                    return Err(CdniVerificationError::ExpressionWrongType(
                        Type::String,
                        expr.tipe(),
                    ));
                }
                CdniVerificationKey::Expr(expr)
            } else {
                CdniVerificationKey::None
            }
        } else {
            CdniVerificationKey::None
        };

        Ok(CdniVerifierContext {
            value: make_context_value!(
                result,
                CdniVerifierContextValue::RequestTransform,
                CdniVerificationKey,
                TypedRequestTransform
            ),
        })
    }

    fn visit_response_transform(
        &self,
        v: &TypedResponseTransform<()>,
        c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        check_generic_md_typename!(v, TypedResponseTransform);

        let v = &v.value;

        let mut result = if let Some(header_xform) = &v.xform {
            let header = Some(
                expect_some_value!(
                    self.visit_header_transform(header_xform, &c.clone())?.value,
                    CdniVerifierContextValue::HeaderTransform
                )
                .clone(),
            );

            ResponseTransform {
                xform: header,
                response_status: v.response_status.clone(),
                response_status_expr: v.response_status_expr,
                synthetic: None,
                aug: CdniVerificationKey::None,
            }
        } else {
            ResponseTransform {
                xform: None,
                response_status: v.response_status.clone(),
                response_status_expr: v.response_status_expr,
                synthetic: None,
                aug: CdniVerificationKey::None,
            }
        };

        result.synthetic = if let Some(synthetic) = &v.synthetic {
            Some(
                expect_some_value!(
                    self.visit_synthetic_response(synthetic, &c.clone())?.value,
                    CdniVerifierContextValue::SyntheticResponse
                )
                .clone(),
            )
        } else {
            None
        };

        result.aug = if let Some(rs) = &v.response_status {
            if let Some(rs_is_expr) = &v.response_status_expr
                && *rs_is_expr
            {
                let expr = Self::compile_and_analyze_expr(rs)?;

                if expr.tipe() != Type::Integer {
                    return Err(CdniVerificationError::ExpressionWrongType(
                        Type::Integer,
                        expr.tipe(),
                    ));
                }
                CdniVerificationKey::Expr(expr)
            } else {
                CdniVerificationKey::None
            }
        } else {
            CdniVerificationKey::None
        };

        Ok(CdniVerifierContext {
            value: make_context_value!(
                result,
                CdniVerifierContextValue::ResponseTransform,
                CdniVerificationKey,
                TypedResponseTransform
            ),
        })
    }

    fn visit_generic_metadata(
        &self,
        v: &TypedGenericMetadata<()>,
        _c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        // We only verify that the type starts with "MI.".
        if !v.tpe.starts_with("MI.") {
            return Err(CdniVerificationError::WrongGenericMetadataTypeName(
                "MI. ...".to_string(),
                v.tpe.clone(),
            ));
        }

        Ok(CdniVerifierContext {
            value: Some(CdniVerifierContextValue::GenericMetadata(
                TypedGenericMetadata {
                    tpe: v.tpe.clone(),
                    value: v.value.clone(),
                    aug: CdniVerificationKey::None,
                },
            )),
        })
    }

    fn visit_header_transform(
        &self,
        v: &TypedHeaderTransform<()>,
        _c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        check_generic_md_typename!(v, TypedHeaderTransform);
        let v = &v.value;

        let mut result = HeaderTransform {
            delete: v.delete.clone(),
            ..Default::default()
        };

        result.add = if let Some(adds) = &v.add {
            let mut verified_adds: Vec<TypedHeader<CdniVerificationKey>> = vec![];
            for add in adds {
                verified_adds.push(
                    expect_some_value!(
                        self.visit_header(add, &_c.clone())?.value,
                        CdniVerifierContextValue::Header
                    )
                    .clone(),
                );
            }
            Some(verified_adds)
        } else {
            None
        };

        result.replace = if let Some(replaceds) = &v.replace {
            let mut verified_replaceds: Vec<TypedHeader<CdniVerificationKey>> = vec![];
            for replaced in replaceds {
                verified_replaceds.push(
                    expect_some_value!(
                        self.visit_header(replaced, &_c.clone())?.value,
                        CdniVerifierContextValue::Header
                    )
                    .clone(),
                );
            }
            Some(verified_replaceds)
        } else {
            None
        };
        Ok(CdniVerifierContext {
            value: make_context_value!(
                result,
                CdniVerifierContextValue::HeaderTransform,
                CdniVerificationKey,
                TypedHeaderTransform
            ),
        })
    }

    fn visit_header(
        &self,
        v: &TypedHeader<()>,
        _c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        check_generic_md_typename!(v, TypedHeader);

        let v = &v.value;
        let mut result = Header::<CdniVerificationKey> {
            name: v.name.clone(),
            value: v.value.clone(),
            value_expr: v.value_expr,
            aug: CdniVerificationKey::None,
        };

        result.aug = if let Some(hv_is_expr) = &v.value_expr
            && *hv_is_expr
        {
            let expr = Self::compile_and_analyze_expr(&v.value)?;

            if expr.tipe() != Type::String {
                return Err(CdniVerificationError::ExpressionWrongType(
                    Type::String,
                    expr.tipe(),
                ));
            }
            CdniVerificationKey::Expr(expr)
        } else {
            CdniVerificationKey::None
        };

        Ok(CdniVerifierContext {
            value: make_context_value!(
                result,
                CdniVerifierContextValue::Header,
                CdniVerificationKey,
                TypedHeader
            ),
        })
    }

    fn visit_synthetic_response(
        &self,
        v: &TypedSyntheticResponse<()>,
        _c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        check_generic_md_typename!(v, TypedSyntheticResponse);
        let v = &v.value;

        let mut result = SyntheticResponse {
            headers: None,
            response_status: v.response_status.clone(),
            response_status_expr: v.response_status_expr,
            body: v.body.clone(),
            body_expr: v.body_expr,
            aug: CdniVerificationKey::None,
        };

        result.headers = if let Some(headers) = &v.headers {
            let mut verified_headers: Vec<TypedHeader<CdniVerificationKey>> = vec![];
            for header in headers {
                verified_headers.push(
                    expect_some_value!(
                        self.visit_header(header, &_c.clone())?.value,
                        CdniVerifierContextValue::Header
                    )
                    .clone(),
                );
            }
            Some(verified_headers)
        } else {
            None
        };

        let response_status_expr = if let Some(rs) = &v.response_status {
            if let Some(rs_is_expr) = &v.response_status_expr
                && *rs_is_expr
            {
                let expr = Self::compile_and_analyze_expr(rs)?;

                if expr.tipe() != Type::Integer {
                    return Err(CdniVerificationError::ExpressionWrongType(
                        Type::Integer,
                        expr.tipe(),
                    ));
                }
                Some(expr)
            } else {
                None
            }
        } else {
            None
        };

        let body_expr = if let Some(body) = &v.body {
            if let Some(body_is_expr) = &v.body_expr
                && *body_is_expr
            {
                let expr = Self::compile_and_analyze_expr(body)?;

                if expr.tipe() != Type::String {
                    return Err(CdniVerificationError::ExpressionWrongType(
                        Type::String,
                        expr.tipe(),
                    ));
                }
                Some(expr)
            } else {
                None
            }
        } else {
            None
        };

        result.aug = CdniVerificationKey::ExprPair(response_status_expr, body_expr);

        Ok(CdniVerifierContext {
            value: make_context_value!(
                result,
                CdniVerifierContextValue::SyntheticResponse,
                CdniVerificationKey,
                TypedSyntheticResponse
            ),
        })
    }

    fn visit_client_request_stage(
        &self,
        v: &TypedClientRequestStage<()>,
        c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        check_generic_md_typename!(v, TypedClientRequestStage);
        let v = &v.value;

        // Verify each of the match groups!
        let mut verified_mgs: Vec<TypedMatchGroup<CdniVerificationKey>> = vec![];
        for mg in &v.match_groups {
            verified_mgs.push(
                expect_some_value!(
                    self.visit_match_group(mg, &c.clone())?.value,
                    CdniVerifierContextValue::MatchGroup
                )
                .clone(),
            );
        }

        let result = ClientRequestStage {
            match_groups: verified_mgs,
            aug: CdniVerificationKey::None,
        };

        Ok(CdniVerifierContext {
            value: make_context_value!(
                result,
                CdniVerifierContextValue::ClientRequestStage,
                CdniVerificationKey,
                TypedClientRequestStage
            ),
        })
    }

    fn visit_match_group(
        &self,
        v: &TypedMatchGroup<()>,
        c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        check_generic_md_typename!(v, TypedMatchGroup);
        let v = &v.value;

        let verified_if_rule = expect_some_value!(
            self.visit_stage_rules(&v.if_rule, &c.clone())?.value,
            CdniVerifierContextValue::StageRules
        )
        .clone();

        let result = if let Some(else_if_rules) = &v.else_ifs {
            // Verify each of the match groups!
            let mut verified_eirs: Vec<TypedStageRules<CdniVerificationKey>> = vec![];
            for eir in else_if_rules {
                verified_eirs.push(
                    expect_some_value!(
                        self.visit_stage_rules(eir, &c.clone())?.value,
                        CdniVerifierContextValue::StageRules
                    )
                    .clone(),
                );
            }
            MatchGroup {
                if_rule: verified_if_rule,
                else_ifs: Some(verified_eirs),
                aug: CdniVerificationKey::None,
            }
        } else {
            MatchGroup {
                if_rule: verified_if_rule,
                else_ifs: None,
                aug: CdniVerificationKey::None,
            }
        };
        Ok(CdniVerifierContext {
            value: make_context_value!(
                result,
                CdniVerifierContextValue::MatchGroup,
                CdniVerificationKey,
                TypedMatchGroup
            ),
        })
    }
}

fn verifier() -> (CdniVerifier, CdniVerifierContext) {
    (CdniVerifier {}, CdniVerifierContext::default())
}

/// Verify the (semantic) validity of some CDNI JSON
///
/// In particular, test whether
/// 1. the MEL expressions have the proper types, and
/// 2. the generic metadata type names are correct.
///
#[allow(clippy::result_large_err)]
pub fn verify_cdni(
    stages: &TypedProcessingStages<()>,
) -> Result<TypedProcessingStages<CdniVerificationKey>, CdniVerificationError> {
    let (verifier, context) = verifier();
    let value = &stages;
    let result = verifier.visit_processing_stages(value, &context)?;
    Ok(expect_some_value!(&result.value, CdniVerifierContextValue::ProcessingStages).clone())
}

#[cfg(test)]
mod test_verify {
    use std::assert_matches;

    use crate::mel::tvs::Type;
    use crate::ps::spec::{
        HeaderTransform, ResponseTransform, SyntheticResponse, TypedHeaderTransform,
        TypedSyntheticResponse,
    };
    use crate::ps::verify::{CdniVerifierContextValue, verifier};
    use crate::ps::visit::CdniVisitor;
    use crate::{
        mel::ast::Expr::BinaryExpr,
        ps::{
            spec::TypedResponseTransform,
            verify::{
                CdniVerificationError::{ExpressionWrongType, WrongGenericMetadataTypeName},
                CdniVerificationKey, verify_cdni,
            },
        },
    };

    use crate::ps::tests::test_helpers::{
        expression_match, generic_metadata, header_transform, processing_stages, request_transform,
        response_transform, synthetic_response, typed_header,
    };

    #[test]
    fn test_verify_bad_generic_md_typename() {
        let mut stages = processing_stages(vec![], vec![], vec![], vec![]);
        stages.tpe = "MI.ProcessigStages".to_string();

        let result = verify_cdni(&stages)
            .expect_err("Could verify CDNI generic metadata with invalid metadata type name");

        assert_matches!(result, WrongGenericMetadataTypeName(expected, actual) if expected == "MI.ProcessingStages" && actual == "MI.ProcessigStages")
    }

    #[test]
    fn test_verify_generic_metadata_with_bad_type_name() {
        let generic = generic_metadata("Mi.CachePolicy", None);

        let (verifier, context) = verifier();
        let result = verifier
            .visit_generic_metadata(&generic, &context)
            .expect_err("Could verify CDNI generic metadata with invalid metadata type name");

        assert_matches!(result, WrongGenericMetadataTypeName(expected, actual) if expected == "MI. ..." && actual == "Mi.CachePolicy");
    }

    #[test]
    fn test_verify_match_wrong_type() {
        let mtch = expression_match("5 + 4");
        let (verifier, context) = verifier();
        let result = verifier
            .visit_expression_match(&mtch, &context)
            .expect_err("Could verify CDNI expression match with incorrect MEL type");

        assert_matches!(result, ExpressionWrongType(Type::Boolean, Type::Integer))
    }

    #[test]
    fn test_verify_request_transform_uri_expr_wrong_type() {
        let xform = request_transform(None, Some("5+4".to_string()), Some(true));
        let (verifier, context) = verifier();
        let result = verifier
            .visit_request_transform(&xform, &context)
            .expect_err("Could verify CDNI with incorrect MEL type of expression to calculate URI");

        assert_matches!(result, ExpressionWrongType(Type::String, Type::Integer));
    }

    #[test]
    fn test_verify_response_transform() {
        let xform = response_transform(None, Some("404".to_string()), None, None);
        let (verifier, context) = verifier();
        let result = verifier
            .visit_response_transform(&xform, &context)
            .expect("Could not verify correct CDNI")
            .value
            .expect("No value in CDNI Verification Context");

        assert_matches!(
            result,
            CdniVerifierContextValue::ResponseTransform(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: _,
                    response_status: _,
                    response_status_expr: None,
                    synthetic: None,
                    aug: CdniVerificationKey::None
                }
            })
        );
    }

    #[test]
    fn test_verify_response_transform_rs_expr() {
        let xform = response_transform(None, Some("400+4".to_string()), Some(true), None);
        let (verifier, context) = verifier();
        let result = verifier
            .visit_response_transform(&xform, &context)
            .expect("Could not verify correct CDNI")
            .value
            .expect("No value in CDNI Verification Context");

        assert_matches!(
            result,
            CdniVerifierContextValue::ResponseTransform(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: _,
                    response_status: _,
                    response_status_expr: Some(true),
                    synthetic: None,
                    aug: CdniVerificationKey::Expr(BinaryExpr(_))
                }
            })
        );
    }
    #[test]
    fn test_verify_response_transform_rs_expr_wrong_type() {
        let xform = response_transform(None, Some("false".to_string()), Some(true), None);
        let (verifier, context) = verifier();
        let result = verifier
            .visit_response_transform(&xform, &context)
            .expect_err("Could verify CDNI response transform with incorrect MEL type of expression to calculate response status");

        assert_matches!(result, ExpressionWrongType(Type::Integer, Type::Boolean));
    }

    #[test]
    fn test_verify_header_transform() {
        let header_to_add = typed_header("add", "value", None);
        let header_to_replace = typed_header("replace", "value", None);
        let header_to_delete = vec!["delete1".to_string(), "delete2".to_string()];

        let header_xform = header_transform(
            Some(header_to_delete),
            Some(vec![header_to_add]),
            Some(vec![header_to_replace]),
        );

        let (verifier, context) = verifier();
        let result = verifier
            .visit_header_transform(&header_xform, &context)
            .expect("Could not verify correct CDNI")
            .value
            .expect("No value in CDNI Verification Context");

        assert_matches!(
            &result,
            CdniVerifierContextValue::HeaderTransform(TypedHeaderTransform {
                        tpe: _,
                        value: HeaderTransform {
                            delete: Some(d),
                            add: Some(a),
                            replace: Some(b),
                            aug: CdniVerificationKey::None
                        }
                    }) if d.len() == 2 && a.len() == 1 && b.len() == 1
        );
    }

    #[test]
    fn test_verify_header_transform_value_expr() {
        let header_to_add = typed_header("add", "\"value\"", Some(true));
        let header_to_replace = typed_header("replace", "\"testing\" . \"one\"", Some(true));

        let header_xform = header_transform(
            None,
            Some(vec![header_to_add]),
            Some(vec![header_to_replace]),
        );

        let (verifier, context) = verifier();
        let result = verifier
            .visit_header_transform(&header_xform, &context)
            .expect("Could not verify correct CDNI")
            .value
            .expect("No value in CDNI Verification Context");

        assert_matches!(
            &result,
            CdniVerifierContextValue::HeaderTransform(TypedHeaderTransform {
                        tpe: _,
                        value: HeaderTransform {
                            delete: None,
                            add: Some(a),
                            replace: Some(b),
                            aug: CdniVerificationKey::None
                        }
            }) if matches!(a[0].value.aug, CdniVerificationKey::Expr(_)) && matches!(b[0].value.aug, CdniVerificationKey::Expr(_))
        );
    }

    #[test]
    fn test_verify_header_transform_value_expr_wrong_type_add() {
        let header_to_add = typed_header("add", "5", Some(true));
        let header_to_replace = typed_header("replace", "\"testing\"", Some(true));

        let header_xform = header_transform(
            None,
            Some(vec![header_to_add]),
            Some(vec![header_to_replace]),
        );

        let (verifier, context) = verifier();
        let result = verifier
            .visit_header_transform(&header_xform, &context)
            .expect_err("Could verify CDNI typed header with incorrect MEL type of expression to calculate header value");

        assert_matches!(result, ExpressionWrongType(Type::String, Type::Integer));
    }

    #[test]
    fn test_verify_header_transform_value_expr_wrong_type_replace() {
        let header_to_add = typed_header("add", "\"value\"", Some(true));
        let header_to_replace = typed_header("replace", "true", Some(true));

        let header_xform = header_transform(
            None,
            Some(vec![header_to_add]),
            Some(vec![header_to_replace]),
        );

        let (verifier, context) = verifier();
        let result = verifier
            .visit_header_transform(&header_xform, &context)
            .expect_err("Could verify CDNI typed header with incorrect MEL type of expression to calculate header value");

        assert_matches!(result, ExpressionWrongType(Type::String, Type::Boolean));
    }

    #[test]
    fn test_verify_synthetic_response() {
        let header_to_add = typed_header("add", "value", None);
        let header_to_replace = typed_header("replace", "value", None);
        let headers = Some(vec![header_to_add, header_to_replace]);

        let srt = synthetic_response(headers, None, None, None, None);

        let (verifier, context) = verifier();
        let result = verifier
            .visit_synthetic_response(&srt, &context)
            .expect("Could not verify correct CDNI")
            .value
            .expect("No value in CDNI Verification Context");

        assert_matches!(
            &result,
            CdniVerifierContextValue::SyntheticResponse(TypedSyntheticResponse {
                tpe: _,
                value: SyntheticResponse {
                    headers: _,
                    response_status: _,
                    response_status_expr: _,
                    body: _,
                    body_expr: _,
                    aug: CdniVerificationKey::ExprPair(None, None)
                }
            })
        )
    }

    #[test]
    fn test_verify_synthetic_response_response_expr() {
        let header_to_add = typed_header("add", "value", None);
        let header_to_replace = typed_header("replace", "value", None);
        let headers = Some(vec![header_to_add, header_to_replace]);

        let srt = synthetic_response(headers, Some("400 + 4".to_string()), Some(true), None, None);

        let (verifier, context) = verifier();
        let result = verifier
            .visit_synthetic_response(&srt, &context)
            .expect("Could not verify correct CDNI")
            .value
            .expect("No value in CDNI Verification Context");

        assert_matches!(
            &result,
            CdniVerifierContextValue::SyntheticResponse(TypedSyntheticResponse {
                tpe: _,
                value: SyntheticResponse {
                    headers: _,
                    response_status: _,
                    response_status_expr: _,
                    body: _,
                    body_expr: _,
                    aug: CdniVerificationKey::ExprPair(Some(_), None)
                }
            })
        )
    }

    #[test]
    fn test_verify_synthetic_response_body_expr() {
        let header_to_add = typed_header("add", "value", None);
        let header_to_replace = typed_header("replace", "value", None);
        let headers = Some(vec![header_to_add, header_to_replace]);

        let srt = synthetic_response(
            headers,
            None,
            None,
            Some("\"testing\"".to_string()),
            Some(true),
        );

        let (verifier, context) = verifier();
        let result = verifier
            .visit_synthetic_response(&srt, &context)
            .expect("Could not verify correct CDNI")
            .value
            .expect("No value in CDNI Verification Context");

        assert_matches!(
            &result,
            CdniVerifierContextValue::SyntheticResponse(TypedSyntheticResponse {
                tpe: _,
                value: SyntheticResponse {
                    headers: _,
                    response_status: _,
                    response_status_expr: _,
                    body: _,
                    body_expr: _,
                    aug: CdniVerificationKey::ExprPair(None, Some(_))
                }
            })
        )
    }

    #[test]
    fn test_verify_synthetic_response_body_and_response_expr() {
        let header_to_add = typed_header("add", "value", None);
        let header_to_replace = typed_header("replace", "value", None);
        let headers = Some(vec![header_to_add, header_to_replace]);

        let srt = synthetic_response(
            headers,
            Some("400 + 4".to_string()),
            Some(true),
            Some("\"testing\"".to_string()),
            Some(true),
        );

        let (verifier, context) = verifier();
        let result = verifier
            .visit_synthetic_response(&srt, &context)
            .expect("Could not verify correct CDNI")
            .value
            .expect("No value in CDNI Verification Context");

        assert_matches!(
            &result,
            CdniVerifierContextValue::SyntheticResponse(TypedSyntheticResponse {
                tpe: _,
                value: SyntheticResponse {
                    headers: _,
                    response_status: _,
                    response_status_expr: _,
                    body: _,
                    body_expr: _,
                    aug: CdniVerificationKey::ExprPair(Some(_), Some(_))
                }
            })
        )
    }

    #[test]
    fn test_verify_synthetic_response_wrong_response_expr_type() {
        let header_to_add = typed_header("add", "value", None);
        let header_to_replace = typed_header("replace", "value", None);
        let headers = Some(vec![header_to_add, header_to_replace]);

        let srt = synthetic_response(headers, Some("true".to_string()), Some(true), None, None);

        let (verifier, context) = verifier();
        let result = verifier
            .visit_synthetic_response(&srt, &context)
            .expect_err("Could verify CDNI synthetic response with incorrect MEL type of expression to calculate response");

        assert_matches!(result, ExpressionWrongType(Type::Integer, Type::Boolean));
    }

    #[test]
    fn test_verify_synthetic_response_wrong_body_expr_type() {
        let header_to_add = typed_header("add", "value", None);
        let header_to_replace = typed_header("replace", "value", None);
        let headers = Some(vec![header_to_add, header_to_replace]);

        let srt = synthetic_response(headers, None, None, Some("true".to_string()), Some(true));

        let (verifier, context) = verifier();
        let result = verifier
            .visit_synthetic_response(&srt, &context)
            .expect_err("Could verify CDNI synthetic response with incorrect MEL type of expression to calculate body");

        assert_matches!(result, ExpressionWrongType(Type::String, Type::Boolean));
    }
}

#[cfg(test)]
mod test_verify_from_json {
    use std::assert_matches;

    use crate::mel::tvs::Type;
    use crate::ps::spec::{
        HeaderTransform, ResponseTransform, SyntheticResponse, TypedHeaderTransform,
        TypedSyntheticResponse,
    };
    use crate::tests::read_test_file;
    use crate::{
        mel::ast::Expr::BinaryExpr,
        ps::{
            spec::{
                RequestTransform, TypedProcessingStages, TypedRequestTransform,
                TypedResponseTransform,
            },
            verify::{
                CdniVerificationError::{ExpressionWrongType, WrongGenericMetadataTypeName},
                CdniVerificationKey, verify_cdni,
            },
        },
    };

    use std::path::Path;

    #[test]
    fn test_verify_simple() {
        let json = read_test_file(Path::new("./src/ps/tests/simple/deserialize_verify.json"));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages).expect("Could not verify proper CDNI");

        let result_serialized =
            serde_json::to_string_pretty(&result).expect("Could not serialize verified CDNI");
        pretty_assertions::assert_eq!(json, result_serialized);
    }

    #[test]
    fn test_verify_bad_generic_md_typename() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/generic_metadata/bad-typename.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages)
            .expect_err("Could verify CDNI generic metadata with invalid metadata type name");

        assert_matches!(result, WrongGenericMetadataTypeName(expected, actual) if expected == "MI.ProcessingStages" && actual == "MI.ProcessigStages")
    }

    #[test]
    fn test_verify_stage_metadata_with_generic_metadata_bad_type_name() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/generic_metadata/bad-typename2.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages).expect_err("Could verify mistyped CDNI");

        assert_matches!(result, WrongGenericMetadataTypeName(expected, actual) if expected == "MI. ..." && actual == "Mi.CachePolicy");
    }

    #[test]
    fn test_verify_match_wrong_type() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/stage_rules/wrong_match_type.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages)
            .expect_err("Could verify CDNI expression match with incorrect MEL type");

        assert_matches!(result, ExpressionWrongType(Type::Boolean, Type::Integer))
    }

    #[test]
    fn test_verify_request_transform() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/request_transform/uri_no_expr.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages).expect("Could not verify correct CDNI");

        assert_matches!(
            result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .request_xform,
            Some(TypedRequestTransform {
                tpe: _,
                value: RequestTransform {
                    xform: _,
                    uri: _,
                    uri_is_expr: Some(false),
                    aug: CdniVerificationKey::None
                }
            })
        );
    }

    #[test]
    fn test_verify_request_transform_uri_expr() {
        let json = read_test_file(Path::new("./src/ps/tests/request_transform/uri_expr.json"));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages).expect("Could not verify correct CDNI");

        assert_matches!(
            result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .request_xform,
            Some(TypedRequestTransform {
                tpe: _,
                value: RequestTransform {
                    xform: _,
                    uri: _,
                    uri_is_expr: Some(true),
                    aug: CdniVerificationKey::Expr(BinaryExpr(_))
                }
            })
        );
    }

    #[test]
    fn test_verify_request_transform_uri_expr_wrong_type() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/request_transform/uri_expr_wrong_type.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages)
            .expect_err("Could verify CDNI with incorrect MEL type of expression to calculate URI");

        assert_matches!(result, ExpressionWrongType(Type::String, Type::Integer));
    }

    #[test]
    fn test_verify_response_transform() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/response_transform/rs_no_expr.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages).expect("Could not verify correct CDNI");

        assert_matches!(
            result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .response_xform,
            Some(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: _,
                    response_status: _,
                    response_status_expr: None,
                    synthetic: None,
                    aug: CdniVerificationKey::None
                }
            })
        );
    }

    #[test]
    fn test_verify_response_transform_rs_expr() {
        let json = read_test_file(Path::new("./src/ps/tests/response_transform/rs_expr.json"));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages).expect("Could not verify correct CDNI");

        assert_matches!(
            result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .response_xform,
            Some(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: _,
                    response_status: _,
                    response_status_expr: Some(true),
                    synthetic: None,
                    aug: CdniVerificationKey::Expr(BinaryExpr(_))
                }
            })
        );
    }

    #[test]
    fn test_verify_response_transform_rs_expr_wrong_type() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/response_transform/rs_expr_wrong_type.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages)
            .expect_err("Could verify CDNI response transform with incorrect MEL type of expression to calculate response status");

        assert_matches!(result, ExpressionWrongType(Type::Integer, Type::Boolean));
    }

    #[test]
    fn test_verify_header_transform() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/header_transform/value_no_expr.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages).expect("Could not verify correct CDNI");

        assert_matches!(
            &result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .response_xform,
            Some(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: Some(TypedHeaderTransform {
                        tpe: _,
                        value: HeaderTransform {
                            delete: Some(d),
                            add: Some(a),
                            replace: Some(b),
                            aug: CdniVerificationKey::None
                        }
                    }),
                    response_status: _,
                    response_status_expr: None,
                    synthetic: None,
                    aug: CdniVerificationKey::None
                }
            }) if d.len() == 2 && a.len() == 1 && b.len() == 1
        );
    }

    #[test]
    fn test_verify_header_transform_value_expr() {
        let json = read_test_file(Path::new("./src/ps/tests/header_transform/value_expr.json"));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages).expect("Could not verify correct CDNI");

        assert_matches!(
            &result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .response_xform,
            Some(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: Some(TypedHeaderTransform {
                        tpe: _,
                        value: HeaderTransform {
                            delete: _,
                            add: Some(a),
                            replace: Some(b),
                            aug: CdniVerificationKey::None
                        }
                    }),
                    response_status: _,
                    response_status_expr: None,
                    synthetic: None,
                    aug: CdniVerificationKey::None
                }
            }) if matches!(a[0].value.aug, CdniVerificationKey::Expr(_)) && matches!(b[0].value.aug, CdniVerificationKey::Expr(_))
        );
    }

    #[test]
    fn test_verify_header_transform_value_expr_wrong_type_add() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/header_transform/value_expr_wrong_type.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages)
            .expect_err("Could verify CDNI typed header with incorrect MEL type of expression to calculate header value");

        assert_matches!(result, ExpressionWrongType(Type::String, Type::Boolean));
    }

    #[test]
    fn test_verify_header_transform_value_expr_wrong_type_replace() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/header_transform/value_expr_wrong_type_replace.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result =
            verify_cdni(&stages).expect_err("Could verify CDNI with invalid metadata type name");
        assert_matches!(result, ExpressionWrongType(Type::String, Type::Boolean));
    }

    #[test]
    fn test_verify_synthetic_response() {
        let json = read_test_file(Path::new("./src/ps/tests/synthetic_response/no_expr.json"));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages).expect("Could not verify correct CDNI");

        assert_matches!(
            &result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .response_xform,
            Some(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: None,
                    response_status: _,
                    response_status_expr: _,
                    synthetic: Some(TypedSyntheticResponse {
                        tpe: _,
                        value: SyntheticResponse {
                            headers: _,
                            response_status: _,
                            response_status_expr: _,
                            body: _,
                            body_expr: _,
                            aug: CdniVerificationKey::ExprPair(None, None)
                        }
                    }),
                    aug: CdniVerificationKey::None
                }
            })
        );
    }

    #[test]
    fn test_verify_synthetic_response_response_expr() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/synthetic_response/response_expr.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages).expect("Could not verify correct CDNI");

        assert_matches!(
            &result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .response_xform,
            Some(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: None,
                    response_status: _,
                    response_status_expr: _,
                    synthetic: Some(TypedSyntheticResponse {
                        tpe: _,
                        value: SyntheticResponse {
                            headers: _,
                            response_status: _,
                            response_status_expr: _,
                            body: _,
                            body_expr: _,
                            aug: CdniVerificationKey::ExprPair(Some(_), None)
                        }
                    }),
                    aug: CdniVerificationKey::None
                }
            })
        );
    }

    #[test]
    fn test_verify_synthetic_response_body_expr() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/synthetic_response/body_expr.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages).expect("Could not verify correct CDNI");

        assert_matches!(
            &result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .response_xform,
            Some(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: None,
                    response_status: _,
                    response_status_expr: _,
                    synthetic: Some(TypedSyntheticResponse {
                        tpe: _,
                        value: SyntheticResponse {
                            headers: _,
                            response_status: _,
                            response_status_expr: _,
                            body: _,
                            body_expr: _,
                            aug: CdniVerificationKey::ExprPair(None, Some(_))
                        }
                    }),
                    aug: CdniVerificationKey::None
                }
            })
        );
    }

    #[test]
    fn test_verify_synthetic_response_body_and_response_expr() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/synthetic_response/both_body_response_exprs.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages).expect("Could not verify correct CDNI");

        assert_matches!(
            &result.value.client_req[0]
                .value
                .stage_metadata
                .value
                .response_xform,
            Some(TypedResponseTransform {
                tpe: _,
                value: ResponseTransform {
                    xform: None,
                    response_status: _,
                    response_status_expr: _,
                    synthetic: Some(TypedSyntheticResponse {
                        tpe: _,
                        value: SyntheticResponse {
                            headers: _,
                            response_status: _,
                            response_status_expr: _,
                            body: _,
                            body_expr: _,
                            aug: CdniVerificationKey::ExprPair(Some(_), Some(_))
                        }
                    }),
                    aug: CdniVerificationKey::None
                }
            })
        );
    }

    #[test]
    fn test_verify_synthetic_response_wrong_response_expr_type() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/synthetic_response/response_expr_wrong_type.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages)
            .expect_err("Could verify CDNI synthetic response with incorrect MEL type of expression to calculate response");

        assert_matches!(result, ExpressionWrongType(Type::Integer, Type::String));
    }

    #[test]
    fn test_verify_synthetic_response_wrong_body_expr_type() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/synthetic_response/body_expr_wrong_type.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages)
            .expect_err("Could verify CDNI synthetic response with incorrect MEL type of expression to calculate body");

        assert_matches!(result, ExpressionWrongType(Type::String, Type::Boolean));
    }
}
