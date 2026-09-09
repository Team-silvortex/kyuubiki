use crate::migration::ProjectMigrationLimits;
use crate::migration_input::safe_name;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

/// Check the physical directory before ZipArchive builds its name-indexed map.
/// zip 2.x otherwise hides exact duplicate names and allocates from untrusted counts.
pub(crate) fn check_directory(
    file: &mut File,
    limits: &ProjectMigrationLimits,
) -> Result<usize, String> {
    let length = file.metadata().map_err(|e| e.to_string())?.len();
    if length > limits.max_source_bytes || length < 22 {
        return Err("invalid or oversized ZIP file".into());
    }
    let tail_start = length.saturating_sub(65_557);
    file.seek(SeekFrom::Start(tail_start))
        .map_err(|e| e.to_string())?;
    let mut tail = vec![0; (length - tail_start) as usize];
    file.read_exact(&mut tail).map_err(|e| e.to_string())?;
    let position = (0..=tail.len() - 22)
        .rev()
        .find(|&i| {
            tail[i..i + 4] == *b"PK\x05\x06"
                && i + 22 + u16_at(&tail, i + 20) as usize == tail.len()
        })
        .ok_or("ZIP end record missing, truncated, or followed by unrecognized data")?;
    let end = &tail[position..];
    let end_start = tail_start + position as u64;
    if u16_at(end, 4) != 0 || u16_at(end, 6) != 0 || u16_at(end, 8) != u16_at(end, 10) {
        return Err("multi-volume ZIP projects are not supported".into());
    }
    let mut count = u16_at(end, 10) as u64;
    let mut size = u32_at(end, 12) as u64;
    let mut offset = u32_at(end, 16) as u64;
    let mut directory_end = end_start;
    if count == u16::MAX as u64 || size == u32::MAX as u64 || offset == u32::MAX as u64 {
        let locator_start = end_start.checked_sub(20).ok_or("missing ZIP64 locator")?;
        let locator = read_at::<20>(file, locator_start)?;
        if locator[..4] != *b"PK\x06\x07" || u32_at(&locator, 4) != 0 || u32_at(&locator, 16) != 1 {
            return Err("invalid or multi-volume ZIP64 locator".into());
        }
        let record_start = u64_at(&locator, 8);
        let record = read_at::<56>(file, record_start)?;
        let record_size = u64_at(&record, 4);
        if record[..4] != *b"PK\x06\x06"
            || record_size < 44
            || record_start
                .checked_add(12)
                .and_then(|v| v.checked_add(record_size))
                != Some(locator_start)
            || u32_at(&record, 16) != 0
            || u32_at(&record, 20) != 0
            || u64_at(&record, 24) != u64_at(&record, 32)
        {
            return Err("invalid or multi-volume ZIP64 end record".into());
        }
        count = u64_at(&record, 32);
        size = u64_at(&record, 40);
        offset = u64_at(&record, 48);
        directory_end = record_start;
    }
    if count > limits.max_entries as u64 {
        return Err("archive exceeds configured entry limit".into());
    }
    if offset.checked_add(size) != Some(directory_end)
        || count.checked_mul(46).is_none_or(|minimum| minimum > size)
    {
        return Err("invalid ZIP central directory bounds".into());
    }
    let mut cursor = offset;
    let mut names = BTreeSet::new();
    for _ in 0..count {
        if cursor.checked_add(46).is_none_or(|end| end > directory_end) {
            return Err("truncated ZIP central header".into());
        }
        let header = read_at::<46>(file, cursor)?;
        if header[..4] != *b"PK\x01\x02" || u16_at(&header, 34) != 0 {
            return Err("invalid or multi-volume ZIP central header".into());
        }
        if u16_at(&header, 8) & 1 != 0 {
            return Err(
                "encrypted ZIP projects require explicit decryption before migration".into(),
            );
        }
        let name_length = u16_at(&header, 28) as u64;
        cursor = cursor
            .checked_add(46 + name_length + u16_at(&header, 30) as u64 + u16_at(&header, 32) as u64)
            .ok_or("ZIP central offset overflow")?;
        if name_length == 0 || name_length > 1024 || cursor > directory_end {
            return Err("invalid ZIP name length or directory bound".into());
        }
        let mut name = vec![0; name_length as usize];
        file.read_exact(&mut name).map_err(|e| e.to_string())?;
        if !name.is_ascii() && u16_at(&header, 8) & 0x800 == 0 {
            return Err(
                "non-UTF-8 legacy ZIP names require explicit conversion before migration".into(),
            );
        }
        let name = std::str::from_utf8(&name).map_err(|_| "invalid UTF-8 ZIP name")?;
        safe_name(name)?;
        if !names.insert(name.trim_end_matches('/').to_lowercase()) {
            return Err(format!(
                "duplicate or case-colliding physical ZIP entry: {name}"
            ));
        }
    }
    if cursor != directory_end {
        return Err("ZIP entry count differs from physical directory".into());
    }
    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    Ok(count as usize)
}

fn read_at<const N: usize>(file: &mut File, offset: u64) -> Result<[u8; N], String> {
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| e.to_string())?;
    let mut bytes = [0; N];
    file.read_exact(&mut bytes)
        .map_err(|e| format!("truncated ZIP metadata: {e}"))?;
    Ok(bytes)
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}
