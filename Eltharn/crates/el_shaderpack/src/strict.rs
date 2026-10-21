use serde_json::{Map, Value};
use std::fmt;
type Result<T> = std::result::Result<T, String>;

pub fn strict_json(bytes: &[u8]) -> Result<Value> {
    el_strict_json::strict_json(bytes, 262144)
}

pub(crate) fn ensure(ok: bool, msg: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(msg.into()) }
}
pub(crate) fn object(v: &Value) -> Result<&Map<String, Value>> {
    v.as_object().ok_or("expected object".into())
}
pub(crate) fn array(v: &Value) -> Result<&[Value]> {
    v.as_array()
        .map(Vec::as_slice)
        .ok_or("expected array".into())
}
pub(crate) fn string(v: &Value) -> Result<&str> {
    v.as_str().ok_or("expected string".into())
}
pub(crate) fn uint(v: &Value) -> Result<u64> {
    v.as_u64().ok_or("expected unsigned integer token".into())
}
pub(crate) fn field<'a>(v: &'a Value, name: &str) -> Result<&'a Value> {
    v.get(name).ok_or_else(|| format!("missing {name}"))
}
pub(crate) fn keys(v: &Value, required: &[&str], optional: &[&str]) -> Result<()> {
    let o = object(v)?;
    ensure(
        required.iter().all(|k| o.contains_key(*k)),
        "missing required field",
    )?;
    ensure(
        o.keys()
            .all(|k| required.contains(&k.as_str()) || optional.contains(&k.as_str())),
        "unsupported fixture field",
    )
}
pub(crate) fn typed<T: std::str::FromStr>(v: &Value) -> Result<T>
where
    T::Err: fmt::Display,
{
    string(v)?.parse().map_err(|e: T::Err| e.to_string())
}
pub(crate) fn text_limit(v: &Value, min: usize, max: usize) -> Result<()> {
    let s = string(v)?;
    ensure(
        (min..=max).contains(&s.chars().count()) && !s.chars().any(char::is_control),
        "text bounds/control",
    )
}
pub(crate) fn eq(v: &Value, expected: &str) -> Result<()> {
    ensure(string(v)? == expected, "unexpected profile value")
}
pub(crate) fn id(v: &Value) -> Result<&str> {
    let s = string(v)?;
    ensure(
        (1..=64).contains(&s.len())
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
        "local id syntax",
    )?;
    Ok(s)
}
pub(crate) fn resource_id(v: &Value) -> Result<&str> {
    let s = string(v)?;
    let suffix = s.strip_prefix("pack.").ok_or("private resource prefix")?;
    id(&Value::String(suffix.into()))?;
    Ok(s)
}
