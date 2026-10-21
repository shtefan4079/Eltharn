pub fn validate_members(bytes: &[u8]) -> Result<(), String> {
    let n = bytes.len();
    if !(22..=32 * 1024 * 1024).contains(&n) {
        return Err("archive size".into());
    }
    let eo = (n.saturating_sub(65557)..=n - 22)
        .rev()
        .find(|&p| r32(bytes, p) == 0x0605_4b50 && p + 22 + r16(bytes, p + 20) as usize == n)
        .ok_or("missing exact EOCD")?;
    let disk = r16(bytes, eo + 4);
    let cdd = r16(bytes, eo + 6);
    let ndt = r16(bytes, eo + 8);
    let ntt = r16(bytes, eo + 10);
    let csz = r32(bytes, eo + 12);
    let coff = r32(bytes, eo + 16);
    if disk != 0 || cdd != 0 || ndt != ntt {
        return Err("multidisk".into());
    }
    if ntt == 0 || ntt as usize > 4097 {
        return Err("bad count".into());
    }
    if ndt == 0xffff || csz == 0xffff_ffff || coff == 0xffff_ffff {
        return Err("zip64".into());
    }
    let (cn, co) = (ntt as usize, coff as usize);
    let cs = csz as usize;
    let ce = co.checked_add(cs).ok_or("cd overflow")?;
    if co > eo || ce != eo {
        return Err("cd bounds".into());
    }
    let mut off = co;
    let mut names: Vec<&[u8]> = Vec::new();
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for _ in 0..cn {
        let e = off.checked_add(46).ok_or("cd trunc")?;
        if e > ce {
            return Err("cd trunc".into());
        }
        if r32(bytes, off) != 0x0201_4b50 {
            return Err("cd sig".into());
        }
        let fl = r16(bytes, off + 8);
        let me = r16(bytes, off + 10);
        let crc = r32(bytes, off + 16);
        let cps = r32(bytes, off + 20);
        let ups = r32(bytes, off + 24);
        let nl = r16(bytes, off + 28) as usize;
        let xl = r16(bytes, off + 30) as usize;
        let cl = r16(bytes, off + 32) as usize;
        let ds = r16(bytes, off + 34);
        let at = r32(bytes, off + 38);
        let lo = r32(bytes, off + 42);
        let vn = r16(bytes, off + 6);
        if vn >= 45 || ds != 0 {
            return Err("disk/vers".into());
        }
        if cps == 0xffff_ffff || ups == 0xffff_ffff || lo == 0xffff_ffff {
            return Err("zip64".into());
        }
        if me != 0 && me != 8 {
            return Err("method".into());
        }
        if fl & !0x080e != 0 {
            return Err("flags".into());
        }
        let hs = off.checked_add(46).ok_or("cd var")?;
        let he = hs
            .checked_add(nl)
            .ok_or("cd name")?
            .checked_add(xl)
            .ok_or("cd extra")?
            .checked_add(cl)
            .ok_or("cd cmt")?;
        if he > ce {
            return Err("cd var bounds".into());
        }
        let nm = &bytes[hs..hs + nl];
        let xe = &bytes[hs + nl..hs + nl + xl];
        ck_name(nm)?;
        ck_extra(xe)?;
        ck_mode(at, nm.ends_with(b"/"))?;
        if names.contains(&nm) {
            return Err("dup name".into());
        }
        names.push(nm);
        let l = lo as usize;
        let lh = l.checked_add(30).ok_or("lh trunc")?;
        if lh > co {
            return Err("local after cd".into());
        }
        if r32(bytes, l) != 0x0403_4b50 {
            return Err("local sig".into());
        }
        let lf = r16(bytes, l + 6);
        let lm = r16(bytes, l + 8);
        let lc = r32(bytes, l + 14);
        let lp = r32(bytes, l + 18);
        let lu = r32(bytes, l + 22);
        let lnl = r16(bytes, l + 26) as usize;
        let lxl = r16(bytes, l + 28) as usize;
        if r16(bytes, l + 4) != vn || lf != fl || lm != me {
            return Err("local/central mismatch".into());
        }
        let ds0 = lh
            .checked_add(lnl)
            .ok_or("lname")?
            .checked_add(lxl)
            .ok_or("lextra")?;
        if ds0 > co {
            return Err("local var after cd".into());
        }
        if lnl != nl || &bytes[lh..lh + lnl] != nm {
            return Err("name mismatch".into());
        }
        ck_extra(&bytes[lh + lnl..ds0])?;
        let d3 = fl & 8 != 0;
        if d3 && (lc != 0 && lc != crc || lp != 0 && lp != cps || lu != 0 && lu != ups) {
            return Err("local dd nonzero".into());
        }
        if !d3 && (lc != crc || lp != cps || lu != ups) {
            return Err("crc/size".into());
        }
        let de = ds0.checked_add(cps as usize).ok_or("data overflow")?;
        if de > co {
            return Err("data after cd".into());
        }
        let end = if d3 {
            let e16 = de.checked_add(16).ok_or("dd overflow")?;
            let e12 = de.checked_add(12).ok_or("dd overflow")?;
            if e16 <= co
                && r32(bytes, de) == 0x0807_4b50
                && r32(bytes, de + 4) == crc
                && r32(bytes, de + 8) == cps
                && r32(bytes, de + 12) == ups
            {
                e16
            } else if e12 <= co
                && r32(bytes, de) == crc
                && r32(bytes, de + 4) == cps
                && r32(bytes, de + 8) == ups
            {
                e12
            } else {
                return Err("descriptor".into());
            }
        } else {
            de
        };
        if end > co {
            return Err("desc after cd".into());
        }
        ranges.push((l, end));
        off = he;
    }
    if off != ce {
        return Err("cd size".into());
    }
    ranges.sort_unstable();
    if ranges[0].0 != 0 {
        return Err("prefix".into());
    }
    for w in ranges.windows(2) {
        if w[1].0 < w[0].1 {
            return Err("overlap".into());
        }
        if w[1].0 > w[0].1 {
            return Err("gap".into());
        }
    }
    if ranges.last().unwrap().1 != co {
        return Err("gap before central directory".into());
    }
    Ok(())
}
fn r16(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn r32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn ck_name(n: &[u8]) -> Result<(), String> {
    let name = std::str::from_utf8(n).map_err(|_| "invalid filename")?;
    if el_extension_api::package_path::is_manifest_file_name(name) {
        return Ok(());
    }
    let path = name.strip_suffix('/').unwrap_or(name);
    path.parse::<el_extension_api::PackagePath>()
        .map_err(|e| e.to_string())?;
    Ok(())
}
fn ck_extra(x: &[u8]) -> Result<(), String> {
    let mut o = 0;
    while o < x.len() {
        let e = o.checked_add(4).ok_or("extra trunc")?;
        if e > x.len() {
            return Err("extra trunc".into());
        }
        let id = u16::from_le_bytes([x[o], x[o + 1]]);
        let sz = u16::from_le_bytes([x[o + 2], x[o + 3]]) as usize;
        if id == 1 {
            return Err("zip64 extra".into());
        }
        o = e.checked_add(sz).ok_or("extra overflow")?;
        if o > x.len() {
            return Err("extra trunc".into());
        }
    }
    Ok(())
}
fn ck_mode(a: u32, is_dir: bool) -> Result<(), String> {
    let kind = (a >> 16) & 0o170000;
    if !matches!(kind, 0 | 0o040000 | 0o100000) {
        return Err("filetype".into());
    }
    if (a & 0x10 != 0 || kind == 0o040000) && !is_dir {
        return Err("dir without trailing slash".into());
    }
    if is_dir && kind == 0o100000 {
        return Err("entry mode/path mismatch".into());
    }
    Ok(())
}
