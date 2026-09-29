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

use std::error::Error;
use std::fmt::Display;

use serde::Serialize;

#[cfg(feature = "mi_source")]
use crate::cdni::{
    gmd::spec::TypedSource,
    processors::source::{SourceMetadataAnalyzer, SourceVerificationKey},
};

#[cfg(feature = "mi_ps")]
use crate::cdni::{
    processors::ps::PsMetadataAnalyzer,
    ps::{spec::TypedStage, verify::PsVerificationKey},
};

use crate::{
    cdni::{
        gmd::spec::TypedGenericMetadata, gmdp::Verifier, md::spec::HostMetadata,
        processors::SimpleProcessorsAnalysisContext, ps::verify::PsVerificationError,
    },
    environment::scope::Scopes,
    mel::types::Type,
};

#[derive(Debug, Clone, Default)]
pub enum HostMetadataVerificationError {
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
    #[cfg(feature = "mi_ps")]
    Stage(Box<TypedStage<PsVerificationKey>>),
    #[cfg(feature = "mi_source")]
    Source(Box<TypedSource<SourceVerificationKey>>),
    #[default]
    None,
}

impl From<&CdniVerificationKey> for CdniVerifiedMetadataTypes {
    fn from(value: &CdniVerificationKey) -> Self {
        match value {
            #[cfg(feature = "mi_ps")]
            CdniVerificationKey::Stage(_) => CdniVerifiedMetadataTypes::ProcessingStages,
            #[cfg(feature = "mi_source")]
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

impl Error for HostMetadataVerificationError {}

impl Display for HostMetadataVerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostMetadataVerificationError::NoError => write!(f, "No error!"),
            HostMetadataVerificationError::NotProcessingStage => {
                write!(f, "Value was not a processing stage!")
            }
            HostMetadataVerificationError::NoProcessor(tpe) => {
                write!(f, "No processor registered for Metadata with type {tpe}")
            }
            HostMetadataVerificationError::PsVerificationError(pse) => write!(f, "{pse}"),
            HostMetadataVerificationError::JsonError(je) => write!(f, "JSON error: {je}"),
        }
    }
}

pub fn verify_metadata(
    metadata: &HostMetadata<()>,
    scopes: Scopes<Type>,
) -> Result<HostMetadata<CdniVerificationKey>, Box<HostMetadataVerificationError>> {
    let mut stages: Vec<TypedGenericMetadata<CdniVerificationKey>> = vec![];

    #[allow(clippy::vec_init_then_push)]
    #[allow(unused_mut)]
    let mut processors: Vec<
        &dyn Verifier<SimpleProcessorsAnalysisContext, HostMetadataVerificationError>,
    > = vec![];

    #[allow(clippy::vec_init_then_push)]
    #[cfg(feature = "mi_ps")]
    processors.push(&PsMetadataAnalyzer {}
        as &dyn Verifier<SimpleProcessorsAnalysisContext, HostMetadataVerificationError>);

    #[allow(clippy::vec_init_then_push)]
    #[cfg(feature = "mi_source")]
    processors.push(&SourceMetadataAnalyzer {}
        as &dyn Verifier<SimpleProcessorsAnalysisContext, HostMetadataVerificationError>);

    for md in &metadata.metadata {
        let mut no_processor = true;
        for processor in &processors {
            if !processor.type_name(&md.tpe) {
                continue;
            }

            no_processor = false;

            let (_, analyzed) = processor.analyze(
                md,
                SimpleProcessorsAnalysisContext {
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
            return Err(HostMetadataVerificationError::NoProcessor(md.tpe.clone()).into());
        }
    }

    Ok(HostMetadata {
        metadata: stages,
        aug: Default::default(),
    })
}
