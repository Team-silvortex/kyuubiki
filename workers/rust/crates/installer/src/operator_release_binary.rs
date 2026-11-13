use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

#[derive(Clone, Copy)]
pub(super) struct Target {
    pub id: &'static str,
    pub filename: &'static str,
    machine: u32,
}

pub(super) fn target(triple: &str) -> Result<Target, String> {
    let (id, filename, machine) = match triple {
        "aarch64-apple-darwin" => ("macos-aarch64", "operator.dylib", 0x0100000c),
        "x86_64-apple-darwin" => ("macos-x86_64", "operator.dylib", 0x01000007),
        "aarch64-unknown-linux-gnu" => ("linux-aarch64", "operator.so", 183),
        "x86_64-unknown-linux-gnu" => ("linux-x86_64", "operator.so", 62),
        "x86_64-pc-windows-msvc" => ("windows-x86_64", "operator.dll", 0x8664),
        _ => {
            return Err(format!(
                "unsupported operator release target {triple}; universal, musl and GNU Windows require distinct compatibility contracts"
            ));
        }
    };
    Ok(Target {
        id,
        filename,
        machine,
    })
}

// Header identity only. Never load uploaded code in a packaging or publication service.
pub(super) fn verify(path: &Path, target: Target) -> Result<(), String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut header = [0u8; 32];
    file.read_exact(&mut header)
        .map_err(|e| format!("truncated binary header: {e}"))?;
    let u16_at = |b: &[u8], i| u16::from_le_bytes([b[i], b[i + 1]]);
    let u32_at = |b: &[u8], i| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let valid = if target.id.starts_with("macos-") {
        u32_at(&header, 0) == 0xfeedfacf
            && u32_at(&header, 4) == target.machine
            && u32_at(&header, 12) == 6
    } else if target.id.starts_with("linux-") {
        let mut tail = [0u8; 32];
        file.read_exact(&mut tail)
            .map_err(|e| format!("truncated ELF header: {e}"))?;
        &header[..7] == b"\x7fELF\x02\x01\x01"
            && u16_at(&header, 16) == 3
            && u16_at(&header, 18) as u32 == target.machine
            && u32_at(&header, 20) == 1
            && u16_at(&tail, 20) == 64
    } else {
        file.seek(SeekFrom::Start(60)).map_err(|e| e.to_string())?;
        let mut offset = [0u8; 4];
        file.read_exact(&mut offset)
            .map_err(|e| format!("truncated DOS header: {e}"))?;
        let offset = u32::from_le_bytes(offset) as u64;
        if !(64..=65536).contains(&offset) {
            return Err("PE header offset exceeds release inspection limit".into());
        }
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| e.to_string())?;
        let mut pe = [0u8; 26];
        file.read_exact(&mut pe)
            .map_err(|e| format!("truncated PE header: {e}"))?;
        &header[..2] == b"MZ"
            && &pe[..4] == b"PE\0\0"
            && u16_at(&pe, 4) as u32 == target.machine
            && u16_at(&pe, 20) >= 112
            && u16_at(&pe, 22) & 0x2000 != 0
            && u16_at(&pe, 24) == 0x20b
            && offset + 24 + u16_at(&pe, 20) as u64
                <= file.metadata().map_err(|e| e.to_string())?.len()
    };
    if !valid {
        return Err(format!(
            "binary header does not match {} shared-library target",
            target.id
        ));
    }
    Ok(())
}
