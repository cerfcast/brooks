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

use http::{HeaderName, HeaderValue, StatusCode, Uri, uri::InvalidUri};

use crate::{
    cdni::{
        metadata::{CacheSpecification, CdniMetadata, CdniMetadataElements},
        spec::TypedGenericMetadata,
    },
    logging::LogMsgs,
    mel::{
        analysis::Analyzed,
        ast::Expr,
        interpreter::{
            self,
            builtins::builtin_builtin_function_interpreters,
            interpret::{
                MelInterpAssertion, MelInterpContext, MelInterpError, MelInterpLocatableError,
                TypedValue, Value,
            },
        },
        scope::{Scope, Scopes},
        tvs::Type,
    },
    ps::{
        interpret::{
            PsInterpretMode::HeaderCalculate,
            PsInterpretValue::{Header, MatchNo, MatchYes},
            PsInterpretValueType::{MatchResult, SyntheticResponse, Terminate},
        },
        spec::{
            TypedClientRequestStage, TypedExpressionMatch, TypedHeader, TypedHeaderTransform,
            TypedMatchGroup, TypedProcessingStages, TypedRequestTransform, TypedResponseTransform,
            TypedStage, TypedStageMetadata, TypedStageRules, TypedSyntheticResponse,
        },
        verify::PsVerificationKey,
        visit::{PsGenericMetadataVisitor, PsVisitor, PsVisitorResult},
    },
};

use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    sync::Arc,
};

#[derive(Debug, Clone)]
pub enum ProcessableRequestResponseError {
    BadValue,
    InvalidMode,
}

impl Display for ProcessableRequestResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            ProcessableRequestResponseError::BadValue => write!(f, "Bad value"),
            ProcessableRequestResponseError::InvalidMode => write!(f, "Invalid mode"),
        }
    }
}

pub type ProcessableRequestResponseResult<T> = Result<T, ProcessableRequestResponseError>;

/// A request that can be manipulated by interpretation processing stages.
pub trait ProcessableRequestResponse: Debug {
    fn header_value(&self) -> Option<String>;
    fn headers(&self) -> Vec<String>;

    fn set_header_value(
        &mut self,
        header: &str,
        value: &str,
    ) -> ProcessableRequestResponseResult<()>;
    fn clear_headers(&mut self) -> ProcessableRequestResponseResult<()>;
    fn remove_header(&mut self, header: &str) -> ProcessableRequestResponseResult<()>;
    fn add_header(&mut self, header: &str, value: &str) -> ProcessableRequestResponseResult<()>;

    fn uri(&self) -> ProcessableRequestResponseResult<Uri>;
    fn set_uri(&mut self, uri: &Uri) -> ProcessableRequestResponseResult<()>;

    fn set_response(&mut self, response: &u16) -> ProcessableRequestResponseResult<()>;
}

pub type PsInterpretResult =
    Result<(PsInterpretValue, CdniMetadataElements), Box<PsInterpretError>>;

#[derive(Debug, Clone)]
pub enum PsInterpretAssertionFailures {
    MissingAnalyzedExpression,
    MissingInterpreterExpressionValue,
    InvalidInterpreterMode,
    MissingResult,
}

impl Display for PsInterpretAssertionFailures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PsInterpretAssertionFailures::MissingAnalyzedExpression => {
                write!(f, "Missing information about an analyzed expression")
            }
            PsInterpretAssertionFailures::MissingInterpreterExpressionValue => {
                write!(f, "Missing value from an interpreted expression")
            }
            PsInterpretAssertionFailures::InvalidInterpreterMode => {
                write!(f, "Invalid interpreter mode")
            }
            PsInterpretAssertionFailures::MissingResult => write!(f, "Missing result"),
        }
    }
}

#[derive(Debug, Default)]
pub enum PsInterpretError {
    #[default]
    NoError,
    AssertionFailure(PsInterpretAssertionFailures),
    MelInterpreterError(Box<MelInterpLocatableError>),
    InvalidRequest,
    InvalidUri(InvalidUri),
    InvalidResponse(String),
    WrongType(PsInterpretValueType, PsInterpretValueType),
    WrongMatchGroupValueType(PsInterpretValueType),
    ProcessableRequestResponseError(ProcessableRequestResponseError),
}

impl Display for PsInterpretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PsInterpretError::NoError => write!(f, "No Error"),
            PsInterpretError::AssertionFailure(af) => {
                write!(f, "Assertion failure: {af}")
            }
            PsInterpretError::MelInterpreterError(meli) => {
                write!(f, "MEL Interpreter error: {}", meli)
            }
            PsInterpretError::InvalidRequest => write!(f, "Invalid HTTP request"),
            PsInterpretError::InvalidUri(iuri) => write!(f, "Invalid URI: {iuri}"),
            PsInterpretError::InvalidResponse(response) => {
                write!(f, "Invalid Response: {response}")
            }
            PsInterpretError::WrongType(expected, actual) => {
                write!(f, "Wrong type: expected: {expected} actual: {actual}",)
            }
            PsInterpretError::WrongMatchGroupValueType(actual) => {
                write!(f, "Match group value should not have {actual} type")
            }
            PsInterpretError::ProcessableRequestResponseError(
                processable_request_response_error,
            ) => {
                write!(
                    f,
                    "Error occurred when modifying the request/response: {processable_request_response_error}"
                )
            }
        }
    }
}

#[derive(Default, Clone)]
pub struct PsGenericMetadataInterpreter<A: Debug + Clone + Default, O, E> {
    interpreters: HashMap<String, Arc<dyn PsGenericMetadataVisitor<A, O, E>>>,
}

impl<A: Debug + Clone + Default, O, E> Debug for PsGenericMetadataInterpreter<A, O, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PsGenericMetadataInterpreter")
    }
}

impl<A: Debug + Clone + Default, O, E> PsGenericMetadataInterpreter<A, O, E> {
    pub fn add_interpreter(
        &mut self,
        tpe: &str,
        interp: Arc<dyn PsGenericMetadataVisitor<A, O, E>>,
    ) {
        self.interpreters.insert(tpe.to_string(), interp);
    }

    pub fn get_interpreter(&mut self, tpe: &str) -> Option<&dyn PsGenericMetadataVisitor<A, O, E>> {
        self.interpreters.get(tpe).map(|f| &**f)
    }
}

// CDNI Generic Metadata Interpreters

#[derive(Debug, Clone, Default)]
struct PsGenericMetadataCachePolicy {}

impl PsGenericMetadataVisitor<PsVerificationKey, PsInterpretContext, PsInterpretError>
    for PsGenericMetadataCachePolicy
{
    fn visit_generic_metadata(
        &self,
        _v: &TypedGenericMetadata<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        Ok(c.update_metadata_elements(Arc::new(CacheSpecification {})))
    }
}

// CDNI Processing Stage Interpreter

struct PsInterpreter<'a> {
    pub generic_interpreters:
        PsGenericMetadataInterpreter<PsVerificationKey, PsInterpretContext, PsInterpretError>,
    pub req: &'a mut dyn ProcessableRequestResponse,
}

impl<'a> PsInterpreter<'a> {
    fn install_generic_visitors(&mut self) {
        self.generic_interpreters
            .add_interpreter("MI.CachePolicy", Arc::new(PsGenericMetadataCachePolicy {}));
    }

    fn scopes_from_req(
        &self,
        additional: &[Scope<TypedValue>],
    ) -> Result<Scopes<TypedValue>, PsInterpretError> {
        let mel_req = http::Request::builder()
            .uri(
                self.req
                    .uri()
                    .map_err(|_| PsInterpretError::InvalidRequest)?,
            )
            .body("")
            .map_err(|_| PsInterpretError::InvalidRequest)?;

        Ok(Scopes::<TypedValue> {
            scopes: vec![
                additional
                    .iter()
                    .fold(Scope::<TypedValue>::from(mel_req), |c, n| &c + n),
            ],
        })
    }

    fn evaluate_mel_expr(
        &self,
        expr: &Expr<Analyzed>,
        expected: Type,
    ) -> Result<TypedValue, PsInterpretError> {
        let expr_context = MelInterpContext {
            val: None,
            scopes: self.scopes_from_req(&[builtin_builtin_function_interpreters()])?,
            log: LogMsgs::new(crate::logging::LogLevel::Trace),
        };

        let result = interpreter::interpret(expr, expr_context.clone())
            .map_err(PsInterpretError::MelInterpreterError)?
            .val
            .ok_or(PsInterpretError::AssertionFailure(
                PsInterpretAssertionFailures::MissingInterpreterExpressionValue,
            ))?;

        if result.tipe == expected {
            Ok(result)
        } else {
            Err(PsInterpretError::MelInterpreterError(
                MelInterpLocatableError {
                    error: MelInterpError::Assertion(
                        MelInterpAssertion::TypeMismatch(expected, result.tipe).into(),
                    )
                    .into(),
                    context: expr_context.clone(),
                    location: expr.location().clone(),
                }
                .into(),
            ))
        }
    }

    fn interpret_match_groups_in_stage(
        &mut self,
        mgs: &Vec<TypedMatchGroup<PsVerificationKey>>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        let mut result: Result<(PsInterpretValue, CdniMetadataElements), PsInterpretError> = Err(
            PsInterpretError::AssertionFailure(PsInterpretAssertionFailures::MissingResult),
        );
        for mg in mgs {
            let visit_result = self.visit_match_group(mg, &c.clone())?;
            match visit_result.result {
                Some(
                    r @ (PsInterpretValue::Terminate | PsInterpretValue::SyntheticResponse(_)),
                ) => {
                    // This result stops processing.
                    result = Ok((r, visit_result.metadata));
                    break;
                }
                Some(r @ (PsInterpretValue::MatchYes | PsInterpretValue::MatchNo)) => {
                    // This result continues processing.
                    result = Ok((r, visit_result.metadata));
                }
                Some(r) => {
                    return Err(PsInterpretError::WrongType(
                        PsInterpretValueType::Terminate,
                        r.into(),
                    ));
                }
                None => {
                    return Err(PsInterpretError::AssertionFailure(
                        PsInterpretAssertionFailures::MissingResult,
                    ));
                }
            }
        }

        match result {
            Ok((result, md)) => Ok(c.update_result(Some(result)).replace_metadata_elements(md)),
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum PsInterpretMode {
    Request,
    Response,
    HeaderAdd,
    HeaderReplace,
    HeaderCalculate,
    #[default]
    None,
}

#[derive(Debug, Clone)]
pub enum PsInterpretValueType {
    SyntheticResponse,
    Terminate,
    MatchResult,
    Header,
}

impl Display for PsInterpretValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            SyntheticResponse => write!(f, "Synthetic response"),
            Terminate => write!(f, "Terminate"),
            MatchResult => write!(f, "Match result"),
            PsInterpretValueType::Header => write!(f, "Header"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum PsInterpretValue {
    SyntheticResponse(http::Response<String>),
    Terminate,
    MatchYes,
    MatchNo,
    Header(String, String),
}

impl From<PsInterpretValue> for PsInterpretValueType {
    fn from(value: PsInterpretValue) -> Self {
        match value {
            PsInterpretValue::SyntheticResponse(_) => SyntheticResponse,
            PsInterpretValue::Terminate => Terminate,
            MatchYes | MatchNo => MatchResult,
            PsInterpretValue::Header(_, _) => PsInterpretValueType::Header,
        }
    }
}

impl From<bool> for PsInterpretValue {
    fn from(value: bool) -> Self {
        if value { MatchYes } else { MatchNo }
    }
}

#[derive(Debug, Clone, Default)]
struct PsInterpretContext {
    mode: PsInterpretMode,
    result: Option<PsInterpretValue>,
    metadata: CdniMetadataElements,
}

impl PsInterpretContext {
    fn update_mode(&self, new_mode: PsInterpretMode) -> PsInterpretContext {
        let mut nc = self.clone();
        nc.mode = new_mode;
        nc
    }
    fn update_result(&self, new_result: Option<PsInterpretValue>) -> PsInterpretContext {
        let mut nc = self.clone();
        nc.result = new_result;
        nc
    }

    fn update_metadata_elements(&self, new_element: Arc<dyn CdniMetadata>) -> PsInterpretContext {
        let mut nc = self.clone();
        nc.metadata.elements.push(new_element);
        nc
    }
    fn replace_metadata_elements(&self, new_elements: CdniMetadataElements) -> PsInterpretContext {
        let mut nc = self.clone();
        nc.metadata = new_elements;
        nc
    }
}

impl<'a> PsVisitor<PsVerificationKey, PsInterpretContext, PsInterpretError> for PsInterpreter<'a> {
    fn visit_processing_stages(
        &mut self,
        _v: &TypedProcessingStages<PsVerificationKey>,
        _c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        todo!()
    }

    fn visit_stage_rules(
        &mut self,
        v: &TypedStageRules<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        let doq = if let Some(mtch) = &v.value.mtch {
            let expr = match &mtch.value.aug {
                PsVerificationKey::Expr(expr) => expr,
                _ => {
                    return Err(PsInterpretError::AssertionFailure(
                        PsInterpretAssertionFailures::MissingAnalyzedExpression,
                    ));
                }
            };
            let result = self.evaluate_mel_expr(expr, Type::Boolean)?;
            match result {
                TypedValue {
                    tipe: Type::Boolean,
                    value: Value::Boolean(v),
                } => v,
                _ => unreachable!(),
            }
        } else {
            true
        };

        if doq {
            self.visit_stage_metadata(&v.value.stage_metadata, &c.update_result(Some(doq.into())))
        } else {
            Ok(c.update_result(Some(doq.into())))
        }
    }

    fn visit_expression_match(
        &mut self,
        _v: &TypedExpressionMatch<PsVerificationKey>,
        _c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        todo!()
    }

    fn visit_stage_metadata(
        &mut self,
        v: &TypedStageMetadata<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        let mut c = c.clone();
        if let Some(generic) = &v.value.generic {
            for generic in generic {
                c = self.visit_generic_metadata(generic, &c)?;
            }
        }

        match &c.mode {
            PsInterpretMode::Request => {
                if let Some(req_xform) = &v.value.request_xform {
                    return self.visit_request_transform(req_xform, &c);
                }
            }
            PsInterpretMode::Response => {
                if let Some(res_xform) = &v.value.response_xform {
                    return self.visit_response_transform(res_xform, &c);
                }
            }
            _ => {
                return Err(PsInterpretError::AssertionFailure(
                    PsInterpretAssertionFailures::InvalidInterpreterMode,
                ));
            }
        }
        Ok(c.clone())
    }

    fn visit_request_transform(
        &mut self,
        v: &TypedRequestTransform<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        let mut c = c.clone();
        if let Some(header_xform) = &v.value.xform {
            c = self.visit_header_transform(
                header_xform,
                &PsInterpretContext {
                    mode: PsInterpretMode::Request,
                    metadata: c.metadata.clone(),
                    result: c.result.clone(),
                },
            )?;
        }

        if let Some(new_uri) = &v.value.uri {
            let new_uri = if let Some(uri_is_expr) = &v.value.uri_is_expr
                && *uri_is_expr
            {
                let expr = match &v.value.aug {
                    PsVerificationKey::Expr(expr) => expr,
                    _ => {
                        return Err(PsInterpretError::AssertionFailure(
                            PsInterpretAssertionFailures::MissingAnalyzedExpression,
                        ));
                    }
                };
                let result = self.evaluate_mel_expr(expr, Type::String)?;
                match result {
                    TypedValue {
                        tipe: Type::String,
                        value: Value::String(s),
                    } => Uri::try_from(s),
                    _ => unreachable!(),
                }
            } else {
                Uri::try_from(new_uri.clone())
            }
            .map_err(PsInterpretError::InvalidUri)?;
            self.req
                .set_uri(&new_uri)
                .map_err(PsInterpretError::ProcessableRequestResponseError)?
        }

        Ok(c.clone())
    }

    fn visit_response_transform(
        &mut self,
        v: &TypedResponseTransform<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        if let Some(synthetic_response) = &v.value.synthetic {
            return self.visit_synthetic_response(synthetic_response, c);
        }
        let mut c = c.clone();

        if let Some(header_xform) = &v.value.xform {
            c = self
                .visit_header_transform(header_xform, &c.update_mode(PsInterpretMode::Response))?;
        }

        if let Some(new_response) = &v.value.response_status {
            let new_response = if let Some(response_is_expr) = &v.value.response_status_expr
                && *response_is_expr
            {
                let expr = match &v.value.aug {
                    PsVerificationKey::Expr(expr) => expr,
                    _ => {
                        return Err(PsInterpretError::AssertionFailure(
                            PsInterpretAssertionFailures::MissingAnalyzedExpression,
                        ));
                    }
                };
                let result = self.evaluate_mel_expr(expr, Type::Integer)?;
                match result {
                    TypedValue {
                        tipe: Type::Integer,
                        value: Value::Integer(i),
                    } => Ok(i),
                    _ => unreachable!(),
                }
            } else {
                new_response.parse::<i64>()
            }
            .map_err(|e| PsInterpretError::InvalidResponse(e.to_string()))?;
            let new_response = u16::try_from(new_response).map_err(|e| {
                PsInterpretError::InvalidResponse(format!(
                    "Could not convert {new_response} to unsigned 16-bit number: {e}"
                ))
            })?;
            self.req.set_response(&new_response).map_err(|_| {
                PsInterpretError::ProcessableRequestResponseError(
                    ProcessableRequestResponseError::BadValue,
                )
            })?;
        }

        Ok(c.clone())
    }

    fn visit_generic_metadata(
        &mut self,
        _v: &TypedGenericMetadata<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        match self.generic_interpreters.get_interpreter(&_v.tpe) {
            Some(interp) => interp.visit_generic_metadata(_v, c),
            None => Err(PsInterpretError::InvalidResponse(format!(
                "No generic interpreter available for generic metadata of type {}",
                _v.tpe
            ))),
        }
    }

    fn visit_header_transform(
        &mut self,
        v: &TypedHeaderTransform<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        if let Some(to_delete) = &v.value.delete {
            for htr in to_delete {
                self.req
                    .remove_header(htr)
                    .map_err(PsInterpretError::ProcessableRequestResponseError)?
            }
        }

        if let Some(to_add) = &v.value.add {
            for htr in to_add {
                self.visit_header(
                    htr,
                    &PsInterpretContext {
                        mode: PsInterpretMode::HeaderAdd,
                        ..Default::default()
                    },
                )?;
            }
        }

        if let Some(to_replace) = &v.value.replace {
            for htr in to_replace {
                self.visit_header(
                    htr,
                    &PsInterpretContext {
                        mode: PsInterpretMode::HeaderReplace,
                        ..Default::default()
                    },
                )?;
            }
        }

        Ok(c.clone())
    }

    fn visit_header(
        &mut self,
        v: &TypedHeader<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        let value = if let Some(expr) = &v.value.value_expr
            && *expr
        {
            let expr = match &v.value.aug {
                PsVerificationKey::Expr(expr) => expr,
                _ => {
                    return Err(PsInterpretError::AssertionFailure(
                        PsInterpretAssertionFailures::MissingAnalyzedExpression,
                    ));
                }
            };
            let result = self.evaluate_mel_expr(expr, Type::String)?;
            match result {
                TypedValue {
                    tipe: Type::String,
                    value: Value::String(s),
                } => s,
                _ => unreachable!(),
            }
        } else {
            v.value.value.clone()
        };

        match c.mode {
            PsInterpretMode::HeaderAdd => {
                self.req
                    .add_header(&v.value.name, &value)
                    .map_err(PsInterpretError::ProcessableRequestResponseError)?;
                Ok(c.clone())
            }
            PsInterpretMode::HeaderReplace => {
                self.req
                    .set_header_value(&v.value.name, &value)
                    .map_err(PsInterpretError::ProcessableRequestResponseError)?;
                Ok(c.clone())
            }
            PsInterpretMode::HeaderCalculate => {
                Ok(c.update_result(Some(Header(v.value.name.clone(), value))))
            }
            _ => todo!(),
        }
    }

    fn visit_synthetic_response(
        &mut self,
        v: &TypedSyntheticResponse<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        let mut response = http::Response::builder();

        if let Some(headers) = &v.value.headers {
            for header in headers {
                let result = self.visit_header(header, &c.update_mode(HeaderCalculate))?;
                match result.result.ok_or(PsInterpretError::AssertionFailure(
                    PsInterpretAssertionFailures::MissingAnalyzedExpression,
                ))? {
                    PsInterpretValue::Header(name, value) => {
                        response.headers_mut().unwrap().insert(
                            HeaderName::from_bytes(name.as_bytes()).expect("Todo"),
                            HeaderValue::from_str(&value).expect("Todo"),
                        );
                    }
                    r => {
                        return Err(PsInterpretError::WrongType(
                            PsInterpretValueType::Header,
                            r.into(),
                        ));
                    }
                }
            }
        }

        response = response.status(if let Some(new_response) = &v.value.response_status {
            if let Some(response_is_expr) = &v.value.response_status_expr
                && *response_is_expr
            {
                let expr = match &v.value.aug {
                    PsVerificationKey::ExprPair(Some(expr), _) => expr,
                    _ => {
                        return Err(PsInterpretError::AssertionFailure(
                            PsInterpretAssertionFailures::MissingAnalyzedExpression,
                        ));
                    }
                };
                let result = self.evaluate_mel_expr(expr, Type::Integer)?;
                match result {
                    TypedValue {
                        tipe: Type::Integer,
                        value: Value::Integer(i),
                    } => StatusCode::from_u16(i as u16),
                    _ => unreachable!(),
                }
            } else {
                new_response.parse::<StatusCode>()
            }
            .map_err(|e| PsInterpretError::InvalidResponse(e.to_string()))?
        } else {
            StatusCode::OK
        });

        let response = response
            .body(if let Some(body) = &v.value.body {
                if let Some(body_is_expr) = &v.value.body_expr
                    && *body_is_expr
                {
                    let expr = match &v.value.aug {
                        PsVerificationKey::ExprPair(_, Some(expr)) => expr,
                        _ => {
                            return Err(PsInterpretError::AssertionFailure(
                                PsInterpretAssertionFailures::MissingAnalyzedExpression,
                            ));
                        }
                    };
                    let result = self.evaluate_mel_expr(expr, Type::String)?;
                    match result {
                        TypedValue {
                            tipe: Type::String,
                            value: Value::String(s),
                        } => s,
                        _ => unreachable!(),
                    }
                } else {
                    body.clone()
                }
            } else {
                "".to_string()
            })
            .expect("Could not create HTTP response");

        Ok(c.update_result(Some(PsInterpretValue::SyntheticResponse(response))))
    }

    fn visit_match_group(
        &mut self,
        v: &TypedMatchGroup<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        let else_ifs = v
            .value
            .else_ifs
            .as_ref()
            .map(|else_ifs| else_ifs.iter())
            .unwrap_or([].iter());

        let mut result: Result<(PsInterpretValue, CdniMetadataElements), PsInterpretError> = Err(
            PsInterpretError::AssertionFailure(PsInterpretAssertionFailures::MissingResult),
        );
        let rules = [&v.value.if_rule].into_iter().chain(else_ifs);
        for r in rules {
            let visit_result = self.visit_stage_rules(r, &c.clone())?;
            match visit_result.result {
                Some(r @ PsInterpretValue::MatchNo) => {
                    result = Ok((r, visit_result.metadata));
                    continue; // do the next rule.
                }
                Some(r) => {
                    result = Ok((r, visit_result.metadata));
                    break; // do _not_ do the next rule in any other case.
                }
                None => {
                    return Err(PsInterpretError::AssertionFailure(
                        PsInterpretAssertionFailures::MissingResult,
                    ));
                }
            };
        }
        match result {
            Ok((result, md)) => Ok(c.update_result(Some(result)).replace_metadata_elements(md)),
            Err(e) => Err(e),
        }
    }

    fn visit_client_request_stage(
        &mut self,
        v: &TypedClientRequestStage<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        self.interpret_match_groups_in_stage(&v.value.match_groups, c)
    }

    fn visit_origin_request_stage(
        &mut self,
        v: &super::spec::TypedOriginRequestStage<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        self.interpret_match_groups_in_stage(&v.value.match_groups, c)
    }

    fn visit_client_response_stage(
        &mut self,
        v: &super::spec::TypedClientResponseStage<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        self.interpret_match_groups_in_stage(&v.value.match_groups, c)
    }

    fn visit_origin_response_stage(
        &mut self,
        v: &super::spec::TypedOriginResponseStage<PsVerificationKey>,
        c: &PsInterpretContext,
    ) -> PsVisitorResult<PsInterpretContext, PsInterpretError> {
        self.interpret_match_groups_in_stage(&v.value.match_groups, c)
    }
}

pub fn interpret_stage(
    ts: &TypedStage<PsVerificationKey>,
    req: &mut dyn ProcessableRequestResponse,
    mode: PsInterpretMode,
) -> PsInterpretResult {
    let mut visitor = PsInterpreter {
        req,
        generic_interpreters: Default::default(),
    };

    visitor.install_generic_visitors();

    let context = PsInterpretContext {
        mode,
        ..Default::default()
    };

    let result = match ts {
        TypedStage::ClientRequest(typed_client_request_stage) => {
            visitor.visit_client_request_stage(typed_client_request_stage, &context)
        }
        TypedStage::ClientResponse(typed_client_response_stage) => {
            visitor.visit_client_response_stage(typed_client_response_stage, &context)
        }
        TypedStage::OriginRequest(typed_origin_request_stage) => {
            visitor.visit_origin_request_stage(typed_origin_request_stage, &context)
        }
        TypedStage::OriginResponse(typed_origin_response_stage) => {
            visitor.visit_origin_response_stage(typed_origin_response_stage, &context)
        }
    }?;

    let md = result.metadata;
    let result = result
        .result
        .ok_or(Box::new(PsInterpretError::AssertionFailure(
            PsInterpretAssertionFailures::MissingResult,
        )))?;
    Ok((result, md))
}

#[derive(Debug)]
enum EffectfulRequestActions {
    DeleteHeader(String),
    AddHeader(String, String),
    ClearHeaders,
    SetUri(String),
    SetResponse(u16),
}

#[derive(Debug, Default)]
pub(crate) struct EffectfulProcessableRequestResponse {
    log: Vec<EffectfulRequestActions>,
}

impl ProcessableRequestResponse for EffectfulProcessableRequestResponse {
    fn header_value(&self) -> Option<String> {
        None
    }

    fn headers(&self) -> Vec<String> {
        vec![]
    }

    fn set_header_value(
        &mut self,
        _header: &str,
        _value: &str,
    ) -> Result<(), ProcessableRequestResponseError> {
        Ok(())
    }

    fn clear_headers(&mut self) -> ProcessableRequestResponseResult<()> {
        self.log.push(EffectfulRequestActions::ClearHeaders);
        Ok(())
    }

    fn remove_header(&mut self, header: &str) -> Result<(), ProcessableRequestResponseError> {
        self.log
            .push(EffectfulRequestActions::DeleteHeader(header.to_string()));
        Ok(())
    }

    fn uri(&self) -> Result<Uri, ProcessableRequestResponseError> {
        Ok(Uri::default())
    }

    fn set_uri(&mut self, uri: &Uri) -> Result<(), ProcessableRequestResponseError> {
        self.log
            .push(EffectfulRequestActions::SetUri(uri.to_string()));
        Ok(())
    }

    fn set_response(&mut self, response: &u16) -> Result<(), ProcessableRequestResponseError> {
        self.log
            .push(EffectfulRequestActions::SetResponse(*response));
        Ok(())
    }

    fn add_header(
        &mut self,
        header: &str,
        value: &str,
    ) -> Result<(), ProcessableRequestResponseError> {
        self.log.push(EffectfulRequestActions::AddHeader(
            header.to_string(),
            value.to_string(),
        ));
        Ok(())
    }
}

#[cfg(test)]
mod ps_interpreter_tests {
    use crate::{
        cdni::spec::{CachePolicy, TypedCachePolicy, TypedGenericMetadata},
        mel::{scope::Scopes, tvs::Type},
        ps::{
            interpret::{
                EffectfulProcessableRequestResponse, EffectfulRequestActions, PsInterpretMode,
                PsInterpretValue, interpret_stage,
            },
            spec::{TypedGenericStage, TypedStage},
            tests::test_helpers::{
                client_request_stage, expression_match, match_group, request_transform,
                response_transform, stage_metadata, synthetic_response, typed_header,
                typed_stage_rule,
            },
            verify::{PsVerifierContextValue, verifier, verify_ps_request_stage},
            visit::PsVisitor,
        },
        tests::read_test_file,
    };
    use std::assert_matches;
    use std::path::Path;

    #[test]
    fn test_interpret_client_request_stage() {
        let json = read_test_file(Path::new("./src/ps/tests/client_request_stage/if.json"));

        let result = serde_json::from_str::<TypedGenericStage>(&json)
            .expect("Could not deserialize simple client request stage JSON");

        let result = verify_ps_request_stage(&result, Scopes::<Type>::default())
            .expect("Could not verify valid client request stage JSON");

        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(&result, &mut req, PsInterpretMode::Request)
            .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 2);
        assert_matches!(req.log[0], EffectfulRequestActions::DeleteHeader(_));
        assert_matches!(req.log[1], EffectfulRequestActions::AddHeader(_, _));
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    #[test]
    fn test_interpret_client_request_stage_request_response_header_transform() {
        let json = read_test_file(Path::new(
            "./src/ps/tests/client_request_stage/request_response_header_transform.json",
        ));

        let result = serde_json::from_str::<TypedGenericStage>(&json)
            .expect("Could not deserialize simple client request stage JSON");

        let result = verify_ps_request_stage(&result, Scopes::<Type>::default())
            .expect("Could not verify valid client request stage JSON");
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(&result, &mut req, PsInterpretMode::Request)
            .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 2);
        assert_matches!(req.log[0], EffectfulRequestActions::DeleteHeader(_));
        assert_matches!(req.log[1], EffectfulRequestActions::AddHeader(_, _));
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    #[test]
    fn test_interpret_client_request_stage_request_uri_transform() {
        let mut request_xform_stage = stage_metadata();

        let request_xform_match = expression_match("true");
        let request_xform = request_transform(
            None,
            Some("\"http://\" . \"example.com\"".to_string()),
            Some(true),
        );
        request_xform_stage.value.request_xform = Some(request_xform);

        let request_xform_mg = match_group(
            typed_stage_rule(Some(request_xform_match), request_xform_stage),
            None,
        );

        let crs = client_request_stage(vec![request_xform_mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Request,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetUri(r) if r == "http://example.com/");
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    fn test_interpret_client_request_stage_request_uri_transform_if() {
        let mut request_xform_false_stage = stage_metadata();
        let mut request_xform_true_stage = stage_metadata();

        let request_xform_false_match = expression_match("true");
        let request_xform_false = request_transform(
            None,
            Some("\"http://\" . \"example.com\"".to_string()),
            Some(true),
        );
        request_xform_false_stage.value.request_xform = Some(request_xform_false);

        let request_xform_true_match = expression_match("true");
        let request_xform_true = request_transform(
            None,
            Some("\"http://\" . \"example2.com\"".to_string()),
            Some(true),
        );
        request_xform_true_stage.value.request_xform = Some(request_xform_true);

        let mg = match_group(
            typed_stage_rule(Some(request_xform_false_match), request_xform_false_stage),
            Some(vec![typed_stage_rule(
                Some(request_xform_true_match),
                request_xform_true_stage,
            )]),
        );
        let crs = client_request_stage(vec![mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Request,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetUri(r) if r == "http://example.com/");
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    #[test]
    fn test_interpret_client_request_stage_request_uri_transform_else() {
        let mut request_xform_false_stage = stage_metadata();
        let mut request_xform_false_false_stage = stage_metadata();
        let mut request_xform_true_stage = stage_metadata();

        let request_xform_false_match = expression_match("false");
        let request_xform_false = request_transform(
            None,
            Some("\"http://\" . \"example.com\"".to_string()),
            Some(true),
        );
        request_xform_false_stage.value.request_xform = Some(request_xform_false);

        let request_xform_false_false_match = expression_match("true");
        let request_xform_false_false = request_transform(
            None,
            Some("\"http://\" . \"example1.com\"".to_string()),
            Some(true),
        );
        request_xform_false_false_stage.value.request_xform = Some(request_xform_false_false);

        let request_xform_true_match = expression_match("true");
        let request_xform_true = request_transform(
            None,
            Some("\"http://\" . \"example2.com\"".to_string()),
            Some(true),
        );

        request_xform_true_stage.value.request_xform = Some(request_xform_true);

        let mg = match_group(
            typed_stage_rule(Some(request_xform_false_match), request_xform_false_stage),
            Some(vec![
                typed_stage_rule(
                    Some(request_xform_false_false_match),
                    request_xform_false_false_stage,
                ),
                typed_stage_rule(Some(request_xform_true_match), request_xform_true_stage),
            ]),
        );
        let crs = client_request_stage(vec![mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Request,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetUri(r) if r == "http://example1.com/");
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    fn test_interpret_client_request_stage_request_uri_transform_else_if() {
        let mut request_xform_false_stage = stage_metadata();
        let mut request_xform_false_false_stage = stage_metadata();
        let mut request_xform_true_stage = stage_metadata();

        let request_xform_false_match = expression_match("false");
        let request_xform_false = request_transform(
            None,
            Some("\"http://\" . \"example.com\"".to_string()),
            Some(true),
        );
        request_xform_false_stage.value.request_xform = Some(request_xform_false);

        let request_xform_false_false_match = expression_match("false");
        let request_xform_false_false = request_transform(
            None,
            Some("\"http://\" . \"example1.com\"".to_string()),
            Some(true),
        );
        request_xform_false_false_stage.value.request_xform = Some(request_xform_false_false);

        let request_xform_true_match = expression_match("true");
        let request_xform_true = request_transform(
            None,
            Some("\"http://\" . \"example2.com\"".to_string()),
            Some(true),
        );

        request_xform_true_stage.value.request_xform = Some(request_xform_true);

        let mg = match_group(
            typed_stage_rule(Some(request_xform_false_match), request_xform_false_stage),
            Some(vec![
                typed_stage_rule(
                    Some(request_xform_false_false_match),
                    request_xform_false_false_stage,
                ),
                typed_stage_rule(Some(request_xform_true_match), request_xform_true_stage),
            ]),
        );
        let crs = client_request_stage(vec![mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Request,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetUri(r) if r == "http://example2.com/");
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    #[test]
    fn test_interpret_client_request_stage_response_status_transform() {
        let mut response_xform_stage = stage_metadata();

        let response_xform_match = expression_match("true");
        let response_xform = response_transform(None, Some("5 + 4".to_string()), Some(true), None);
        response_xform_stage.value.response_xform = Some(response_xform);

        let response_xform_mg = match_group(
            typed_stage_rule(Some(response_xform_match), response_xform_stage),
            None,
        );

        let crs = client_request_stage(vec![response_xform_mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Response,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetResponse(9));
    }

    fn test_interpret_client_request_stage_response_status_transform_if() {
        let mut response_xform_false_stage = stage_metadata();
        let mut response_xform_true_stage = stage_metadata();

        let response_xform_false_match = expression_match("true");
        let response_xform_false =
            response_transform(None, Some("5 + 4".to_string()), Some(true), None);
        response_xform_false_stage.value.response_xform = Some(response_xform_false);

        let response_xform_true_match = expression_match("true");
        let response_xform_true =
            response_transform(None, Some("5 + 5".to_string()), Some(true), None);
        response_xform_true_stage.value.response_xform = Some(response_xform_true);

        let mg = match_group(
            typed_stage_rule(Some(response_xform_false_match), response_xform_false_stage),
            Some(vec![typed_stage_rule(
                Some(response_xform_true_match),
                response_xform_true_stage,
            )]),
        );
        let crs = client_request_stage(vec![mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Response,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetResponse(9));
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    #[test]
    fn test_interpret_client_request_stage_response_status_transform_else() {
        let mut response_xform_false_stage = stage_metadata();
        let mut response_xform_false_false_stage = stage_metadata();
        let mut response_xform_true_stage = stage_metadata();

        let response_xform_false_match = expression_match("false");
        let response_xform_false =
            response_transform(None, Some("5 + 4".to_string()), Some(true), None);
        response_xform_false_stage.value.response_xform = Some(response_xform_false);

        let response_xform_false_false_match = expression_match("true");
        let response_xform_false_false =
            response_transform(None, Some("5 + 5".to_string()), Some(true), None);
        response_xform_false_false_stage.value.response_xform = Some(response_xform_false_false);

        let response_xform_true_match = expression_match("true");
        let response_xform_true =
            response_transform(None, Some("5 + 6".to_string()), Some(true), None);

        response_xform_true_stage.value.response_xform = Some(response_xform_true);

        let mg = match_group(
            typed_stage_rule(Some(response_xform_false_match), response_xform_false_stage),
            Some(vec![
                typed_stage_rule(
                    Some(response_xform_false_false_match),
                    response_xform_false_false_stage,
                ),
                typed_stage_rule(Some(response_xform_true_match), response_xform_true_stage),
            ]),
        );
        let crs = client_request_stage(vec![mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Response,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetResponse(10));
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    fn test_interpret_client_request_stage_response_status_transform_else_if() {
        let mut response_xform_false_stage = stage_metadata();
        let mut response_xform_false_false_stage = stage_metadata();
        let mut response_xform_true_stage = stage_metadata();

        let response_xform_false_match = expression_match("false");
        let response_xform_false =
            response_transform(None, Some("5 + 4".to_string()), Some(true), None);
        response_xform_false_stage.value.response_xform = Some(response_xform_false);

        let response_xform_false_false_match = expression_match("false");
        let response_xform_false_false =
            response_transform(None, Some("5 + 5".to_string()), Some(true), None);
        response_xform_false_false_stage.value.response_xform = Some(response_xform_false_false);

        let response_xform_true_match = expression_match("true");
        let response_xform_true =
            response_transform(None, Some("5 + 6".to_string()), Some(true), None);

        response_xform_true_stage.value.response_xform = Some(response_xform_true);

        let mg = match_group(
            typed_stage_rule(Some(response_xform_false_match), response_xform_false_stage),
            Some(vec![
                typed_stage_rule(
                    Some(response_xform_false_false_match),
                    response_xform_false_false_stage,
                ),
                typed_stage_rule(Some(response_xform_true_match), response_xform_true_stage),
            ]),
        );
        let crs = client_request_stage(vec![mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Response,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 1);
        assert_matches!(&req.log[0], EffectfulRequestActions::SetResponse(11));
        assert_matches!(result.0, PsInterpretValue::MatchYes);
    }

    #[test]
    fn test_interpret_client_request_stage_response_synthetic() {
        let mut response_xform_true_stage = stage_metadata();

        let response_xform_true_match = expression_match("true");
        let response_xform_true = response_transform(
            None,
            None,
            None,
            Some(synthetic_response(
                Some(vec![
                    typed_header("X-custom1", "Custom value 1", None),
                    typed_header("X-custom2", "Custom value 2", None),
                ]),
                Some("400 + 4".to_string()),
                Some(true),
                Some("\"This \" . \"is \" . \"a \" . \"test.\"".to_string()),
                Some(true),
            )),
        );
        response_xform_true_stage.value.response_xform = Some(response_xform_true);

        let mg = match_group(
            typed_stage_rule(Some(response_xform_true_match), response_xform_true_stage),
            None,
        );
        let crs = client_request_stage(vec![mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Response,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 0);
        assert_matches!(&result.0, PsInterpretValue::SyntheticResponse(r)
            if r.status() == 404 && r.body() == "This is a test." && r.headers().len() == 2);
    }

    #[test]
    fn test_interpret_client_request_stage_response_generic_metadata() {
        let mut response_xform_true_stage = stage_metadata();

        let response_xform_true_match = expression_match("true");
        let response_xform_true = response_transform(None, None, None, None);
        response_xform_true_stage.value.response_xform = Some(response_xform_true);

        let cp = TypedCachePolicy::<()>::typed_value(CachePolicy {
            policy: "Testing".to_string(),
            aug: (),
        });

        let gcp = TypedGenericMetadata {
            tpe: "MI.CachePolicy".to_string(),
            value: serde_json::to_value(cp.value).expect("TODO"),
            aug: (),
        };

        response_xform_true_stage.value.generic = Some(vec![gcp]);

        let mg = match_group(
            typed_stage_rule(Some(response_xform_true_match), response_xform_true_stage),
            None,
        );
        let crs = client_request_stage(vec![mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Response,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 0);
        assert_eq!(result.1.elements.len(), 1);
    }

    #[test]
    fn test_interpret_client_request_stage_request_generic_metadata() {
        let mut request_xform_true_stage = stage_metadata();

        let request_xform_true_match = expression_match("true");
        let request_xform_true = request_transform(None, None, None);
        request_xform_true_stage.value.request_xform = Some(request_xform_true);

        let cp = TypedCachePolicy::<()>::typed_value(CachePolicy {
            policy: "Testing".to_string(),
            aug: (),
        });

        let gcp = TypedGenericMetadata {
            tpe: "MI.CachePolicy".to_string(),
            value: serde_json::to_value(cp.value).expect("TODO"),
            aug: (),
        };

        request_xform_true_stage.value.generic = Some(vec![gcp]);

        let mg = match_group(
            typed_stage_rule(Some(request_xform_true_match), request_xform_true_stage),
            None,
        );
        let crs = client_request_stage(vec![mg]);

        let (mut verifier, context) = verifier();

        let result = verifier
            .visit_client_request_stage(&crs, &context)
            .expect("Could not verify valid client request")
            .value
            .expect("Could not get value from verified client request");

        let value = match result {
            PsVerifierContextValue::ClientRequestStage(typed_client_request_stage) => {
                typed_client_request_stage
            }
            _ => todo!(),
        };
        let mut req = EffectfulProcessableRequestResponse::default();
        let result = interpret_stage(
            &TypedStage::ClientRequest(value),
            &mut req,
            PsInterpretMode::Response,
        )
        .expect("Could not interpret a valid client request");

        assert_eq!(req.log.len(), 0);
        assert_eq!(result.1.elements.len(), 1);
    }
}
