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

    pub s_ip: Option<net::IpAddr>,
    pub s_port: Option<u16>,
    pub s_hostname: Option<String>,
    pub s_ccid: Option<String>,
    pub s_sid: Option<String>,

    pub u_uri: http::Uri,

    pub cs_uri: Option<http::Uri>,
    pub cs_method: http::method::Method,

    pub sc_status: http::StatusCode,
    pub sc_total_bytes: usize,
    pub sc_entity_bytes: Option<usize>,
    pub sc_cached: Option<bool>,
}

impl TryFrom<&LogRecord> for LiRecord {
    type Error = Error;

    fn try_from(value: &LogRecord) -> Result<Self, Self::Error> {
        // Required
        let datetime = *value
            .datetime
            .as_ref()
            .ok_or(Error::MissingLogRecordField("datetime".to_string()))?;

        // Required
        let time_taken = *value
            .time_taken
            .as_ref()
            .ok_or(Error::MissingLogRecordField("time_taken".to_string()))?;

        // Required
        let c_groupid = value
            .c_groupid
            .as_ref()
            .ok_or(Error::MissingLogRecordField("c_groupid".to_string()))?
            .clone();

        // Optional
        let s_ip = value.s_ip;

        // Optional
        let s_port = value.s_port;

        // Optional
        let s_hostname = value.s_hostname.clone();

        // Optional
        let s_ccid = value.s_ccid.clone();

        // Optional
        let s_sid = value.s_sid.clone();

        // Required
        let cs_method = value
            .cs_method
            .as_ref()
            .ok_or(Error::MissingLogRecordField("cs_method".to_string()))?
            .clone();

        // Optional
        let cs_uri = value.cs_uri.clone();

        // Required
        let u_uri = value
            .u_uri
            .as_ref()
            .ok_or(Error::MissingLogRecordField("u_uri".to_string()))?
            .clone();

        // Required
        let protocol = *value
            .protocol
            .as_ref()
            .ok_or(Error::MissingLogRecordField("protocol".to_string()))?;

        // Required
        let sc_status = *value
            .sc_status
            .as_ref()
            .ok_or(Error::MissingLogRecordField("sc_status".to_string()))?;

        // Required
        let sc_total_bytes = *value
            .sc_total_bytes
            .as_ref()
            .ok_or(Error::MissingLogRecordField("sc_total_bytes".to_string()))?;

        // Optional
        let sc_entity_bytes = value.sc_entity_bytes;

        // Optional
        let sc_cached = value.sc_cached;

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
            s_ccid,
            s_sid,
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
        result += "fields: ";
        result += &[
            "date",
            "time",
            "time-taken",
            "protocol",
            "c-groupid",
            "s-ip",
            "s-port",
            "s-hostname",
            "s-ccid",
            "s-sid",
            "u-uri",
            "cs-uri",
            "cs-method",
            "sc-status",
            "sc-total-bytes",
            "sc-entity-bytes",
            "sc-cached",
        ]
        .join("\t");
        result += "\n";

        Ok(result)
    }

    fn serialize_span(span: &Span<LiRecord>, _format: Format) -> Result<String, Error> {
        let mut result = "".to_string();

        for record in &span.records {
            result += &record.datetime.date_naive().to_string();

            result += "\t";
            result += &record.datetime.time().to_string();

            result += "\t";
            result += &format!("{:.4}", record.time_taken.as_secs_f64());

            result += "\t";
            result += &format!("{:?}", record.protocol);

            result += "\t";
            result += &record.c_groupid;

            result += "\t";
            result += &record
                .s_ip
                .as_ref()
                .map(|ip| ip.to_string())
                .unwrap_or("-".to_string());

            result += "\t";
            result += &record
                .s_port
                .as_ref()
                .map(|port| port.to_string())
                .unwrap_or("-".to_string());

            result += "\t";
            result += &record
                .s_hostname
                .as_ref()
                .map(|hostname| hostname.to_string())
                .unwrap_or("-".to_string());

            result += "\t";
            result += &record
                .s_ccid
                .as_ref()
                .map(|ccid| ccid.to_string())
                .unwrap_or("-".to_string());

            result += "\t";
            result += &record
                .s_sid
                .as_ref()
                .map(|sid| sid.to_string())
                .unwrap_or("-".to_string());

            result += "\t";
            result += &record.u_uri.to_string();

            result += "\t";
            result += &record
                .cs_uri
                .as_ref()
                .map(|u| u.to_string())
                .unwrap_or("-".to_string());

            result += "\t";
            result += record.cs_method.as_ref();

            result += "\t";
            result += &record.sc_status.as_u16().to_string();

            result += "\t";
            result += &record.sc_total_bytes.to_string();

            result += "\t";
            result += &record
                .sc_entity_bytes
                .as_ref()
                .map(|entity_bytes| entity_bytes.to_string())
                .unwrap_or("-".to_string());

            result += "\t";
            result += &record
                .sc_cached
                .as_ref()
                .map(|cached| cached.to_string())
                .unwrap_or("-".to_string());

            result += "\n";
        }

        Ok(result)
    }
}
