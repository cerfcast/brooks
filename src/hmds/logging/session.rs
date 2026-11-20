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

use std::fmt::Debug;

use chrono::Utc;

use crate::cdni::logging::record::LogRecord;

pub trait AddSessionRecord<E>
where
    E: std::error::Error,
{
    fn add_record(&mut self, record: LogRecord) -> Result<(), E>;
}

/// A group of log records [`R`] bounded by time.
#[derive(Debug, Clone, Default)]
pub struct Span<R> {
    pub start: chrono::DateTime<Utc>,
    pub stop: Option<chrono::DateTime<Utc>>,
    pub records: Vec<R>,
}

/// An active sync of log records ([`R`]) divided into [`Span`]s.
#[derive(Debug, Clone, Default)]
pub struct Session<M, R> {
    pub metadata: M,
    pub current_span: Span<R>,
}
