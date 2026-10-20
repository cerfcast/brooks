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

//! All Metadata Information except for what is defined in Processing Stages.

use crate::{
    cdni::{
        gmd::spec::TypedGenericMetadata,
        mel::spec::{SetVariable, TypedSetVariable},
    },
    mel::{
        analysis::{self, Analyzed, MelAnalysisLocatableError},
        ast::Expr,
        compiler::{self, compile::MelCompilerLocatableError},
        interpreter::values::TypedValue,
    },
    verifying::verify::{
        AnalysisResult, CdniVerificationKey, MetadataAnalysisContext, MetadataVerificationError,
        Verifier,
    },
};
use std::fmt::{Debug, Display};

#[derive(Debug, Clone, Default)]
pub struct SetVariableVerificationKey {
    pub value: Option<TypedValue>,
    pub expression: Option<Expr<Analyzed>>,
}

#[derive(Debug, Clone)]
pub enum MelVerificationError {
    CompilationError(Box<MelCompilerLocatableError>),
    AnalysisError(Box<MelAnalysisLocatableError>),
}

impl Display for MelVerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MelVerificationError::CompilationError(mel_compiler_locatable_error) => {
                write!(f, "Compilation error: {mel_compiler_locatable_error}")
            }
            MelVerificationError::AnalysisError(mel_analysis_locatable_error) => {
                write!(f, "Analysis error: {mel_analysis_locatable_error}")
            }
        }
    }
}

#[derive(Debug)]
pub struct SetVariableMetadataAnalyzer {}

impl Verifier<MetadataAnalysisContext, MetadataVerificationError> for SetVariableMetadataAnalyzer {
    fn analyze(
        &self,
        v: &TypedGenericMetadata<()>,
        input: MetadataAnalysisContext,
    ) -> AnalysisResult<MetadataAnalysisContext, MetadataVerificationError> {
        let typed_set_variable = serde_json::from_value::<TypedSetVariable<()>>(
            serde_json::to_value(v)
                .map_err(|e| MetadataVerificationError::JsonError(e.to_string()))?,
        )
        .map_err(|e| MetadataVerificationError::JsonError(e.to_string()))?;

        let mut verification_key = SetVariableVerificationKey::default();
        if let Some(value_is_expression) = typed_set_variable.value.value_is_expression
            && value_is_expression
        {
            let parsed =
                compiler::compile(&typed_set_variable.value.variable_value).map_err(|e| {
                    MetadataVerificationError::MelVerificationError(
                        MelVerificationError::CompilationError(e.into()).into(),
                    )
                })?;

            let analyzed = analysis::analyze(&parsed, &input.scopes).map_err(|e| {
                MetadataVerificationError::MelVerificationError(
                    MelVerificationError::AnalysisError(e).into(),
                )
            })?;
            verification_key.expression = Some(analyzed)
        } else {
            todo!()
        }

        let res = TypedSetVariable {
            tpe: TypedSetVariable::<()>::typed_cdni_metadata_name(),
            value: SetVariable {
                aug: verification_key,
                variable_name: typed_set_variable.value.variable_name,
                variable_value: typed_set_variable.value.variable_value,
                value_is_expression: typed_set_variable.value.value_is_expression,
            },
        };

        Ok((input, CdniVerificationKey::SetVariable(Box::new(res))))
    }

    fn type_name(&self, candidate: &str) -> bool {
        candidate == TypedSetVariable::<()>::typed_cdni_metadata_name()
    }
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use crate::{
        cdni::{
            mel::spec::TypedSetVariable,
            tests::test_helpers::{generic_metadata, set_variable},
        },
        mel::scope::builtin_function_types,
        verifying::{
            mel::verify::SetVariableMetadataAnalyzer,
            verify::{CdniVerificationKey, MetadataAnalysisContext, Verifier},
        },
    };

    #[test]
    fn verify_test() {
        let variable = set_variable("is_bad".to_string(), "true".to_string(), Some(true));

        let json = generic_metadata(
            &TypedSetVariable::<()>::typed_cdni_metadata_name(),
            Some(serde_json::to_value(variable.value).expect("TODO")),
        );

        let svma = SetVariableMetadataAnalyzer {};

        let result = svma.analyze(
            &json,
            MetadataAnalysisContext {
                scopes: builtin_function_types().into(),
            },
        );

        assert_matches!(result, Ok((_, CdniVerificationKey::SetVariable(e))) if e.value.aug.expression.is_some());
    }
}
