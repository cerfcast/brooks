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

use crate::cdni::original::spec::TypedSource;
use crate::{environment::scope::Scopes, mel::types::Type};

use std::error::Error;
use std::fmt::Display;

use serde::Serialize;

use crate::verifying::original::verify::{SourceMetadataAnalyzer, SourceVerificationKey};
use crate::verifying::ps::verify::PsVerificationKey;
use crate::verifying::ps::verify::{PsMetadataAnalyzer, PsVerificationError};

use crate::cdni::ps::spec::TypedStage;

use crate::cdni::{gmd::spec::TypedGenericMetadata, imd::spec::HostMetadata};

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
    PsVerificationError(Box<PsVerificationError>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CdniVerifiedMetadataTypes {
    ProcessingStages,
    Source,
    None,
}

#[derive(Debug, Clone, Default)]
pub enum CdniVerificationKey {
    Stage(Box<TypedStage<PsVerificationKey>>),
    Source(Box<TypedSource<SourceVerificationKey>>),
    #[default]
    None,
}

impl From<&CdniVerificationKey> for CdniVerifiedMetadataTypes {
    fn from(value: &CdniVerificationKey) -> Self {
        match value {
            CdniVerificationKey::Stage(_) => CdniVerifiedMetadataTypes::ProcessingStages,
            CdniVerificationKey::Source(_) => CdniVerifiedMetadataTypes::Source,
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
        }
    }
}

pub type AnalysisResult<Ck, Error> = Result<(Ck, CdniVerificationKey), Box<Error>>;

/// Generate a validated representation of a given piece of Metadata Information, if possible.
pub trait Verifier<Ck, Error>: Debug {
    fn analyze(&self, v: &TypedGenericMetadata<()>, input: Ck) -> AnalysisResult<Ck, Error>;

    fn type_name(&self, candidate: &str) -> bool;
}

/// Verify given Interconnection Metadata for the given request.
///
/// TODO: Make clear in the documentation that this function is a _major_ entry point for library clients.
pub fn verify_interconnection_metadata(
    metadata: &HostMetadata<()>,
    scopes: Scopes<Type>,
) -> Result<HostMetadata<CdniVerificationKey>, Box<MetadataVerificationError>> {
    let mut stages: Vec<TypedGenericMetadata<CdniVerificationKey>> = vec![];

    #[allow(clippy::vec_init_then_push)]
    #[allow(unused_mut)]
    let mut processors: Vec<&dyn Verifier<MetadataAnalysisContext, MetadataVerificationError>> =
        vec![];

    #[allow(clippy::vec_init_then_push)]
    processors
        .push(&PsMetadataAnalyzer {}
            as &dyn Verifier<MetadataAnalysisContext, MetadataVerificationError>);

    #[allow(clippy::vec_init_then_push)]
    processors.push(&SourceMetadataAnalyzer {}
        as &dyn Verifier<MetadataAnalysisContext, MetadataVerificationError>);

    for md in &metadata.metadata {
        let mut no_processor = true;
        for processor in &processors {
            if !processor.type_name(&md.tpe) {
                continue;
            }

            no_processor = false;

            let (_, analyzed) = processor.analyze(
                md,
                MetadataAnalysisContext {
                    scopes: scopes.clone(),
                },
            )?;

            stages.push(TypedGenericMetadata {
                tpe: md.tpe.clone(),
                value: md.value.clone(),
                aug: analyzed,
            });
        }

        if no_processor {
            return Err(MetadataVerificationError::NoProcessor(md.tpe.clone()).into());
        }
    }

    Ok(HostMetadata {
        metadata: stages,
        aug: Default::default(),
    })
}
