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

//! The CDNI Logging Interface

use core::net;
use std::{fmt::Debug, time::Duration};

use chrono::Utc;

use crate::{
    cdni::logging::record::LogRecord,
    hmds::logging::{
        error::Error,
        format::{Format, LoggingSerializable},
        session::{AddSessionRecord, Session, Span},
    },
};

/// Logging record consistent with the format(s) in the Logging Interface specification.
#[derive(Debug, Clone)]
pub struct LiRecord {
    pub datetime: chrono::DateTime<Utc>,
    pub time_taken: Duration,

    pub protocol: http::Version,

    pub c_groupid: String,

    pub s_ip: net::IpAddr,
    pub s_port: u16,
    pub s_hostname: String,

    pub u_uri: http::Uri,

    pub cs_uri: http::Uri,
    pub cs_method: http::method::Method,

    pub sc_status: http::StatusCode,
    pub sc_total_bytes: usize,
    pub sc_entity_bytes: usize,
    pub sc_cached: bool,
}

impl TryFrom<&LogRecord> for LiRecord {
    type Error = Error;

    fn try_from(value: &LogRecord) -> Result<Self, Self::Error> {
        let datetime = *value
            .datetime
            .as_ref()
            .ok_or(Error::MissingLogRecordField("datetime".to_string()))?;

        let time_taken = *value
            .time_taken
            .as_ref()
            .ok_or(Error::MissingLogRecordField("time_taken".to_string()))?;

        let s_ip = *value
            .s_ip
            .as_ref()
            .ok_or(Error::MissingLogRecordField("s_ip".to_string()))?;

        let s_port = *value
            .s_port
            .as_ref()
            .ok_or(Error::MissingLogRecordField("s_port".to_string()))?;

        let s_hostname = value
            .s_hostname
            .as_ref()
            .ok_or(Error::MissingLogRecordField("s_hostname".to_string()))?
            .clone();

        let c_groupid = value
            .c_groupid
            .as_ref()
            .ok_or(Error::MissingLogRecordField("c_groupid".to_string()))?
            .clone();

        let cs_method = value
            .cs_method
            .as_ref()
            .ok_or(Error::MissingLogRecordField("cs_method".to_string()))?
            .clone();

        let cs_uri = value
            .cs_uri
            .as_ref()
            .ok_or(Error::MissingLogRecordField("cs_uri".to_string()))?
            .clone();

        let u_uri = value
            .u_uri
            .as_ref()
            .ok_or(Error::MissingLogRecordField("u_uri".to_string()))?
            .clone();

        let protocol = *value
            .protocol
            .as_ref()
            .ok_or(Error::MissingLogRecordField("protocol".to_string()))?;

        let sc_status = *value
            .sc_status
            .as_ref()
            .ok_or(Error::MissingLogRecordField("sc_status".to_string()))?;

        let sc_total_bytes = *value
            .sc_total_bytes
            .as_ref()
            .ok_or(Error::MissingLogRecordField("sc_total_bytes".to_string()))?;

        let sc_entity_bytes = *value
            .sc_entity_bytes
            .as_ref()
            .ok_or(Error::MissingLogRecordField("sc_entity_bytes".to_string()))?;

        let sc_cached = *value
            .sc_cached
            .as_ref()
            .ok_or(Error::MissingLogRecordField("sc_cached".to_string()))?;

        Ok(LiRecord {
            datetime,
            c_groupid,
            cs_method,
            protocol,
            u_uri,
            cs_uri,
            sc_status,
            sc_total_bytes,
            time_taken,
            s_ip,
            s_port,
            s_hostname,
            sc_entity_bytes,
            sc_cached,
        })
    }
}

/// Metadata object consistent with the format(s) in the Logging Interface specification.
#[derive(Debug, Clone, Default)]
pub struct LiMetadata {
    pub uuid: Option<uuid::Uuid>,
    pub author: Option<String>,
    pub generator: Option<String>,
    pub origin: Option<(Option<String>, Option<net::IpAddr>)>,
    pub start: Option<chrono::DateTime<Utc>>,
    pub stop: Option<chrono::DateTime<Utc>>,
}

pub type LiSession = Session<LiMetadata, LiRecord>;

impl AddSessionRecord<Error> for LiSession {
    fn add_record(&mut self, record: LogRecord) -> Result<(), Error> {
        self.current_span
            .records
            .push(TryInto::<LiRecord>::try_into(&record)?);
        Ok(())
    }
}

impl LoggingSerializable<Error, Span<LiRecord>> for LiSession {
    fn serialize(&self, span: &Span<LiRecord>, format: Format) -> Result<String, Error> {
        let mut result = "".to_string();
        result += &self.serialize_metadata(format.clone())?;
        result += &LiSession::serialize_span(span, format.clone())?;
        Ok(result)
    }

    fn supports_format(&self, format: Format) -> bool {
        // Only support CdniLf1 format now.
        format == Format::CdniLf1
    }
}

impl LiSession {
    fn serialize_metadata(&self, _format: Format) -> Result<String, Error> {
        let mut result = "".to_string();
        if let Some(origin) = &self.metadata.origin {
            let origin = match origin {
                (Some(hostname), None) => hostname.to_string(),
                (None, Some(addr)) => addr.to_string(),
                _ => unreachable!(),
            };
            result += &format!("#claimed-origin:\t{origin}\n");
        }

        if let Some(uuid) = &self.metadata.uuid {
            result += &format!("#uuid:\t{uuid}\n");
        }

        result += "#version:\tcdni/1.0\n";
        result += "#record-type:\tcdni_http_request_v1\n";
        result += "#fields:\tdate\ttime\t\tc-groupid\tcs-uri\t\tu-uri\tprotocol\tsc-status\tsc-total-bytes\ts-ccid\ts-upstream-header-size-bytes\ts-upstream-total-bytes\n";

        Ok(result)
    }

    fn serialize_span(span: &Span<LiRecord>, _format: Format) -> Result<String, Error> {
        let mut result = "".to_string();

        for record in &span.records {
            result += &record.datetime.date_naive().to_string();
            result += "\t";
            result += &record.datetime.time().to_string();
            result += "\t";
            result += &record.c_groupid;
            result += "\t";
            result += &record.cs_uri.to_string();
            result += "\t";
            result += &record.u_uri.to_string();
            result += "\t";
            result += &format!("{:?}", record.protocol);
            result += "\t";
            result += &record.sc_status.as_u16().to_string();
            result += "\t";
            result += &record.sc_total_bytes.to_string();
            result += "\n";
        }

        Ok(result)
    }
}
