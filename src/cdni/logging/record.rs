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

use chrono::{DateTime, Utc};
use http::Version;
use serde::{Deserialize, Serialize};
use std::{
    net::{self, IpAddr},
    str::FromStr,
    time::Duration,
};

use crate::cdni::logging::Error;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LogRecordRaw {
    pub date: Option<String>,
    pub time: Option<String>,
    #[serde(rename = "time-taken")]
    pub time_taken: Option<usize>,

    #[serde(rename = "protocol")]
    pub protocol: Option<String>,

    #[serde(rename = "c-groupid")]
    pub c_groupid: Option<String>,

    #[serde(rename = "s-ip")]
    pub s_ip: Option<String>,
    #[serde(rename = "s-port")]
    pub s_port: Option<usize>,
    #[serde(rename = "s-hostname")]
    pub s_hostname: Option<String>,

    #[serde(rename = "u-uri")]
    pub u_uri: Option<String>,

    #[serde(rename = "cs-uri")]
    pub cs_uri: Option<String>,
    #[serde(rename = "cs-method")]
    pub cs_method: Option<String>,

    #[serde(rename = "sc-status")]
    pub sc_status: Option<String>,
    #[serde(rename = "sc-total-bytes")]
    pub sc_total_bytes: Option<usize>,
    #[serde(rename = "sc-entity-bytes")]
    pub sc_entity_bytes: Option<usize>,
    #[serde(rename = "sc-cached")]
    pub sc_cached: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct LogRecordMetadata {
    pub complete: chrono::DateTime<Utc>,
    pub log: LogRecord,
}

/// See [`LogRecordRaw`].
#[derive(Debug, Clone, Default)]
pub struct LogRecord {
    pub datetime: Option<chrono::DateTime<Utc>>,
    pub time_taken: Option<Duration>,

    pub protocol: Option<http::Version>,

    pub c_groupid: Option<String>,

    pub s_ip: Option<net::IpAddr>,
    pub s_port: Option<u16>,
    pub s_hostname: Option<String>,

    pub u_uri: Option<http::Uri>,

    pub cs_uri: Option<http::Uri>,
    pub cs_method: Option<http::method::Method>,

    pub sc_status: Option<http::StatusCode>,
    pub sc_total_bytes: Option<usize>,
    pub sc_entity_bytes: Option<usize>,
    pub sc_cached: Option<bool>,
}

impl From<&LogRecord> for LogRecordRaw {
    fn from(value: &LogRecord) -> Self {
        LogRecordRaw {
            date: value.datetime.map(|f| f.date_naive().to_string()),
            time: value.datetime.map(|f| f.time().to_string()),
            time_taken: value.time_taken.map(|f| f.as_secs() as usize),
            protocol: value.protocol.as_ref().map(|f| format!("{:?}", f)),
            c_groupid: value.c_groupid.clone(),

            s_ip: value.s_ip.map(|f| f.to_string()),
            s_port: value.s_port.map(|f| f as usize),
            s_hostname: value.s_hostname.as_ref().map(|f| f.to_string()),

            u_uri: value.u_uri.as_ref().map(|f| f.to_string()),

            cs_uri: value.cs_uri.as_ref().map(|f| f.to_string()),
            cs_method: value.cs_method.as_ref().map(|f| f.to_string()),

            sc_status: value.sc_status.map(|f| f.as_u16().to_string()),
            sc_total_bytes: value.sc_total_bytes,
            sc_entity_bytes: value.sc_entity_bytes,
            sc_cached: value.sc_cached,
        }
    }
}

struct ParsableHttpProtocol(http::Version);

impl FromStr for ParsableHttpProtocol {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pieces = s.split("/").collect::<Vec<_>>();

        if pieces.len() != 2 {
            return Err(Error::InvalidRawValue(
                "protocol".to_string(),
                "Missing / delimiter".to_string(),
            ));
        }

        let http = pieces[0].trim();

        if http != "HTTP" {
            return Err(Error::InvalidRawValue(
                "protocol".to_string(),
                "Missing HTTP before the / delimiter".to_string(),
            ));
        }

        let number = pieces[1].trim();

        let number = match number.parse::<f64>() {
            Ok(number) => number,
            Err(e) => {
                return Err(Error::InvalidRawValue(
                    "protocol".to_string(),
                    format!("Could not parse {number} into a version number: {e}"),
                ));
            }
        };

        match number {
            0.9 => Ok(ParsableHttpProtocol(Version::HTTP_09)),
            1.0 => Ok(ParsableHttpProtocol(Version::HTTP_10)),
            1.1 => Ok(ParsableHttpProtocol(Version::HTTP_11)),
            2.0 => Ok(ParsableHttpProtocol(Version::HTTP_2)),
            v => Err(Error::InvalidRawValue(
                "protocol".to_string(),
                format!("{v} is not a valid HTTP version number"),
            )),
        }
    }
}

impl TryFrom<LogRecordRaw> for LogRecord {
    type Error = Error;

    fn try_from(value: LogRecordRaw) -> Result<Self, Self::Error> {
        Ok(LogRecord {
            datetime: match (value.date, value.time) {
                (Some(date), Some(time)) => Some(
                    DateTime::parse_from_rfc3339(&format!("{}T{}Z", date, time))
                        .map_err(|e| Error::InvalidRawValue("time".to_string(), e.to_string()))?
                        .into(),
                ),
                _ => None,
            },
            time_taken: value.time_taken.map(|f| Duration::from_secs(f as u64)),

            protocol: match value.protocol {
                Some(protocol) => Some(protocol.parse::<ParsableHttpProtocol>()?.0),
                None => None,
            },
            c_groupid: value.c_groupid,

            s_hostname: value.s_hostname,
            s_ip: match value.s_ip {
                Some(ip) => Some(
                    ip.parse::<IpAddr>()
                        .map_err(|f| Error::InvalidRawValue("s_ip".to_string(), f.to_string()))?,
                ),
                None => None,
            },
            s_port: match value.s_port {
                Some(port) => Some(
                    TryInto::<u16>::try_into(port)
                        .map_err(|f| Error::InvalidRawValue("s_ip".to_string(), f.to_string()))?,
                ),
                None => None,
            },
            u_uri: match value.u_uri.map(|f| http::Uri::from_str(&f)) {
                Some(res) => Some(
                    res.map_err(|f| Error::InvalidRawValue("u_uri".to_string(), f.to_string()))?,
                ),
                None => None,
            },

            cs_uri: match value.cs_uri.map(|f| http::Uri::from_str(&f)) {
                Some(res) => Some(
                    res.map_err(|f| Error::InvalidRawValue("cs_uri".to_string(), f.to_string()))?,
                ),
                None => None,
            },

            cs_method: match value.cs_method.map(|f| http::Method::from_str(&f)) {
                Some(res) => {
                    Some(res.map_err(|f| {
                        Error::InvalidRawValue("cs_method".to_string(), f.to_string())
                    })?)
                }
                None => None,
            },

            sc_status: match value.sc_status.map(|f| http::StatusCode::from_str(&f)) {
                Some(res) => {
                    Some(res.map_err(|f| {
                        Error::InvalidRawValue("sc_status".to_string(), f.to_string())
                    })?)
                }
                None => None,
            },
            sc_total_bytes: value.sc_total_bytes,
            sc_entity_bytes: value.sc_entity_bytes,

            sc_cached: value.sc_cached,
        })
    }
}
