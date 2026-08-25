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

use std::{collections::HashMap, io, path::{Path, PathBuf}};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

use crate::cdni::{spec::HostMetadata, verify::HostMetadataVerificationKey};

pub struct HmdsConfiguration {
    pub(crate) hmds_path: PathBuf,
    pub(crate) hmds_cache: HashMap<
        String,
        (
            chrono::DateTime<Utc>,
            HostMetadata<HostMetadataVerificationKey>,
        ),
    >,
}

#[derive(Serialize, Deserialize)]
pub struct ExpirableJsonValue {
    pub expiry: DateTime<Utc>,
    pub value: Value,
}

async fn hmds_write_entire(s: &mut UnixStream, d: &[u8]) -> io::Result<usize> {
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

async fn hmds_read_entire(s: &mut UnixStream, d: &mut [u8]) -> io::Result<usize> {
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

pub async fn query_hmds(
    query: &str,
    server_path: &Path,
) -> io::Result<Option<(DateTime<Utc>, Value)>> {
    let mut socket = UnixStream::connect(server_path).await?;

    // Wait for the socket to be readable
    socket.writable().await?;

    hmds_write_entire(&mut socket, query.as_bytes()).await?;

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
