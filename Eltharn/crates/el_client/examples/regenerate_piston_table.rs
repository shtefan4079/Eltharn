use std::{collections::BTreeMap, error::Error, fmt::Write, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    if arguments.len() != 2 {
        return Err("Usage: regenerate_piston_table INPUT.csv OUTPUT.rs".into());
    }
    let input = fs::read_to_string(&arguments[0])?;
    let mut entries = BTreeMap::new();
    for line in input.lines() {
        if line.is_empty() || line.starts_with('#') || line == "block,push_reaction" {
            continue;
        }
        let (path, reaction) = line.split_once(',').ok_or("Invalid reaction row")?;
        let path = path.strip_prefix("minecraft:").unwrap_or(path);
        if path.is_empty()
            || !path.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'/')
            })
        {
            return Err(format!("Invalid block path: {path}").into());
        }
        let reaction = match reaction {
            "NORMAL" => continue,
            "DESTROY" => "Destroy",
            "BLOCK" => "Block",
            "PUSH_ONLY" => "PushOnly",
            _ => return Err(format!("Invalid reaction: {reaction}").into()),
        };
        if entries.insert(path, reaction).is_some() {
            return Err(format!("Duplicate block: {path}").into());
        }
    }
    if entries.is_empty() {
        return Err("Empty reaction table".into());
    }
    let mut output = String::from(
        "use super::PistonPushReaction;\n\npub(super) const ENTRIES: &[(&str, PistonPushReaction)] = &[\n",
    );
    for (path, reaction) in entries {
        writeln!(output, "    (\"{path}\", PistonPushReaction::{reaction}),")?;
    }
    output.push_str("];\n");
    fs::write(&arguments[1], output)?;
    Ok(())
}
