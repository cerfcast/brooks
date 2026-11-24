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

#[derive(Debug, Clone)]
pub struct GeneratorId {
    pub version: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct AtomHeader {
    pub title: String,
    pub updated: chrono::DateTime<Utc>,
    pub uuid: uuid::Uuid,
    pub generator: GeneratorId,
    pub author: String,
}

#[derive(Debug, Clone)]
pub struct AtomEntry {
    pub title: String,
    pub id: uuid::Uuid,
    pub updated: chrono::DateTime<Utc>,
    pub path: String,
    pub summary: Option<String>,
}

pub trait Atomic {
    type Error;

    fn atomic(&self) -> Result<(AtomHeader, Vec<AtomEntry>), Self::Error>;
}

pub trait Feed {
    type Error;

    fn feed(&self) -> Result<String, Self::Error>;
}

impl<I> Feed for I
where
    I: Atomic,
{
    type Error = I::Error;

    fn feed(&self) -> Result<String, Self::Error> {
        let (header, entries) = self.atomic()?;

        let mut result = format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>
    <feed xmlns=\"http://www.w3.org/2005/Atom\">
        <title type=\"text\">CDNI Logging Feed</title>
        <updated>{}</updated>
        <id>urn:uuid:{}</id>
        <generator version=\"{}\">{}</generator>
        <author><name>{}</name></author>",
            header.updated.to_rfc3339(),
            header.uuid,
            header.generator.version,
            header.generator.name,
            header.author,
        );

        result += &{
            let mut result = "".to_string();
            for entry in entries {
                result += &format!(
                    "
        <entry>
            <title type=\"text\">{}</title>
            <id>urn:uuid:{}</id>
            <updated>{}</updated>
            <content src=\"log/{}\" type=\"application/cdni\" ptype=\"logging-file\"/>
            <summary>{}</summary>
        </entry>",
                    entry.title,
                    entry.id,
                    entry.updated.to_rfc3339(),
                    entry.path,
                    entry.summary.unwrap_or("N/A".to_string())
                );
            }
            result
        };

        result += "
    </feed>\n";

        Ok(result)
    }
}
