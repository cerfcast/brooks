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

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{net, str::FromStr};

use crate::cdni::logging::Error;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LogRecordRaw {
    // Minimal
    /// The timestamp of the first byte read by the OCN application to a best
    /// effort of accuracy.
    #[serde(rename = "timestamp-ns")]
    pub timestamp_ns: u64,
    /// An opaque identifier for an aggregate set of clients, derived from the
    /// client IPv4 or IPv6 address in the request received by the surrogate
    /// and/or other network-level identifying information. See \[RFC7937\] section
    /// 3.4.1.
    #[serde(rename = "c-groupid")]
    pub c_groupid: String,
    /// Request URI prior to any transformation from MI.ProcessingStages. It is
    /// RECOMMENDED to limit the maximum length of this field with an applied
    /// MI.LoggingTransformTruncate, allowing at least 2048 bytes.
    #[serde(rename = "cs-uri")]
    pub cs_uri: String,
    /// The response status code.
    #[serde(rename = "sc-status")]
    pub sc_status: String,
    /// The response size total bytes, including headers.
    #[serde(rename = "sc-total-bytes")]
    pub sc_total_bytes: u64,
    /// The ccid assigned via MI.Grouping or MI.GroupingExtended configuration.
    /// For backward compatibility, s-ccid is aliased to ccid, and either
    /// identifier may be used to refer to this field.
    #[serde(rename = "s-ccid")]
    pub s_ccid: String,
    /// Content header length from the upstream source. The value of this field
    /// is a "best effort". If there are intermediate cache layers, it is
    /// understood that this value may not accurately reflect the complete
    /// upstream picture.
    #[serde(rename = "s-upstream-header-size-bytes")]
    pub s_upstream_header_size_bytes: u64,
    /// Total bytes transferred from the upstream source. The value of this
    /// field is a "best effort". If there are intermediate cache layers, it is
    /// understood that this value may not accurately reflect the complete
    /// upstream picture.
    #[serde(rename = "s-upstream-total-bytes")]
    pub s_upstream_total_bytes: u64,

    // Standard
    /// The elapsed time between reading the first byte of the request and
    /// sending the last byte of the response to a best effort of accuracy.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "s-time-total-ms")]
    pub s_time_total_ms: Option<u64>,
    /// The Session ID assigned by the dCDN. This can be derived from Common
    /// Media Client Data (CMCD) or another mechanism, but it is defined by the
    /// dCDN, and uCDN allocated session identifiers are outside the scope of
    /// this field.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "s-sid")]
    pub s_sid: Option<String>,
    /// Indicates a cache hit or cache miss. For this field, a cache hit is
    /// defined as a request that does not result in content retrieval from the
    /// upstream source.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "s-cached")]
    pub s_cached: Option<bool>,
    /// A short name for the dCDN intended to be used as an identifier, e.g. in
    /// log records. Example: "CDN Example Company" might have s-shortname of
    /// "cdn-ec".
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "s-shortname")]
    pub s_shortname: Option<String>,

    // Extended
    /// The timestamp of the first byte read by the OCN application to a best
    /// effort of accuracy. It is RECOMMENDED that timestamps use Coordinated
    /// Universal Time (UTC).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "timestamp-iso8601")]
    pub timestamp_iso8601: Option<String>,
    /// The HTTP request method.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "cs-method")]
    pub cs_method: Option<String>,
    /// The protocol version (e.g., HTTP/1.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "cs-version")]
    pub cs_version: Option<String>,
    /// Full request URI, including query parameters. If this field is identical
    /// to cs-uri, the field SHOULD be truncated to an empty string.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "cs-uri-transformed")]
    pub cs_uri_transformed: Option<String>,
    /// The client IP address.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "c-ip")]
    pub c_ip: Option<String>,
    /// The server instance ID assigned by the dCDN. This value SHOULD be unique
    /// to a particular instance.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "s-id")]
    pub s_id: Option<String>,
    /// The elapsed time from the first request byte read until the first
    /// response byte sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "s-time-first-ms")]
    pub s_time_first_ms: Option<u64>,
    /// The session duration as determined by the dCDN.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "s-sdur-ms")]
    pub s_sdur_ms: Option<u64>,
    /// The time spent accessing the upstream source. The value of this field is
    /// a "best effort". If there are intermediate cache layers, it is
    /// understood that this value may not accurately reflect the complete
    /// upstream picture.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "s-time-upstream-ms")]
    pub s_time_upstream_ms: Option<u64>,
    /// The response status code returned by the upstream source in the event of
    /// a cache miss.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "s-upstream-status")]
    pub s_upstream_status: Option<String>,
    /// The IP address of the upstream source.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "s-upstream-ip")]
    pub s_upstream_ip: Option<String>,
}

/// See [`LogRecordRaw`].
#[derive(Debug, Clone, Default)]
pub struct LogRecord {
    pub timestamp_ns: u64,
    pub timestamp_iso8601: Option<chrono::DateTime<Utc>>,
    pub s_time_total_ms: Option<u64>,
    pub c_groupid: String,
    pub cs_method: Option<http::method::Method>,
    pub cs_version: Option<String>,
    pub cs_uri: http::Uri,
    pub cs_uri_transformed: Option<http::Uri>,
    pub sc_status: http::StatusCode,
    pub sc_total_bytes: u64,
    pub s_ccid: String,
    pub s_sid: Option<String>,
    pub s_cached: Option<bool>,
    pub c_ip: Option<net::IpAddr>,
    pub s_shortname: Option<String>,
    pub s_id: Option<String>,
    pub s_time_first_ms: Option<u64>,
    pub s_sdur_ms: Option<u64>,
    pub s_time_upstream_ms: Option<u64>,
    pub s_upstream_header_size_bytes: u64,
    pub s_upstream_total_bytes: u64,
    pub s_upstream_status: Option<http::StatusCode>,
    pub s_upstream_ip: Option<net::IpAddr>,
}

impl LogRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new_basic(
        timestamp_ns: u64,
        c_groupid: &str,
        cs_uri: http::Uri,
        sc_status: http::StatusCode,
        sc_total_bytes: u64,
        s_ccid: &str,
        s_upstream_header_size_bytes: u64,
        s_upstream_total_bytes: u64,
    ) -> Self {
        LogRecord {
            timestamp_ns,
            c_groupid: c_groupid.to_string(),
            cs_uri,
            sc_status,
            sc_total_bytes,
            s_ccid: s_ccid.to_string(),
            s_upstream_header_size_bytes,
            s_upstream_total_bytes,
            ..Default::default()
        }
    }

    pub fn is_minimal(&self) -> bool {
        true
    }

    pub fn is_standard(&self) -> bool {
        self.s_time_total_ms.is_some() && self.s_sid.is_some() && self.s_cached.is_some()
    }

    pub fn is_extended(&self) -> bool {
        self.is_standard()
            && self.cs_version.is_some()
            && self.cs_uri_transformed.is_some()
            && self.s_shortname.is_some()
            && self.s_id.is_some()
            && self.s_time_first_ms.is_some()
            && self.s_sdur_ms.is_some()
            && self.s_time_upstream_ms.is_some()
            && self.s_upstream_status.is_some()
            && self.s_upstream_ip.is_some()
    }
}

impl From<&LogRecord> for LogRecordRaw {
    fn from(value: &LogRecord) -> Self {
        LogRecordRaw {
            timestamp_ns: value.timestamp_ns,
            c_groupid: value.c_groupid.clone(),
            cs_uri: value.cs_uri.to_string(),
            sc_status: value.sc_status.as_u16().to_string(),
            sc_total_bytes: value.sc_total_bytes,
            s_ccid: value.s_ccid.clone(),
            s_upstream_header_size_bytes: value.s_upstream_header_size_bytes,
            s_upstream_total_bytes: value.s_upstream_total_bytes,
            ..Default::default()
        }
    }
}

impl TryFrom<LogRecordRaw> for LogRecord {
    type Error = Error;

    fn try_from(value: LogRecordRaw) -> Result<Self, Self::Error> {
        Ok(LogRecord {
            timestamp_ns: value.timestamp_ns,
            c_groupid: value.c_groupid,
            cs_uri: http::Uri::from_str(&value.cs_uri)
                .map_err(|_| Error::InvalidRawValue("cs_uri".to_string(), value.cs_uri))?,
            sc_status: http::StatusCode::from_str(&value.sc_status)
                .map_err(|_| Error::InvalidRawValue("sc_status".to_string(), value.sc_status))?,
            sc_total_bytes: value.sc_total_bytes,
            s_ccid: value.s_ccid,
            s_upstream_header_size_bytes: value.s_upstream_header_size_bytes,
            s_upstream_total_bytes: value.s_upstream_total_bytes,
            ..Default::default()
        })
    }
}
