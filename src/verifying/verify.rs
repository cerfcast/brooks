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

use std::fmt::Debug;

use crate::cdni::mel::spec::TypedSetVariable;
use crate::cdni::original::spec::TypedSource;
use crate::verifying::mel::verify::{MelVerificationError, SetVariableVerificationKey};
use crate::verifying::ps::error::VerificationError;
use crate::{environment::scope::Scopes, mel::types::Type};

use std::error::Error;
use std::fmt::Display;

use serde::Serialize;

use crate::verifying::original::verify::SourceVerificationKey;
use crate::verifying::ps::verify::PsVerificationKey;

use crate::cdni::ps::spec::TypedStage;

use crate::cdni::gmd::spec::TypedGenericMetadata;

#[derive(Debug, Clone, Default)]
pub struct MetadataAnalysisContext {
    pub scopes: Scopes<Type>,
}

#[derive(Debug, Clone, Default)]
pub enum MetadataVerificationError {
    #[default]
    NoError,
    NotProcessingStage,
    NoProcessor(String),
    JsonError(String),
    PsVerificationError(Box<VerificationError>),
    MelVerificationError(Box<MelVerificationError>),
}

impl Error for MetadataVerificationError {}

impl Display for MetadataVerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetadataVerificationError::NoError => write!(f, "No error!"),
            MetadataVerificationError::NotProcessingStage => {
                write!(f, "Value was not a processing stage!")
            }
            MetadataVerificationError::NoProcessor(tpe) => {
                write!(f, "No processor registered for Metadata with type {tpe}")
            }
            MetadataVerificationError::PsVerificationError(pse) => write!(f, "{pse}"),
            MetadataVerificationError::JsonError(je) => write!(f, "JSON error: {je}"),
            MetadataVerificationError::MelVerificationError(mel_verification_error) => {
                write!(f, "{mel_verification_error}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CdniVerifiedMetadataTypes {
    ProcessingStages,
    Source,
    SetVariable,
    None,
}

#[derive(Debug, Clone, Default)]
pub enum CdniVerificationKey {
    Stage(Box<TypedStage<PsVerificationKey>>),
    Source(Box<TypedSource<SourceVerificationKey>>),
    SetVariable(Box<TypedSetVariable<SetVariableVerificationKey>>),
    #[default]
    None,
}

impl From<&CdniVerificationKey> for CdniVerifiedMetadataTypes {
    fn from(value: &CdniVerificationKey) -> Self {
        match value {
            CdniVerificationKey::Stage(_) => CdniVerifiedMetadataTypes::ProcessingStages,
            CdniVerificationKey::Source(_) => CdniVerifiedMetadataTypes::Source,
            CdniVerificationKey::SetVariable(_) => CdniVerifiedMetadataTypes::SetVariable,
            CdniVerificationKey::None => CdniVerifiedMetadataTypes::None,
        }
    }
}

impl Serialize for CdniVerificationKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_none()
    }
}

pub type AnalysisResult<Ck, Error> = Result<(Ck, CdniVerificationKey), Box<Error>>;

/// Generate a validated representation of a given piece of Metadata Information, if possible.
pub trait Verifier<Ck, Error>: Debug {
    fn analyze(&self, v: &TypedGenericMetadata<()>, input: Ck) -> AnalysisResult<Ck, Error>;

    fn type_name(&self, candidate: &str) -> bool;
}
