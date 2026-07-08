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
    cdni::{
        spec::{
            ExpressionMatch, Header, HeaderTransform, ProcessingStages, RequestTransform,
            ResponseTransform, StageMetadata, StageRules, TypedExpressionMatch,
            TypedGenericMetadata, TypedHeader, TypedHeaderTransform, TypedProcessingStages,
            TypedRequestTransform, TypedResponseTransform, TypedStageMetadata, TypedStageRules,
            TypedSyntheticResponse,
        },
        visit::CdniVisitor,
    },
    mel::{
        analysis::{Analyzed, MelAnalysisLocatableError, analyze},
        ast::Expr,
        compiler::{self, compile::CompilerError},
        scope::Scopes,
        tvs::Type,
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
        analyze(&expr, Scopes::default()).map_err(CdniVerificationError::ExpressionAnalyze)
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
        _v: &TypedSyntheticResponse<()>,
        _c: &CdniVerifierContext,
    ) -> super::visit::CdniVisitorResult<CdniVerifierContext, CdniVerificationError> {
        todo!()
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

    use crate::cdni::spec::{HeaderTransform, ResponseTransform, TypedHeaderTransform};
    use crate::cdni::verify::{CdniVerifierContextValue, verifier};
    use crate::cdni::visit::CdniVisitor;
    use crate::mel::tvs::Type;
    use crate::{
        cdni::{
            spec::TypedResponseTransform,
            verify::{
                CdniVerificationError::{ExpressionWrongType, WrongGenericMetadataTypeName},
                CdniVerificationKey, verify_cdni,
            },
        },
        mel::ast::Expr::BinaryExpr,
    };

    use crate::cdni::tests::test_helpers::{
        expression_match, generic_metadata, header_transform, processing_stages, request_transform,
        response_transform, typed_header,
    };

    #[test]
    fn test_verify_bad_generic_md_typename() {
        let mut stages = processing_stages(vec![], vec![], vec![], vec![]);
        stages.tpe = "MI.ProcessigStages".to_string();

        let result = verify_cdni(&stages).expect_err("Could verify invalid CDNI");

        assert_matches!(result, WrongGenericMetadataTypeName(expected, actual) if expected == "MI.ProcessingStages" && actual == "MI.ProcessigStages")
    }

    #[test]
    fn test_verify_generic_metadata_with_bad_type_name() {
        let generic = generic_metadata("Mi.CachePolicy", None);

        let (verifier, context) = verifier();
        let result = verifier
            .visit_generic_metadata(&generic, &context)
            .expect_err("Could verify mistyped CDNI");

        assert_matches!(result, WrongGenericMetadataTypeName(expected, actual) if expected == "MI. ..." && actual == "Mi.CachePolicy");
    }

    #[test]
    fn test_verify_match_wrong_type() {
        let mtch = expression_match("5 + 4");
        let (verifier, context) = verifier();
        let result = verifier
            .visit_expression_match(&mtch, &context)
            .expect_err("Could verify mistyped CDNI");

        assert_matches!(result, ExpressionWrongType(Type::Boolean, Type::Integer))
    }

    #[test]
    fn test_verify_request_transform_uri_expr_wrong_type() {
        let xform = request_transform(None, Some("5+4".to_string()), Some(true));
        let (verifier, context) = verifier();
        let result = verifier
            .visit_request_transform(&xform, &context)
            .expect_err("Could verify CDNI with incorrect expression type in URI");

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
            .expect_err("Could verify CDNI with incorrect expression type in URI");

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
            .expect_err("Could verify incorrect CDNI");

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
            .expect_err("Could verify incorrect CDNI");

        assert_matches!(result, ExpressionWrongType(Type::String, Type::Boolean));
    }
}

#[cfg(test)]
mod test_verify_from_json {
    use std::assert_matches;

    use crate::cdni::spec::{HeaderTransform, ResponseTransform, TypedHeaderTransform};
    use crate::mel::tvs::Type;
    use crate::{
        cdni::{
            spec::{
                RequestTransform, TypedProcessingStages, TypedRequestTransform,
                TypedResponseTransform,
            },
            verify::{
                CdniVerificationError::{ExpressionWrongType, WrongGenericMetadataTypeName},
                CdniVerificationKey, verify_cdni,
            },
        },
        mel::ast::Expr::BinaryExpr,
    };

    use std::path::Path;

    use crate::cdni::tests::test_helpers::read_test_file;

    #[test]
    fn test_verify_simple() {
        let json = read_test_file(Path::new("./src/cdni/tests/simple/deserialize_verify.json"));
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
            "./src/cdni/tests/generic_metadata/bad-typename.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages)
            .expect_err("Could verify CDNI with bad generic metadata typenames");

        assert_matches!(result, WrongGenericMetadataTypeName(expected, actual) if expected == "MI.ProcessingStages" && actual == "MI.ProcessigStages")
    }

    #[test]
    fn test_verify_stage_metadata_with_generic_metadata_bad_type_name() {
        let json = read_test_file(Path::new(
            "./src/cdni/tests/generic_metadata/bad-typename2.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages).expect_err("Could verify mistyped CDNI");

        assert_matches!(result, WrongGenericMetadataTypeName(expected, actual) if expected == "MI. ..." && actual == "Mi.CachePolicy");
    }

    #[test]
    fn test_verify_match_wrong_type() {
        let json = read_test_file(Path::new(
            "./src/cdni/tests/stage_rules/wrong_match_type.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages).expect_err("Could verify mistyped CDNI");

        assert_matches!(result, ExpressionWrongType(Type::Boolean, Type::Integer))
    }

    #[test]
    fn test_verify_request_transform() {
        let json = read_test_file(Path::new(
            "./src/cdni/tests/request_transform/uri_no_expr.json",
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
        let json = read_test_file(Path::new(
            "./src/cdni/tests/request_transform/uri_expr.json",
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
                    uri_is_expr: Some(true),
                    aug: CdniVerificationKey::Expr(BinaryExpr(_))
                }
            })
        );
    }

    #[test]
    fn test_verify_request_transform_uri_expr_wrong_type() {
        let json = read_test_file(Path::new(
            "./src/cdni/tests/request_transform/uri_expr_wrong_type.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages)
            .expect_err("Could verify CDNI with incorrect expression type in URI");

        assert_matches!(result, ExpressionWrongType(Type::String, Type::Integer));
    }

    #[test]
    fn test_verify_response_transform() {
        let json = read_test_file(Path::new(
            "./src/cdni/tests/response_transform/rs_no_expr.json",
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
        let json = read_test_file(Path::new(
            "./src/cdni/tests/response_transform/rs_expr.json",
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
            "./src/cdni/tests/response_transform/rs_expr_wrong_type.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages)
            .expect_err("Could verify CDNI with incorrect expression type in response status");

        assert_matches!(result, ExpressionWrongType(Type::Integer, Type::Boolean));
    }

    #[test]
    fn test_verify_header_transform() {
        let json = read_test_file(Path::new(
            "./src/cdni/tests/header_transform/value_no_expr.json",
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
        let json = read_test_file(Path::new(
            "./src/cdni/tests/header_transform/value_expr.json",
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
    fn test_verify_header_transform_value_expr_wrong_type() {
        let json = read_test_file(Path::new(
            "./src/cdni/tests/header_transform/value_expr_wrong_type.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages)
            .expect_err("Could verify CDNI with incorrect expression type in response status");

        assert_matches!(result, ExpressionWrongType(Type::String, Type::Boolean));
    }

    #[test]
    fn test_verify_header_transform_value_expr_wrong_type_replace() {
        let json = read_test_file(Path::new(
            "./src/cdni/tests/header_transform/value_expr_wrong_type_replace.json",
        ));
        let stages = serde_json::from_str::<TypedProcessingStages<()>>(&json)
            .expect("Could not parse JSON test file");

        let result = verify_cdni(&stages)
            .expect_err("Could verify CDNI with incorrect expression type in response status");

        assert_matches!(result, ExpressionWrongType(Type::String, Type::Boolean));
    }
}
