use el_core::resource_location::ResourceLocation;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
static ACTIVE_LANGUAGE: RwLock<Option<Arc<HashMap<String, String>>>> = RwLock::new(None);
fn install_language(map: HashMap<String, String>) {
    *ACTIVE_LANGUAGE
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::new(map));
}
pub fn active_language_code_from_options(options_content: &str) -> &str {
    let mut saved_code = None;
    for line in options_content.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.trim() == "lang" {
            saved_code = Some(value.trim());
        }
    }
    match saved_code {
        Some(code) if !code.is_empty() => code,
        _ => "en_us",
    }
}
pub fn reload_from_resource_manager(
    manager: &crate::resources::manager::ClientResourceManager,
    active_code: &str,
) {
    install_language(load_translations(manager, active_code));
}
fn load_translations(
    manager: &crate::resources::manager::ClientResourceManager,
    active_code: &str,
) -> HashMap<String, String> {
    let mut combined = HashMap::new();
    let selected_exists = manager
        .extract_languages()
        .iter()
        .any(|language| language.code == active_code);
    let languages = std::iter::once("en_us")
        .chain((active_code != "en_us" && selected_exists).then_some(active_code));
    let namespaces = manager
        .namespaces()
        .unwrap_or_else(|_| vec!["minecraft".to_owned()]);
    for code in languages {
        for namespace in &namespaces {
            if let Ok(stack) = manager.read_lang_stack(&ResourceLocation::new(namespace, code)) {
                for bytes in stack {
                    append_language_json(&bytes, &mut combined);
                }
            }
        }
    }
    if let Ok(Some(bytes)) = manager.read_default_language_deprecations() {
        apply_deprecated_translations(&bytes, &mut combined);
    }
    combined
}
fn append_language_json(bytes: &[u8], target: &mut HashMap<String, String>) {
    let Ok(serde_json::Value::Object(entries)) = serde_json::from_slice(bytes) else {
        return;
    };
    for (key, value) in entries {
        let text = match value {
            serde_json::Value::String(text) => text,
            serde_json::Value::Bool(value) => value.to_string(),
            serde_json::Value::Number(value) => value.to_string(),
            _ => continue,
        };
        target.insert(key, normalize_numeric_formats(&text));
    }
}
fn normalize_numeric_formats(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut result = String::with_capacity(text.len());
    let mut copied = 0;
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] != b'%' {
            cursor += 1;
            continue;
        }
        let start = cursor;
        cursor += 1;
        let digits_start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        let positional_end = if cursor > digits_start && bytes.get(cursor) == Some(&b'$') {
            cursor += 1;
            cursor
        } else {
            cursor = digits_start;
            digits_start
        };
        while cursor < bytes.len() && (bytes[cursor].is_ascii_digit() || bytes[cursor] == b'.') {
            cursor += 1;
        }
        if matches!(bytes.get(cursor), Some(b'd' | b'f')) {
            result.push_str(&text[copied..start]);
            result.push('%');
            result.push_str(&text[digits_start..positional_end]);
            result.push('s');
            cursor += 1;
            copied = cursor;
        }
    }
    result.push_str(&text[copied..]);
    result
}
fn apply_deprecated_translations(bytes: &[u8], target: &mut HashMap<String, String>) {
    #[derive(serde::Deserialize)]
    struct DeprecatedTranslations {
        removed: Vec<String>,
        renamed: std::collections::BTreeMap<String, String>,
    }
    let Ok(info) = serde_json::from_slice::<DeprecatedTranslations>(bytes) else {
        return;
    };
    for key in info.removed {
        target.remove(&key);
    }
    for (from, to) in info.renamed {
        if let Some(value) = target.remove(&from) {
            target.insert(to, value);
        } else {
            target.remove(&to);
        }
    }
}
fn eltharn_bootstrap_translation(key: &str) -> Option<&'static str> {
    static STRINGS: OnceLock<HashMap<String, String>> = OnceLock::new();
    STRINGS
        .get_or_init(|| {
            serde_json::from_str(include_str!("../../assets/lang/en_us.json"))
                .expect("embedded Eltharn language JSON must be valid")
        })
        .get(key)
        .map(String::as_str)
}
pub fn translate(key: &str) -> Cow<'_, str> {
    if let Ok(guard) = ACTIVE_LANGUAGE.read() {
        if let Some(map) = guard.as_ref() {
            if let Some(val) = map.get(key) {
                return Cow::Owned(val.clone());
            }
        }
    }
    if let Some(value) = eltharn_bootstrap_translation(key) {
        return Cow::Borrowed(value);
    }
    Cow::Borrowed(key)
}
pub fn keys_with_prefix(prefix: &str) -> Vec<String> {
    let Ok(guard) = ACTIVE_LANGUAGE.read() else {
        return Vec::new();
    };
    let Some(entries) = guard.as_ref() else {
        return Vec::new();
    };
    let mut keys = entries
        .keys()
        .filter(|key| key.starts_with(prefix))
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    keys
}
pub fn format(key: &str, args: &[&dyn std::fmt::Display]) -> String {
    let template = translate(key);
    let mut result = String::with_capacity(template.len() + args.len() * 8);
    let mut chars = template.char_indices().peekable();
    let mut auto_idx = 0;
    while let Some((_, ch)) = chars.next() {
        if ch != '%' {
            result.push(ch);
            continue;
        }
        match chars.peek() {
            Some(&(_, '%')) => {
                chars.next();
                result.push('%');
            }
            Some(&(_, 's')) => {
                chars.next();
                if auto_idx < args.len() {
                    use std::fmt::Write;
                    let _ = write!(result, "{}", args[auto_idx]);
                    auto_idx += 1;
                }
            }
            Some(&(_, digit)) if digit.is_ascii_digit() => {
                let mut num = 0usize;
                while let Some(&(_, d)) = chars.peek() {
                    if d.is_ascii_digit() {
                        num = num * 10 + (d as usize - '0' as usize);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if let Some(&(_, '$')) = chars.peek() {
                    chars.next();
                    if let Some(&(_, 's')) = chars.peek() {
                        chars.next();
                        let arg_idx = num.saturating_sub(1);
                        if arg_idx < args.len() {
                            use std::fmt::Write;
                            let _ = write!(result, "{}", args[arg_idx]);
                        }
                    }
                }
            }
            _ => {
                result.push('%');
            }
        }
    }
    result
}
