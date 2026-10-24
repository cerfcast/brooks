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
    common::GrammarLocation,
    mel::{
        MEL_USER_SET_VARIABLE_STRUCT_NAME,
        analysis::{self, Analyzed, MelAnalysisError, MelAnalysisLocatableError},
        ast::Expr,
        compiler::{self, compile::MelCompilerLocatableError},
        types::{
            self,
            Type::{self, Struct},
        },
    },
    verifying::verify::{
        AnalysisResult, CdniVerificationKey, MetadataAnalysisContext, MetadataVerificationError,
        Verifier,
    },
};
use std::fmt::{Debug, Display};

#[derive(Debug, Clone, Default)]
pub struct SetVariableVerificationKey {
    pub value: Option<()>,
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
        mut input: MetadataAnalysisContext,
    ) -> AnalysisResult<MetadataAnalysisContext, MetadataVerificationError> {
        let typed_set_variable = serde_json::from_value::<TypedSetVariable<()>>(
            serde_json::to_value(v)
                .map_err(|e| MetadataVerificationError::JsonError(e.to_string()))?,
        )
        .map_err(|e| MetadataVerificationError::JsonError(e.to_string()))?;

        let mut verification_key = SetVariableVerificationKey::default();

        let expression = if let Some(value_is_expression) =
            typed_set_variable.value.value_is_expression
            && value_is_expression
        {
            let parsed =
                compiler::compile(&typed_set_variable.value.variable_value).map_err(|e| {
                    MetadataVerificationError::MelVerificationError(
                        MelVerificationError::CompilationError(e.into()).into(),
                    )
                })?;

            analysis::analyze(&parsed, &input.scopes).map_err(|e| {
                MetadataVerificationError::MelVerificationError(
                    MelVerificationError::AnalysisError(e).into(),
                )
            })?
        } else {
            todo!()
        };

        verification_key.expression = Some(expression.clone());

        let res = TypedSetVariable {
            tpe: TypedSetVariable::<()>::typed_cdni_metadata_name(),
            value: SetVariable {
                aug: verification_key.clone(),
                variable_name: typed_set_variable.value.variable_name.clone(),
                variable_value: typed_set_variable.value.variable_value,
                value_is_expression: typed_set_variable.value.value_is_expression,
            },
        };

        // Now, modify the fields of the var to include this field!
        let var = input
            .scopes
            .lookup(MEL_USER_SET_VARIABLE_STRUCT_NAME)
            .ok_or(MetadataVerificationError::MelVerificationError(
                MelVerificationError::AnalysisError(
                    MelAnalysisLocatableError {
                        error: MelAnalysisError::UnknownIdentifier(
                            MEL_USER_SET_VARIABLE_STRUCT_NAME.to_string(),
                        )
                        .into(),
                        location: GrammarLocation {
                            start: 0,
                            extent: 0,
                        },
                    }
                    .into(),
                )
                .into(),
            ))?;

        let mut var_type = match var {
            Type::Struct(s) => s,
            it => {
                return Err(MetadataVerificationError::MelVerificationError(
                    MelVerificationError::AnalysisError(
                        MelAnalysisLocatableError {
                            error: MelAnalysisError::InvalidType(
                                vec![Struct(types::Struct::new(
                                    MEL_USER_SET_VARIABLE_STRUCT_NAME,
                                ))],
                                it,
                            )
                            .into(),
                            location: GrammarLocation {
                                start: 0,
                                extent: 0,
                            },
                        }
                        .into(),
                    )
                    .into(),
                )
                .into());
            }
        };

        var_type.insert_field(&typed_set_variable.value.variable_name, expression.tipe());

        input.scopes = input
            .scopes
            .insert(MEL_USER_SET_VARIABLE_STRUCT_NAME, Type::Struct(var_type));

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
        mel::types::{builtin_function_types, minimal_core_variable_types},
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
                scopes: (&builtin_function_types() + &minimal_core_variable_types()).into(),
            },
        );

        assert_matches!(result, Ok((_, CdniVerificationKey::SetVariable(e))) if e.value.aug.expression.is_some());
    }
}
