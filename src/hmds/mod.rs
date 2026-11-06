// brooks-lib, Copyright 2026, Will Hawkins
//
// This file is part of brooks-lib.
//
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

use std::{collections::HashMap, fs, io, path::PathBuf, str::FromStr};

use chrono::{DateTime, Utc};
use http::{Uri, header::CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    cdni::{
        imd::spec::HostMetadata,
        logging::{self, record::LogRecordRaw},
    },
    environment::scope::Scopes,
    mel::types::Type,
    verifying::verify::CdniVerificationKey,
};

#[cfg(all(unix, not(feature = "disable-domain")))]
use crate::hmds::hmds_domain_communication::{post_log_hmds_domain, query_hmds_domain};

//BEGIN: HMDS Server Configuration

#[derive(Debug, Default, Clone)]
pub struct HmdsServerConfiguration {
    pub(crate) path: Option<PathBuf>,
    pub(crate) url: Option<http::Uri>,
}

impl HmdsServerConfiguration {
    pub fn new_by_sense(config_str: &str) -> io::Result<Self> {
        if config_str.starts_with("http://") || config_str.starts_with("https://") {
            let url = http::Uri::try_from(config_str).map_err(io::Error::other)?;

            Ok(HmdsServerConfiguration {
                url: Some(url),
                ..Default::default()
            })
        } else {
            let path = PathBuf::from(config_str);

            let exists = fs::exists(&path)?;
            if !exists {
                return Err(io::Error::other(format!(
                    "Could not open connection to HMD server over UNIX domain socket at path {config_str}"
                )));
            }

            Ok(HmdsServerConfiguration {
                path: Some(path),
                ..Default::default()
            })
        }
    }

    pub fn new_domain(path: PathBuf) -> Self {
        Self {
            path: Some(path),
            ..Default::default()
        }
    }

    pub fn new_http(http: Uri) -> Self {
        Self {
            url: Some(http),
            ..Default::default()
        }
    }

    pub fn is_http(&self) -> bool {
        self.url.is_some()
    }

    pub fn is_domain(&self) -> bool {
        self.path.is_some()
    }
}

pub type HostMetadataCacheValue = (HostMetadata<CdniVerificationKey>, Scopes<Type>);

pub struct HmdsConfiguration {
    pub(crate) hmds_server: HmdsServerConfiguration,
    pub(crate) hmds_cache: HashMap<String, (chrono::DateTime<Utc>, HostMetadataCacheValue)>,
}

impl From<HmdsServerConfiguration> for HmdsConfiguration {
    fn from(value: HmdsServerConfiguration) -> Self {
        HmdsConfiguration {
            hmds_server: value,
            hmds_cache: HashMap::new(),
        }
    }
}

//BEGIN: HMDS Server Domain Protocol

#[derive(Serialize, Deserialize)]
pub struct ExpirableJsonValue {
    pub expiry: DateTime<Utc>,
    pub host: String,
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HmdsDomainMessageType {
    Query,
    Log,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HmdsDomainMessageQuery {
    pub query: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HmdsDomainMessageLog {
    pub log: LogRecordRaw,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HmdsDomainMessage {
    pub ty: HmdsDomainMessageType,
    pub value: Value,
}

//BEGIN HMDS Domain Support Methods

#[cfg(all(unix, not(feature = "disable-domain")))]
pub mod hmds_domain_support {
    use std::io;

    use tokio::{io::AsyncWriteExt, net::UnixStream};

    pub(crate) async fn hmds_write_entire(s: &mut UnixStream, d: &[u8]) -> io::Result<usize> {
        s.write_u64_le(d.len() as u64).await?;

        let mut already_sent = 0usize;
        loop {
            s.writable().await?;
            match s.try_write(&d[already_sent..]) {
                Ok(n) => {
                    if n == 0 {
                        return Ok(already_sent);
                    }
                    already_sent += n;
                    if already_sent == d.len() {
                        return Ok(already_sent);
                    }
                    continue;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub(crate) async fn hmds_read_entire(s: &mut UnixStream, d: &mut [u8]) -> io::Result<usize> {
        let mut already_read = 0usize;
        loop {
            s.readable().await?;
            match s.try_read(&mut d[already_read..]) {
                Ok(n) => {
                    if n == 0 {
                        return Ok(already_read);
                    }
                    already_read += n;
                    if already_read == d.len() {
                        return Ok(already_read);
                    }
                    continue;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

pub async fn post_log_hmds(
    log: &logging::record::LogRecord,
    config: &HmdsServerConfiguration,
) -> io::Result<()> {
    let log_raw: LogRecordRaw = log.into();

    #[cfg(all(unix, not(feature = "disable-domain")))]
    if config.is_domain() {
        return post_log_hmds_domain(&log_raw, config.path.as_ref().unwrap().as_path()).await;
    }

    if config.is_http() {
        return post_log_hmds_http(&log_raw, config.url.as_ref().unwrap()).await;
    } else {
        Err(io::Error::other(
            "Posting logs to HMDS over HTTP is the only supported option and configuration is missing",
        ))
    }
}

async fn post_log_hmds_http(log: &logging::record::LogRecordRaw, server: &Uri) -> io::Result<()> {
    let post_log_uri =
        reqwest::Url::from_str(&(server.to_string() + "plog/")).map_err(io::Error::other)?;

    let client = reqwest::Client::new();
    let response = client
        .post(post_log_uri)
        .header(CONTENT_TYPE, "application/json")
        .body(serde_json::to_string(log).expect("Could not serialize log record"))
        .send()
        .await
        .map_err(io::Error::other)?;

    response.error_for_status().map_err(io::Error::other)?;

    Ok(())
}

pub async fn query_hmds(
    query: &str,
    config: &HmdsServerConfiguration,
) -> io::Result<Option<(DateTime<Utc>, Value)>> {
    #[cfg(all(unix, not(feature = "disable-domain")))]
    if config.is_domain() {
        return query_hmds_domain(query, config.path.as_ref().unwrap().as_path()).await;
    }

    if config.is_http() {
        return query_hmds_http(query, config.url.as_ref().unwrap()).await;
    } else {
        Err(io::Error::other(
            "Querying HMDS over HTTP is the only supported option and configuration is missing",
        ))
    }
}

#[cfg(all(unix, not(feature = "disable-domain")))]
pub mod hmds_domain_communication {
    use std::{io, path::Path};

    use chrono::{DateTime, Utc};
    use serde_json::Value;
    use tokio::io::AsyncReadExt;
    use tokio::net::UnixStream;

    use crate::hmds::{
        ExpirableJsonValue, HmdsDomainMessageQuery, hmds_domain_support::hmds_read_entire,
    };
    use crate::{
        cdni::logging,
        hmds::{
            HmdsDomainMessage, HmdsDomainMessageLog, HmdsDomainMessageType,
            hmds_domain_support::hmds_write_entire,
        },
    };

    pub(crate) async fn post_log_hmds_domain(
        log: &logging::record::LogRecordRaw,
        server_path: &Path,
    ) -> io::Result<()> {
        let log_message = serde_json::to_string(&HmdsDomainMessage {
            ty: HmdsDomainMessageType::Log,
            value: serde_json::to_value(HmdsDomainMessageLog { log: log.clone() })
                .map_err(std::io::Error::other)?,
        })
        .map_err(std::io::Error::other)?;
        let mut socket = UnixStream::connect(server_path).await?;

        // Wait for the socket to be readable
        socket.writable().await?;

        hmds_write_entire(&mut socket, log_message.as_bytes()).await?;

        Ok(())
    }

    pub(crate) async fn query_hmds_domain(
        query: &str,
        server_path: &Path,
    ) -> io::Result<Option<(DateTime<Utc>, Value)>> {
        let query_message = serde_json::to_string(&HmdsDomainMessage {
            ty: HmdsDomainMessageType::Query,
            value: serde_json::to_value(HmdsDomainMessageQuery {
                query: query.to_string(),
            })
            .map_err(std::io::Error::other)?,
        })
        .map_err(std::io::Error::other)?;
        let mut socket = UnixStream::connect(server_path).await?;

        // Wait for the socket to be readable
        socket.writable().await?;

        hmds_write_entire(&mut socket, query_message.as_bytes()).await?;

        socket.readable().await?;

        let incoming_buffer_size = socket.read_u64_le().await?;

        let mut buf: Vec<u8> = vec![0; incoming_buffer_size as usize];

        hmds_read_entire(&mut socket, &mut buf).await?;

        let s = String::from_utf8(buf).map_err(|_| io::ErrorKind::InvalidData)?;

        if s.is_empty() {
            return Ok(None);
        }

        let result: ExpirableJsonValue =
            serde_json::from_str(&s).map_err(|_| io::ErrorKind::InvalidData)?;

        Ok(Some((result.expiry, result.value)))
    }
}

pub(crate) async fn query_hmds_http(
    query: &str,
    server: &Uri,
) -> io::Result<Option<(DateTime<Utc>, Value)>> {
    let query_uri = reqwest::Url::from_str(&(server.to_string() + &format!("qry/?host={query}")))
        .map_err(io::Error::other)?;

    let query = reqwest::get(query_uri);

    let response = query.await.map_err(io::Error::other)?;

    match response.error_for_status() {
        Ok(response) => {
            let body = response.bytes().await.map_err(std::io::Error::other)?;

            let result: ExpirableJsonValue =
                serde_json::from_slice(&body).map_err(|_| io::ErrorKind::InvalidData)?;

            Ok(Some((result.expiry, result.value)))
        }
        Err(e) => Err(io::Error::other(e)),
    }
}
