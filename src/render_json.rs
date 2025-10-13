use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

pub fn write_conversation_json(raw: &Value, pretty: usize, path: &Path) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    if pretty > 0 {
        let indent = vec![b' '; pretty];
        let formatter = serde_json::ser::PrettyFormatter::with_indent(indent.as_slice());
        {
            let mut serializer = serde_json::Serializer::with_formatter(&mut writer, formatter);
            raw.serialize(&mut serializer)?;
        }
    } else {
        serde_json::to_writer(&mut writer, raw)?;
    }

    writer.flush()?;
    drop(writer);

    let mut file = OpenOptions::new().append(true).open(path)?;
    file.write_all(b"\n")?;
    file.flush()?;

    Ok(())
}
