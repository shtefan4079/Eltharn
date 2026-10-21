use el_extension_api::{PackagePath, package_path::is_manifest_file_name};
use std::collections::{BTreeMap, HashSet};
use std::io::{Cursor, Read};
pub const MAX_ARCHIVE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_PAYLOAD_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_FILES: usize = 4097;
const MANIFEST: &str = el_extension_api::package_path::MANIFEST_FILE_NAME;
const MANIFEST_MAX: usize = 262144;
fn parents(p: &str) -> impl Iterator<Item = &str> {
    p.match_indices('/').map(move |(i, _)| &p[..i])
}
fn prefix_child(parent: &str, child: &str) -> bool {
    child.len() > parent.len()
        && child.as_bytes()[parent.len()] == b'/'
        && child.starts_with(parent)
}
fn u16le(b: &[u8], o: usize) -> Result<u16, String> {
    if o.checked_add(2).map_or(true, |e| e > b.len()) {
        return Err("truncated".into());
    }
    Ok(u16::from_le_bytes([b[o], b[o + 1]]))
}
fn u32le(b: &[u8], o: usize) -> Result<u32, String> {
    if o.checked_add(4).map_or(true, |e| e > b.len()) {
        return Err("truncated".into());
    }
    Ok(u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]))
}
fn entry_count(bytes: &[u8]) -> Result<usize, String> {
    if bytes.len() > 33_554_432 {
        return Err("too large".into());
    }
    if bytes.len() < 22 {
        return Err("truncated".into());
    }
    if bytes.len() < 4
        || bytes[0] != 0x50
        || bytes[1] != 0x4B
        || bytes[2] != 0x03
        || bytes[3] != 0x04
    {
        return Err("no local at 0".into());
    }
    let lo = bytes.len().saturating_sub(65_557);
    let mut st = bytes.len().checked_sub(22).ok_or("truncated".to_string())?;
    let mut eocd: Option<usize> = None;
    loop {
        if bytes[st] == 0x50
            && bytes[st + 1] == 0x4B
            && bytes[st + 2] == 0x05
            && bytes[st + 3] == 0x06
        {
            let cl = u16le(bytes, st + 20)? as usize;
            if st.checked_add(22).and_then(|v| v.checked_add(cl)) == Some(bytes.len()) {
                eocd = Some(st);
                break;
            }
        }
        if st <= lo || st == 0 {
            break;
        }
        st -= 1;
    }
    let e = eocd.ok_or("no eocd".to_string())?;
    let disk = u16le(bytes, e + 4)?;
    let cdisk = u16le(bytes, e + 6)?;
    let on_disk = u16le(bytes, e + 8)?;
    let total = u16le(bytes, e + 10)?;
    let csize = u32le(bytes, e + 12)?;
    let coff = u32le(bytes, e + 16)?;
    if disk != 0 || cdisk != 0 {
        return Err("multidisk".into());
    }
    if total == 0 || total == 0xFFFF || on_disk == 0xFFFF {
        return Err("zip64/count".into());
    }
    if (total as usize) > 4224 {
        return Err("too many".into());
    }
    if on_disk != total {
        return Err("count mismatch".into());
    }
    if csize == 0xFFFF_FFFF || coff == 0xFFFF_FFFF {
        return Err("zip64".into());
    }
    let co = coff as usize;
    let cs = csize as usize;
    if co.checked_add(cs) != Some(e) {
        return Err("central bounds".into());
    }
    let cend = e;
    let mut pos = co;
    for _ in 0..total as usize {
        if pos.checked_add(46).map_or(true, |x| x > cend) {
            return Err("central trunc".into());
        }
        if bytes[pos] != 0x50
            || bytes[pos + 1] != 0x4B
            || bytes[pos + 2] != 0x01
            || bytes[pos + 3] != 0x02
        {
            return Err("bad central sig".into());
        }
        let cc = u32le(bytes, pos + 20)?;
        let cu = u32le(bytes, pos + 24)?;
        let nl = u16le(bytes, pos + 28)? as usize;
        let el = u16le(bytes, pos + 30)? as usize;
        let cl2 = u16le(bytes, pos + 32)? as usize;
        let ds = u16le(bytes, pos + 34)?;
        let lh = u32le(bytes, pos + 42)?;
        if ds != 0 {
            return Err("multidisk".into());
        }
        if cc == 0xFFFF_FFFF || cu == 0xFFFF_FFFF || lh == 0xFFFF_FFFF {
            return Err("zip64".into());
        }
        let rl = nl
            .checked_add(el)
            .and_then(|v| v.checked_add(cl2))
            .and_then(|v| v.checked_add(46))
            .ok_or("overflow".to_string())?;
        if pos.checked_add(rl).map_or(true, |x| x > cend) {
            return Err("field overflow".into());
        }
        let exs = pos
            .checked_add(46)
            .and_then(|v| v.checked_add(nl))
            .ok_or("overflow".to_string())?;
        let exe = exs.checked_add(el).ok_or("overflow".to_string())?;
        let mut q = exs;
        while q < exe {
            if q.checked_add(4).map_or(true, |x| x > exe) {
                return Err("extra trunc".into());
            }
            let tag = u16::from_le_bytes([bytes[q], bytes[q + 1]]);
            let sz = u16::from_le_bytes([bytes[q + 2], bytes[q + 3]]) as usize;
            if tag == 0x0001 {
                return Err("zip64 extra".into());
            }
            q = q
                .checked_add(4)
                .and_then(|v| v.checked_add(sz))
                .ok_or("overflow".to_string())?;
            if q > exe {
                return Err("extra overflow".into());
            }
        }
        let lu = lh as usize;
        if lu >= co {
            return Err("bad local off".into());
        }
        if lu.checked_add(4).map_or(true, |x| x > co) {
            return Err("bad local off".into());
        }
        if bytes[lu] != 0x50
            || bytes[lu + 1] != 0x4B
            || bytes[lu + 2] != 0x03
            || bytes[lu + 3] != 0x04
        {
            return Err("bad local sig".into());
        }
        pos = pos.checked_add(rl).ok_or("overflow".to_string())?;
    }
    if pos != cend {
        return Err("central trailing".into());
    }
    Ok(total as usize)
}
pub fn unpack(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, String> {
    crate::archive_headers::validate_members(bytes)?;
    if bytes.is_empty() {
        return Err("empty archive".into());
    }
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err("archive too large".into());
    }
    let declared_entries = entry_count(bytes)?;
    let mut za = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let n = za.len();
    if n != declared_entries {
        return Err("duplicate archive entry names".into());
    }
    if n > MAX_FILES + 128 {
        return Err("too many entries".into());
    }
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut flower: HashSet<String> = HashSet::new();
    let mut dlower: HashSet<String> = HashSet::new();
    let mut dirs: HashSet<String> = HashSet::new();
    let mut total: usize = 0;
    for i in 0..n {
        let mut f = za.by_index(i).map_err(|e| e.to_string())?;
        let raw = f.name().to_owned();
        let comp = f.compression();
        let enc = f.encrypted();
        let mode = f.unix_mode();
        let adv = f.size();
        if enc {
            return Err(format!("encrypted entry: {raw}"));
        }
        if !matches!(
            comp,
            zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
        ) {
            return Err(format!("unsupported compression: {raw}"));
        }
        if let Some(m) = mode {
            let k = m & 0o170000;
            if k == 0o120000 {
                return Err(format!("symlink denied: {raw}"));
            }
            if k != 0 && k != 0o100000 && k != 0o040000 {
                return Err(format!("nonregular mode: {raw}"));
            }
        }
        if raw.is_empty() || raw.contains('\\') || raw.contains('\0') || raw.starts_with('/') {
            return Err(format!("bad path: {raw}"));
        }
        let is_dir = raw.ends_with('/');
        if let Some(m) = mode {
            let k = m & 0o170000;
            if (k == 0o040000 && !is_dir) || (k == 0o100000 && is_dir) {
                return Err("entry mode/path mismatch".into());
            }
        }
        if is_dir {
            if adv != 0 {
                return Err("nonempty directory entry".into());
            }
            if raw.len() <= 1 {
                return Err(format!("bad dir: {raw}"));
            }
            let s = &raw[..raw.len() - 1];
            if s.is_empty() || s.contains('\\') || s.contains('\0') || s.starts_with('/') {
                return Err(format!("bad dir: {raw}"));
            }
            s.parse::<PackagePath>().map_err(|e| e.to_string())?;
            let dl = s.to_ascii_lowercase();
            if dirs.contains(s) || dlower.contains(&dl) {
                return Err(format!("duplicate dir: {s}"));
            }
            if files.contains_key(s) || flower.contains(&dl) {
                return Err(format!("file-dir collision: {s}"));
            }
            for a in parents(s) {
                if flower.contains(&a.to_ascii_lowercase()) {
                    return Err(format!("file-dir collision: {s}"));
                }
            }
            dlower.insert(dl);
            dirs.insert(s.to_owned());
        } else {
            if raw.parse::<PackagePath>().is_err() && !is_manifest_file_name(&raw) {
                return Err(format!("bad path: {raw}"));
            }
            if is_manifest_file_name(&raw) && files.keys().any(|name| is_manifest_file_name(name)) {
                return Err("multiple root manifests".into());
            }
            let pl = raw.to_ascii_lowercase();
            if files.contains_key(&raw) || flower.contains(&pl) {
                return Err(format!("duplicate file: {raw}"));
            }
            if dirs.contains(&raw) || dlower.contains(&pl) {
                return Err(format!("file-dir collision: {raw}"));
            }
            for a in parents(&raw) {
                if files.contains_key(a) || flower.contains(&a.to_ascii_lowercase()) {
                    return Err(format!("file-dir collision: {raw}"));
                }
            }
            for q in files.keys() {
                if prefix_child(&raw, q) || prefix_child(&pl, &q.to_ascii_lowercase()) {
                    return Err(format!("file-dir collision: {raw}"));
                }
            }
            for d in dirs.iter() {
                let dl2 = d.to_ascii_lowercase();
                if prefix_child(&raw, d) || prefix_child(&pl, &dl2) {
                    return Err(format!("file-dir collision: {raw}"));
                }
            }
            if files.len() >= MAX_FILES {
                return Err("too many files".into());
            }
            let rem = if is_manifest_file_name(&raw) {
                MANIFEST_MAX.min(MAX_PAYLOAD_BYTES - total)
            } else {
                MAX_PAYLOAD_BYTES - total
            };
            if adv > rem as u64 {
                return Err(format!("entry too large: {raw}"));
            }
            let mut buf = Vec::new();
            (&mut f)
                .take(rem as u64 + 1)
                .read_to_end(&mut buf)
                .map_err(|e| e.to_string())?;
            if buf.len() as u64 != adv {
                return Err(format!("size mismatch: {raw}"));
            }
            if is_manifest_file_name(&raw) && buf.len() > MANIFEST_MAX {
                return Err("manifest too large".into());
            }
            total += buf.len();
            flower.insert(pl);
            files.insert(raw, buf);
        }
    }
    if let Some(manifest) = files.remove(el_extension_api::package_path::LEGACY_MANIFEST_FILE_NAME)
    {
        files.insert(MANIFEST.to_owned(), manifest);
    }
    if !files.contains_key(MANIFEST) {
        return Err("missing manifest".into());
    }
    Ok(files)
}
