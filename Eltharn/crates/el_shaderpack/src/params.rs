use crate::strict::{array, field, keys, string, uint};
use serde_json::Value;
fn is_local_id(s: &str) -> bool {
    if s.is_empty() || s.len() > 64 {
        return false;
    }
    for b in s.bytes() {
        if !(b == b'_' || b.is_ascii_lowercase() || b.is_ascii_digit()) {
            return false;
        }
    }
    true
}
fn strict_i32(v: &Value) -> Result<i32, String> {
    if let Some(i) = v.as_i64() {
        if i < -2147483648 || i > 2147483647 {
            return Err("int out of i32 range".into());
        }
        return Ok(i as i32);
    }
    if let Some(u) = v.as_u64() {
        if u > 2147483647 {
            return Err("int out of i32 range".into());
        }
        return Ok(u as i32);
    }
    Err("expected strict integer".into())
}
fn num_to_f32(v: &Value) -> Result<f32, String> {
    let f = v.as_f64().ok_or("expected number".to_string())?;
    if !f.is_finite() {
        return Err("float not finite".into());
    }
    let c = f as f32;
    if !c.is_finite() {
        return Err("float f32 overflow".into());
    }
    Ok(c)
}
enum P {
    Bool {
        id: String,
        index: u32,
        def: bool,
    },
    Int {
        id: String,
        index: u32,
        min: i32,
        max: i32,
        def: i32,
    },
    Float {
        id: String,
        index: u32,
        min: f32,
        max: f32,
        def: f32,
    },
    Enum {
        id: String,
        index: u32,
        values: Vec<String>,
        def_idx: u32,
    },
}
impl P {
    fn id(&self) -> &str {
        match self {
            P::Bool { id, .. } | P::Int { id, .. } | P::Float { id, .. } | P::Enum { id, .. } => id,
        }
    }
    fn index(&self) -> u32 {
        match self {
            P::Bool { index, .. }
            | P::Int { index, .. }
            | P::Float { index, .. }
            | P::Enum { index, .. } => *index,
        }
    }
}
fn validate(params: &Value) -> Result<Vec<P>, String> {
    let arr = array(params)?;
    if arr.len() > 64 {
        return Err("params len > 64".into());
    }
    let mut ids = std::collections::HashSet::new();
    let mut idxs = std::collections::HashSet::new();
    let mut out = Vec::new();
    for p in arr.iter() {
        let t = string(field(p, "type")?)?.to_string();
        let id = string(field(p, "id")?)?.to_string();
        if !is_local_id(&id) {
            return Err("bad id".into());
        }
        if !ids.insert(id.clone()) {
            return Err("dup id".into());
        }
        let idx = uint(field(p, "index")?)?;
        if idx > 63 {
            return Err("index out of range".into());
        }
        if !idxs.insert(idx) {
            return Err("dup index".into());
        }
        let index = idx as u32;
        match t.as_str() {
            "bool" => {
                keys(p, &["id", "index", "type", "default"], &[])?;
                let d = field(p, "default")?;
                if !d.is_boolean() {
                    return Err("bool default type".into());
                }
                out.push(P::Bool {
                    id,
                    index,
                    def: d.as_bool().unwrap(),
                });
            }
            "int" => {
                keys(p, &["id", "index", "type", "min", "max", "default"], &[])?;
                let mn = strict_i32(field(p, "min")?)?;
                let mx = strict_i32(field(p, "max")?)?;
                let df = strict_i32(field(p, "default")?)?;
                if mn > mx || df < mn || df > mx {
                    return Err("int default range".into());
                }
                out.push(P::Int {
                    id,
                    index,
                    min: mn,
                    max: mx,
                    def: df,
                });
            }
            "float" => {
                keys(p, &["id", "index", "type", "min", "max", "default"], &[])?;
                let mn = num_to_f32(field(p, "min")?)?;
                let mx = num_to_f32(field(p, "max")?)?;
                let df = num_to_f32(field(p, "default")?)?;
                if mn > mx || df < mn || df > mx {
                    return Err("float default range".into());
                }
                out.push(P::Float {
                    id,
                    index,
                    min: mn,
                    max: mx,
                    def: df,
                });
            }
            "enum" => {
                keys(p, &["id", "index", "type", "values", "default"], &[])?;
                let vs = array(field(p, "values")?)?;
                if vs.is_empty() || vs.len() > 64 {
                    return Err("enum values len".into());
                }
                let mut vals = Vec::new();
                let mut seen = std::collections::HashSet::new();
                for e in vs.iter() {
                    let s = string(e)?.to_string();
                    if !is_local_id(&s) {
                        return Err("bad enum id".into());
                    }
                    if !seen.insert(s.clone()) {
                        return Err("dup enum value".into());
                    }
                    vals.push(s);
                }
                let d = string(field(p, "default")?)?.to_string();
                let di = vals
                    .iter()
                    .position(|x| x == &d)
                    .ok_or("bad enum default".to_string())? as u32;
                out.push(P::Enum {
                    id,
                    index,
                    values: vals,
                    def_idx: di,
                });
            }
            _ => return Err("bad type".into()),
        }
    }
    Ok(out)
}
pub(crate) fn ids(params: &Value) -> Result<Vec<String>, String> {
    Ok(validate(params)?
        .into_iter()
        .map(|p| p.id().to_string())
        .collect())
}
pub(crate) fn parse(params: &Value, settings: &Value) -> Result<Option<Vec<u8>>, String> {
    let defs = validate(params)?;
    if defs.is_empty() {
        return Ok(None);
    }
    let obj = settings
        .as_object()
        .ok_or("settings must be object".to_string())?;
    let mut buf = vec![0u8; 1024];
    for d in defs.iter() {
        let v: u32 = match d {
            P::Bool { id, def, .. } => match obj.get(id) {
                Some(s) => {
                    if !s.is_boolean() {
                        return Err("bool setting type".into());
                    }
                    if s.as_bool().unwrap() { 1 } else { 0 }
                }
                None => {
                    if *def {
                        1
                    } else {
                        0
                    }
                }
            },
            P::Int {
                id, min, max, def, ..
            } => match obj.get(id) {
                Some(s) => {
                    let x = strict_i32(s)?;
                    if x < *min || x > *max {
                        return Err("int setting range".into());
                    }
                    x as u32
                }
                None => *def as u32,
            },
            P::Float {
                id, min, max, def, ..
            } => match obj.get(id) {
                Some(s) => {
                    let c = num_to_f32(s)?;
                    if c < *min || c > *max {
                        return Err("float setting range".into());
                    }
                    c.to_bits()
                }
                None => def.to_bits(),
            },
            P::Enum {
                id,
                values,
                def_idx,
                ..
            } => match obj.get(id) {
                Some(s) => {
                    let st = string(s)?.to_string();
                    vals_pos(values, &st)?
                }
                None => *def_idx,
            },
        };
        let off = d.index() as usize * 16;
        buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
    Ok(Some(buf))
}
fn vals_pos(values: &[String], s: &str) -> Result<u32, String> {
    values
        .iter()
        .position(|x| x == s)
        .map(|i| i as u32)
        .ok_or("unknown enum".into())
}
