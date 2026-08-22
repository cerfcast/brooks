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

use crate::common::GrammarLocation;

#[cfg(feature = "json")]
use serde::Serialize;

pub trait Formatter<T> {
    fn format(&self, value: &T) -> String;
}

#[derive(Default, Debug, Clone)]
pub struct LogMsgFormatter {
    pub newline: bool,
    pub show_level: bool,
}

impl Formatter<LogMsg> for LogMsgFormatter {
    fn format(&self, value: &LogMsg) -> String {
        if let Some(location) = &value.location {
            format!("{location}: {}", value.msg)
        } else {
            value.msg.clone()
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub enum LogLevel {
    Trace,
    Debug,
    Warn,
    #[default]
    Error,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct LogMsg {
    msg: String,
    location: Option<GrammarLocation>,
    level: LogLevel,
}

impl LogMsg {
    pub fn new(msg: &str, level: LogLevel) -> Self {
        Self::new_with_location(msg, level, None)
    }

    pub fn new_with_location(
        msg: &str,
        level: LogLevel,
        location: Option<GrammarLocation>,
    ) -> Self {
        LogMsg {
            msg: msg.to_string(),
            level,
            location,
        }
    }

    pub fn pretty(&self, _formatter: &impl Formatter<LogMsg>) -> String {
        _formatter.format(self)
    }

    pub fn level(&self) -> LogLevel {
        self.level.clone()
    }

    pub fn msg(&self) -> String {
        self.msg.clone()
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct LogMsgs {
    msgs: Vec<LogMsg>,
    level: LogLevel,
    prefix: Option<String>,
}

impl LogMsgs {
    pub fn new(level: LogLevel) -> Self {
        LogMsgs {
            msgs: vec![],
            level,
            prefix: None,
        }
    }

    pub fn new_with_prefix(prefix: &str, level: LogLevel) -> Self {
        LogMsgs {
            msgs: vec![],
            level,
            prefix: Some(prefix.to_string()),
        }
    }

    pub fn update_level(&self, new_level: LogLevel) -> Self {
        LogMsgs {
            msgs: self.msgs.clone(),
            level: new_level,
            prefix: self.prefix.clone(),
        }
    }

    pub fn log(&self, msg: LogMsg) -> Self {
        if msg.level >= self.level {
            let msg = if let Some(prefix) = &self.prefix {
                LogMsg {
                    msg: format!("{prefix}: {}", msg.msg),
                    location: msg.location,
                    level: msg.level,
                }
            } else {
                msg
            };
            let mut ns = self.clone();
            ns.msgs.push(msg);
            ns
        } else {
            self.clone()
        }
    }

    pub fn count(&self) -> usize {
        self.msgs.len()
    }

    pub fn msgs(&self, formatter: &LogMsgFormatter) -> String {
        self.msgs
            .iter()
            .map(|msg| msg.pretty(formatter))
            .collect::<Vec<_>>()
            .join(if formatter.newline { "\n" } else { ";" })
    }

    pub fn use_msgs(&self) -> &Vec<LogMsg> {
        &self.msgs
    }
}

macro_rules! emit_ {
    ($nameloc:ident, $name:ident, $level:path) => {
        #[allow(unused_macros)]
        macro_rules! $nameloc {
            ($log:expr, $loc:expr, $msg:expr ) => {
                $log.log(LogMsg::new_with_location($msg, $level, Some($loc)))
            };
        }

        #[allow(unused_macros)]
        macro_rules! $name {
            ($log:expr, $msg:expr ) => {
                $log.log(LogMsg::new($msg, $level))
            };
        }
    };
}

emit_!(trace_with_loc, trace, LogLevel::Trace);
emit_!(debug_with_loc, debug, LogLevel::Debug);
emit_!(warn_with_loc, warn, LogLevel::Warn);
emit_!(error_with_loc, error, LogLevel::Error);
