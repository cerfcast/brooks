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

use crate::{
    ffi::{
        nginx_proxy::NginxTransformError, ngx_buf_s, ngx_log_error_core, ngx_log_s, ngx_pcalloc,
        ngx_pool_s, ngx_str_t,
    },
    logging::{LogLevel, LogMsgs},
};

/// Create an owned String from an nginx string.
pub(crate) unsafe fn from_nginx_str(s: ngx_str_t) -> String {
    let mut v = vec![0u8; s.len];

    let mut d = s.data;
    for y in v.iter_mut().take(s.len) {
        *y = *d;
        d = d.wrapping_add(1);
    }

    String::from_utf8_unchecked(v)
}

/// Create an nginx string from a str.
///
/// The function will allocate space for the nginx string from the given pool.
pub(crate) unsafe fn to_nginx_str(s: &str, pool: *mut ngx_pool_s) -> ngx_str_t {
    let len = s.len();
    let (data, _) = copy_to_pool(s.as_bytes(), pool);

    ngx_str_t { len, data }
}

/// Copy the given bytes into newly allocated space in the given pool.
pub(crate) unsafe fn copy_to_pool(s: &[u8], pool: *mut ngx_pool_s) -> (*mut u8, *mut u8) {
    let len = s.len();
    let data = ngx_pcalloc(pool, len) as *mut u8;
    let mut datai = data;
    for y in s {
        *datai = *y;
        datai = datai.wrapping_add(1);
    }

    (data, datai)
}

/// Copy the given bytes into newly allocated space from the given pool and framed in a buffer.
pub(crate) unsafe fn to_nginx_buf(
    s: &[u8],
    pool: *mut ngx_pool_s,
) -> Result<*mut ngx_buf_s, NginxTransformError> {
    let buf = ngx_pcalloc(pool, size_of::<ngx_buf_s>()) as *mut ngx_buf_s;
    let buf = buf.as_mut().ok_or(NginxTransformError::BadMemory)?;

    // Say that it is a memory buffer.
    buf.set_memory(1);
    (buf.pos, buf.last) = copy_to_pool(s, pool);

    buf.set_last_in_chain(1);
    buf.set_last_buf(1);

    Ok(buf)
}

#[derive(Debug, Clone)]
#[repr(usize)]
enum NginxLogLevels {
    StdErr = 0,
    Emerg = 1,
    Alert = 2,
    Crit = 3,
    Err = 4,
    Warn = 5,
    Notice = 6,
    Info = 7,
    Debug = 8,
}

impl From<LogLevel> for NginxLogLevels {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Trace => NginxLogLevels::Notice,
            LogLevel::Debug => NginxLogLevels::Debug,
            LogLevel::Warn => NginxLogLevels::Warn,
            LogLevel::Error => NginxLogLevels::Err,
        }
    }
}

/// Copy the given bytes into newly allocated space from the given pool and framed in a buffer.
pub(crate) unsafe fn log_nginx_msgs(nxl: *mut ngx_log_s, log: &LogMsgs) {
    for msg in log.use_msgs() {
        let msg_contents = msg.msg();
        let nginx_msg = ngx_str_t {
            len: msg_contents.len(),
            data: msg_contents.as_ptr() as *mut u8,
        };

        ngx_log_error_core(
            Into::<NginxLogLevels>::into(msg.level()) as usize,
            nxl,
            0,
            c"%V".as_ptr(),
            &nginx_msg,
        );
    }
}
