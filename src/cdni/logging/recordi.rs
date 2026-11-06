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

use std::str::FromStr;

use crate::cdni::logging::{
    Error,
    record::{LogRecord, LogRecordRaw},
};

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
