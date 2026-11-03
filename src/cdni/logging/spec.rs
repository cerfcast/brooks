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

//! Logging (and related) metadata definitions

use crate::macros::IntoCdniMetadata;
use brooks_macros::CdniMetadata;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::fmt::Debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoggingContainerType {
    #[serde(rename = "json_v1")]
    JsonV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingContainerMetadata {
    #[serde(rename = "container-type")]
    pub container_type: LoggingContainerType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoggingRecordType {
    #[serde(rename = "opencaching_minimal_json_v1")]
    MinimalJsonV1,
    #[serde(rename = "opencaching_standard_json_v1")]
    StandardJsonV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoggingTransportType {
    #[serde(rename = "MI.LoggingTransportKafka")]
    Kafka,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingTransport {
    #[serde(rename = "transport-type")]
    pub transport_type: LoggingTransportType,
    #[serde(rename = "record-type")]
    pub record_type: LoggingRecordType,
    #[serde(rename = "container-metadata")]
    pub container_metadata: LoggingContainerMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingMetadata<A: Debug + Clone + Default> {
    pub transports: Vec<LoggingTransport>,

    #[serde(skip)]
    pub aug: A,
}

#[derive(Debug, Clone, Serialize, Deserialize, CdniMetadata)]
pub struct TypedLoggingMetadata<A: Debug + Clone + Default> {
    #[serde(rename = "generic-metadata-type")]
    pub tpe: String,
    #[serde(rename = "generic-metadata-value")]
    pub value: LoggingMetadata<A>,
}
