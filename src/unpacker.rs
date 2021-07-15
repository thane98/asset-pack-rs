use anyhow::Context;
use mila::{BinArchive, BinArchiveWriter};
use std::collections::HashMap;

pub fn unpack(archive: &BinArchive) -> anyhow::Result<String> {
    let mut pointers: HashMap<usize, usize> = HashMap::new();
    let mut pointer_destinations: HashMap<usize, usize> = HashMap::new();
    for addr in (0..archive.size()).step_by(4) {
        if let Some(ptr) = archive.read_pointer(addr)? {
            let id = if let Some(id) = pointer_destinations.get(&ptr) {
                *id
            } else {
                pointer_destinations.len()
            };
            pointer_destinations.insert(ptr, id);
            pointers.insert(addr, id);
        }
    }

    let mut lines: Vec<String> = Vec::new();
    for addr in (0..archive.size()).step_by(4) {
        if let Some(id) = pointer_destinations.get(&addr) {
            lines.push(format!("DEST: {}", id));
        }
        if let Some(labels) = archive.read_labels(addr)? {
            for label in labels {
                lines.push(format!("LABEL: {}", label));
            }
        }
        if let Some(id) = pointers.get(&addr) {
            lines.push(format!("SRC: {}", id));
        } else if let Some(text) = archive.read_string(addr)? {
            lines.push(text);
        } else {
            let data = archive.read_bytes(addr, 4)?;
            lines.push(format!(
                "0x{:02X}{:02X}{:02X}{:02X}",
                data[0], data[1], data[2], data[3]
            ));
        }
    }
    Ok(lines.join("\n"))
}

// Modified from: https://stackoverflow.com/questions/52987181/how-can-i-convert-a-hex-string-to-a-u8-slice
fn decode_hex(s: &str) -> anyhow::Result<Vec<u8>> {
    if s.len() % 2 != 0 {
        Err(anyhow::anyhow!("Hex string has odd length"))
    } else {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.into()))
            .collect()
    }
}

pub fn pack(text: &str) -> anyhow::Result<BinArchive> {
    let lines: Vec<String> = text.split("\n").map(|l| l.trim().to_owned()).collect();
    let size = lines
        .iter()
        .filter(|l| !l.starts_with("LABEL:") && !l.starts_with("DEST:") && !l.is_empty())
        .count();

    let mut pointers: HashMap<String, usize> = HashMap::new();
    let mut pointer_sources: Vec<(usize, String)> = Vec::new();
    let mut archive = BinArchive::new();
    archive.allocate_at_end(size * 4);
    let mut writer = BinArchiveWriter::new(&mut archive, 0);
    for i in 0..lines.len() {
        let line = &lines[i];
        if line.starts_with("DEST:") {
            let pointer_id = (&line[5..]).trim().to_owned();
            pointers.insert(pointer_id, writer.tell());
        } else if line.starts_with("SRC:") {
            let pointer_id = (&line[4..]).trim().to_owned();
            pointer_sources.push((writer.tell(), pointer_id));
            writer.write_u32(0)?;
        } else if line.starts_with("LABEL:") {
            writer.write_label((&line[6..]).trim())?;
        } else if line.starts_with("0x") {
            let bytes = decode_hex(&line[2..])
                .with_context(|| format!("Bad hex string at line {}", i + 1))?;
            if bytes.len() != 4 {
                return Err(anyhow::anyhow!(
                    "Hex string has incorrect length at line {}",
                    i + 1
                ));
            }
            writer.write_bytes(&bytes)?;
        } else {
            writer.write_string(Some(&line))?;
        }
    }
    for (addr, pointer_id) in pointer_sources {
        if let Some(dest) = pointers.get(&pointer_id) {
            println!("{:X}, {:X}, {}", addr, dest, pointer_id);
            archive.write_pointer(addr, Some(*dest))?;
        } else {
            return Err(anyhow::anyhow!("Unresolved pointer {}", pointer_id));
        }
    }
    Ok(archive)
}
