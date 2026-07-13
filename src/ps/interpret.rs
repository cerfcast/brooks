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

use http::Uri;

use crate::{
    logging::LogMsgs,
    mel::{
        interpreter::{
            self,
            interpret::{
                MelInterpContext, MelInterpError, MelInterpLocatableError, TypedValue, Value,
            },
        },
        scope::Scopes,
        tvs::Type,
    },
    ps::{
        spec::{
            TypedClientRequestStage, TypedExpressionMatch, TypedGenericMetadata, TypedHeader,
            TypedHeaderTransform, TypedMatchGroup, TypedProcessingStages, TypedRequestTransform,
            TypedResponseTransform, TypedStageMetadata, TypedStageRules, TypedSyntheticResponse,
        },
        verify::CdniVerificationKey,
        visit::CdniVisitor,
    },
};

use std::fmt::Debug;

pub trait ProcessableRequest: Debug {
    fn header_value(&self) -> &str;
    fn headers(&self) -> &[&str];
    fn set_header_value(&mut self, header: &str, value: &str);
    fn remove_header(&mut self, header: &str);
    fn add_header(&mut self, header: &str, value: &str);
    fn request(&self) -> Uri;
}

pub type CdniInterpreterResult = Result<(), CdniInterpreterError>;

#[derive(Debug, Clone)]
pub enum CdniInterpreterAssertionFailures {
    MissingAnalyzedExpression,
    MissingInterpreterExpressionValue,
    InvalidInterpreterMode,
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for CdniInterpreterAssertionFailures {
    fn to_string(&self) -> String {
        match self {
            CdniInterpreterAssertionFailures::MissingAnalyzedExpression => {
                "Missing information about an analyzed expression".to_string()
            }
            CdniInterpreterAssertionFailures::MissingInterpreterExpressionValue => {
                "Missing value from an interpreted expression".to_string()
            }
            CdniInterpreterAssertionFailures::InvalidInterpreterMode => {
                "Invalid interpreter mode".to_string()
            }
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Default)]
pub enum CdniInterpreterError {
    #[default]
    NoError,
    AssertionFailure(CdniInterpreterAssertionFailures),
    MelInterpreterError(MelInterpLocatableError),
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for CdniInterpreterError {
    fn to_string(&self) -> String {
        match self {
            CdniInterpreterError::NoError => "No Error".to_string(),
            CdniInterpreterError::AssertionFailure(af) => {
                format!("Assertion failure: {}", af.to_string())
            }
            CdniInterpreterError::MelInterpreterError(meli) => {
                format!("MEL Interpreter error: {}", meli)
            }
        }
    }
}

#[derive(Debug)]
struct CdniInterpreter<'a> {
    pub req: &'a mut dyn ProcessableRequest,
}

#[derive(Debug, Clone, Default)]
enum CdniInterpreterMode {
    Request,
    Response,
    HeaderAdd,
    HeaderReplace,
    #[default]
    None,
}

#[derive(Debug, Clone)]
enum CdniInterpreterVisitResult {
    SyntheticResponse,
    Terminate,
    MatchYes,
    MatchNo,
}

#[derive(Debug, Clone, Default)]
struct CdniInterpreterContext {
    mode: CdniInterpreterMode,
    match_result: Option<CdniInterpreterVisitResult>,
}

impl<'a> CdniVisitor<CdniVerificationKey, CdniInterpreterContext, CdniInterpreterError>
    for CdniInterpreter<'a>
{
    fn visit_processing_stages(
        &mut self,
        _v: &TypedProcessingStages<CdniVerificationKey>,
        _c: &CdniInterpreterContext,
    ) -> super::visit::CdniVisitorResult<CdniInterpreterContext, CdniInterpreterError> {
        todo!()
    }

    fn visit_stage_rules(
        &mut self,
        v: &TypedStageRules<CdniVerificationKey>,
        c: &CdniInterpreterContext,
    ) -> super::visit::CdniVisitorResult<CdniInterpreterContext, CdniInterpreterError> {
        let doq = if let Some(mtch) = &v.value.mtch {
            let expr = match &mtch.value.aug {
                CdniVerificationKey::Expr(expr) => expr,
                _ => {
                    return Err(CdniInterpreterError::AssertionFailure(
                        CdniInterpreterAssertionFailures::MissingAnalyzedExpression,
                    ));
                }
            };

            let expr_context = MelInterpContext {
                val: None,
                scopes: Scopes::default(),
                log: LogMsgs::new(crate::logging::LogLevel::Trace),
            };

            let result = interpreter::interpret(expr, expr_context.clone())
                .map_err(CdniInterpreterError::MelInterpreterError)?
                .val
                .ok_or(CdniInterpreterError::AssertionFailure(
                    CdniInterpreterAssertionFailures::MissingInterpreterExpressionValue,
                ))?;

            match result {
                TypedValue {
                    tipe: Type::Boolean,
                    value: Value::Boolean(v),
                } => v,
                TypedValue { tipe: a, value: _ } => {
                    return Err(CdniInterpreterError::MelInterpreterError(
                        MelInterpLocatableError {
                            error: MelInterpError::Assertion(
                                interpreter::interpret::MelInterpAssertion::TypeMismatch(
                                    Type::Boolean,
                                    a,
                                ),
                            ),
                            context: expr_context.clone(),
                            location: expr.location().clone(),
                        },
                    ));
                }
            }
        } else {
            true
        };

        if doq {
            match &c.mode {
                CdniInterpreterMode::Request => {
                    if let Some(req_xform) = &v.value.stage_metadata.value.request_xform {
                        self.visit_request_transform(req_xform, &c.clone())?;
                    }
                }
                CdniInterpreterMode::Response => {
                    if let Some(res_xform) = &v.value.stage_metadata.value.response_xform {
                        self.visit_response_transform(res_xform, &c.clone())?;
                    }
                }
                _ => {
                    return Err(CdniInterpreterError::AssertionFailure(
                        CdniInterpreterAssertionFailures::InvalidInterpreterMode,
                    ));
                }
            }
        }

        Ok(c.clone())
    }

    fn visit_expression_match(
        &mut self,
        _v: &TypedExpressionMatch<CdniVerificationKey>,
        _c: &CdniInterpreterContext,
    ) -> super::visit::CdniVisitorResult<CdniInterpreterContext, CdniInterpreterError> {
        todo!()
    }

    fn visit_stage_metadata(
        &mut self,
        _v: &TypedStageMetadata<CdniVerificationKey>,
        _c: &CdniInterpreterContext,
    ) -> super::visit::CdniVisitorResult<CdniInterpreterContext, CdniInterpreterError> {
        todo!()
    }

    fn visit_request_transform(
        &mut self,
        v: &TypedRequestTransform<CdniVerificationKey>,
        c: &CdniInterpreterContext,
    ) -> super::visit::CdniVisitorResult<CdniInterpreterContext, CdniInterpreterError> {
        if let Some(header_xform) = &v.value.xform {
            self.visit_header_transform(
                header_xform,
                &CdniInterpreterContext {
                    mode: CdniInterpreterMode::Request,
                    match_result: None,
                },
            )?;
        }

        Ok(c.clone())
    }

    fn visit_response_transform(
        &mut self,
        _v: &TypedResponseTransform<CdniVerificationKey>,
        _c: &CdniInterpreterContext,
    ) -> super::visit::CdniVisitorResult<CdniInterpreterContext, CdniInterpreterError> {
        todo!()
    }

    fn visit_generic_metadata(
        &mut self,
        _v: &TypedGenericMetadata<CdniVerificationKey>,
        _c: &CdniInterpreterContext,
    ) -> super::visit::CdniVisitorResult<CdniInterpreterContext, CdniInterpreterError> {
        todo!()
    }

    fn visit_header_transform(
        &mut self,
        v: &TypedHeaderTransform<CdniVerificationKey>,
        c: &CdniInterpreterContext,
    ) -> super::visit::CdniVisitorResult<CdniInterpreterContext, CdniInterpreterError> {
        if let Some(to_delete) = &v.value.delete {
            for htr in to_delete {
                self.req.remove_header(htr);
            }
        }

        if let Some(to_add) = &v.value.add {
            for htr in to_add {
                self.visit_header(
                    htr,
                    &CdniInterpreterContext {
                        mode: CdniInterpreterMode::HeaderAdd,
                        ..Default::default()
                    },
                )?;
            }
        }

        if let Some(to_replace) = &v.value.replace {
            for htr in to_replace {
                self.visit_header(
                    htr,
                    &CdniInterpreterContext {
                        mode: CdniInterpreterMode::HeaderReplace,
                        ..Default::default()
                    },
                )?;
            }
        }

        Ok(c.clone())
    }

    fn visit_header(
        &mut self,
        v: &TypedHeader<CdniVerificationKey>,
        c: &CdniInterpreterContext,
    ) -> super::visit::CdniVisitorResult<CdniInterpreterContext, CdniInterpreterError> {
        let value = if let Some(expr) = &v.value.value_expr
            && *expr
        {
            let expr_context = MelInterpContext {
                val: None,
                scopes: Scopes::default(),
                log: LogMsgs::new(crate::logging::LogLevel::Trace),
            };

            let expr = match &v.value.aug {
                CdniVerificationKey::Expr(expr) => expr,
                _ => {
                    return Err(CdniInterpreterError::AssertionFailure(
                        CdniInterpreterAssertionFailures::MissingAnalyzedExpression,
                    ));
                }
            };

            let result = interpreter::interpret(expr, expr_context.clone())
                .map_err(CdniInterpreterError::MelInterpreterError)?
                .val
                .ok_or(CdniInterpreterError::AssertionFailure(
                    CdniInterpreterAssertionFailures::MissingInterpreterExpressionValue,
                ))?;

            match result {
                TypedValue {
                    tipe: Type::String,
                    value: Value::String(s),
                } => s,
                TypedValue { tipe: a, value: _ } => {
                    return Err(CdniInterpreterError::MelInterpreterError(
                        MelInterpLocatableError {
                            error: MelInterpError::Assertion(
                                interpreter::interpret::MelInterpAssertion::TypeMismatch(
                                    Type::Boolean,
                                    a,
                                ),
                            ),
                            context: expr_context.clone(),
                            location: expr.location().clone(),
                        },
                    ));
                }
            }
        } else {
            v.value.value.clone()
        };

        match c.mode {
            CdniInterpreterMode::HeaderAdd => {
                self.req.add_header(&v.value.name, &value);
            }
            CdniInterpreterMode::HeaderReplace => {
                self.req.set_header_value(&v.value.name, &value);
            }
            _ => todo!(),
        }

        Ok(c.clone())
    }

    fn visit_synthetic_response(
        &mut self,
        _v: &TypedSyntheticResponse<CdniVerificationKey>,
        _c: &CdniInterpreterContext,
    ) -> super::visit::CdniVisitorResult<CdniInterpreterContext, CdniInterpreterError> {
        todo!()
    }

    fn visit_client_request_stage(
        &mut self,
        v: &TypedClientRequestStage<CdniVerificationKey>,
        c: &CdniInterpreterContext,
    ) -> super::visit::CdniVisitorResult<CdniInterpreterContext, CdniInterpreterError> {
        for mg in &v.value.match_groups {
            match &self.visit_match_group(mg, &c.clone())?.match_result {
                Some(CdniInterpreterVisitResult::Terminate) => {
                    // This result stops processing.
                    todo!()
                }
                Some(CdniInterpreterVisitResult::SyntheticResponse) => {
                    // This result stops processing.
                    todo!()
                }
                Some(CdniInterpreterVisitResult::MatchYes) => {
                    todo!()
                }
                Some(CdniInterpreterVisitResult::MatchNo) => {
                    todo!()
                }
                None => {
                    // Continue
                }
            }
        }
        Ok(c.clone())
    }

    fn visit_match_group(
        &mut self,
        v: &TypedMatchGroup<CdniVerificationKey>,
        c: &CdniInterpreterContext,
    ) -> super::visit::CdniVisitorResult<CdniInterpreterContext, CdniInterpreterError> {
        self.visit_stage_rules(&v.value.if_rule, &c.clone())
    }
}

#[allow(clippy::result_large_err)]
pub fn interpret_client_request(
    cr: &TypedClientRequestStage<CdniVerificationKey>,
    req: &mut dyn ProcessableRequest,
) -> CdniInterpreterResult {
    let mut visitor = CdniInterpreter { req };
    let context = CdniInterpreterContext {
        mode: CdniInterpreterMode::Request,
        ..Default::default()
    };
    visitor.visit_client_request_stage(cr, &context)?;
    Ok(())
}

#[derive(Debug)]
enum EffectfulRequestActions {
    Delete(String),
    Add(String, String),
}

#[derive(Debug, Default)]
struct EffectfulProcessableRequest {
    log: Vec<EffectfulRequestActions>,
}

#[cfg(test)]
impl ProcessableRequest for EffectfulProcessableRequest {
    fn header_value(&self) -> &str {
        todo!()
    }

    fn headers(&self) -> &[&str] {
        todo!()
    }

    fn set_header_value(&mut self, _header: &str, _value: &str) {
        todo!()
    }

    fn remove_header(&mut self, header: &str) {
        self.log
            .push(EffectfulRequestActions::Delete(header.to_string()));
    }

    fn request(&self) -> Uri {
        todo!()
    }

    fn add_header(&mut self, header: &str, value: &str) {
        self.log.push(EffectfulRequestActions::Add(
            header.to_string(),
            value.to_string(),
        ));
    }
}

#[cfg(test)]
mod ps_interpreter_tests {
    use crate::{
        ps::{
            interpret::{
                EffectfulProcessableRequest, EffectfulRequestActions, interpret_client_request,
            },
            spec::TypedClientRequestStage,
            verify::{CdniVerifierContextValue, verifier},
            visit::CdniVisitor,
        },
        tests::read_test_file,
    };
    use std::assert_matches;
    use std::path::Path;

    #[test]
    fn test_interpret_client_request_stage() {
        let json = read_test_file(Path::new("./src/ps/tests/client_request_stage/if.json"));

        let result = serde_json::from_str::<TypedClientRequestStage<()>>(&json)
            .expect("Could not deserialize simple client request stage JSON");

        let (mut verifier, context) = verifier();
        let result = verifier
            .visit_client_request_stage(&result, &context)
            .expect("Could not verify value client request stage JSON")
            .value
            .expect("Could not get the client request stage value");

        let value = match result {
            CdniVerifierContextValue::ClientRequestStage(crs) => crs,
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequest::default();
        interpret_client_request(&value, &mut req)
            .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 2);
        assert_matches!(req.log[0], EffectfulRequestActions::Delete(_));
        assert_matches!(req.log[1], EffectfulRequestActions::Add(_, _));
    }
}
