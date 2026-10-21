use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub fn read_block_tag_entries(path: &Path) -> Vec<String> {
    let source = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "failed to read Java 26.2 block tag at {}: {error}",
            path.display()
        )
    });
    let value: serde_json::Value = serde_json::from_str(&source).unwrap_or_else(|error| {
        panic!(
            "failed to parse Java 26.2 block tag JSON at {}: {error}",
            path.display()
        )
    });
    let object = value.as_object().unwrap_or_else(|| {
        panic!(
            "Java 26.2 block tag at {} is not a JSON object",
            path.display()
        )
    });
    if let Some(replace) = object.get("replace") {
        assert_eq!(
            replace,
            &serde_json::Value::Bool(false),
            "Java 26.2 block tag at {} sets `replace: true`, which build-time merging does not implement",
            path.display()
        );
    }
    let values = object
        .get("values")
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| {
            panic!(
                "Java 26.2 block tag at {} has no array `values` field",
                path.display()
            )
        });

    values
        .iter()
        .map(|entry| {
            let entry = entry.as_str().unwrap_or_else(|| {
                panic!(
                    "Java 26.2 block tag at {} uses an unsupported non-string entry: {entry}",
                    path.display()
                )
            });
            normalize_resource_location(entry)
        })
        .collect()
}

pub fn normalize_resource_location(value: &str) -> String {
    let (prefix, id) = if let Some(tag) = value.strip_prefix('#') {
        ("#", tag)
    } else {
        ("", value)
    };
    if id.contains(':') {
        format!("{prefix}{id}")
    } else {
        format!("{prefix}minecraft:{id}")
    }
}

pub fn collect_json_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(directory)
        .unwrap_or_else(|error| {
            panic!(
                "failed to enumerate Java 26.2 block tags at {}: {error}",
                directory.display()
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|error| {
            panic!(
                "failed to enumerate Java 26.2 block tags at {}: {error}",
                directory.display()
            )
        });
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            files.push(path);
        }
    }
}

pub fn load_java_26_2_block_tags(root: &Path) -> BTreeMap<String, Vec<String>> {
    let mut files = Vec::new();
    collect_json_files(root, &mut files);

    let mut tags = BTreeMap::new();
    for path in files {
        let relative = path
            .strip_prefix(root)
            .unwrap_or_else(|_| {
                panic!(
                    "tag path {} escaped root {}",
                    path.display(),
                    root.display()
                )
            })
            .with_extension("");
        let relative = relative.to_string_lossy().replace('\\', "/");
        let tag_id = format!("minecraft:{relative}");
        let previous = tags.insert(tag_id.clone(), read_block_tag_entries(&path));
        assert!(
            previous.is_none(),
            "duplicate Java 26.2 block tag id `{tag_id}` while reading {}",
            root.display()
        );
    }
    tags
}

fn resolve_java_26_2_block_tag(
    tag_id: &str,
    raw_tags: &BTreeMap<String, Vec<String>>,
    resolved: &mut BTreeMap<String, BTreeSet<String>>,
    visiting: &mut BTreeSet<String>,
) -> BTreeSet<String> {
    if let Some(blocks) = resolved.get(tag_id) {
        return blocks.clone();
    }
    assert!(
        visiting.insert(tag_id.to_owned()),
        "recursive Java 26.2 block-tag cycle while resolving `{tag_id}`"
    );

    let entries = raw_tags.get(tag_id).unwrap_or_else(|| {
        panic!(
            "Java 26.2 block-tag `{tag_id}` references a tag absent from the checked-in source data"
        )
    });
    let mut blocks = BTreeSet::new();
    for entry in entries {
        if let Some(nested_tag) = entry.strip_prefix('#') {
            blocks.extend(resolve_java_26_2_block_tag(
                nested_tag, raw_tags, resolved, visiting,
            ));
        } else {
            blocks.insert(entry.clone());
        }
    }
    visiting.remove(tag_id);
    resolved.insert(tag_id.to_owned(), blocks.clone());
    blocks
}

pub fn resolve_all_in_input_order(
    pairs: &[(String, Vec<String>)],
) -> BTreeMap<String, BTreeSet<String>> {
    let lookup: BTreeMap<String, Vec<String>> = pairs.iter().cloned().collect();
    assert_eq!(
        lookup.len(),
        pairs.len(),
        "duplicate Java 26.2 block tag id in input-order pair list",
    );
    let mut resolved = BTreeMap::new();
    for (tag_id, _) in pairs {
        resolve_java_26_2_block_tag(tag_id, &lookup, &mut resolved, &mut BTreeSet::new());
    }
    resolved
}

pub fn emit_tag_table_sorted(resolved: &BTreeMap<String, BTreeSet<String>>) -> String {
    let mut output =
        String::from("static GENERATED_JAVA_26_2_BLOCK_TAGS: &[(&str, &[&str])] = &[\n");
    for (tag_id, blocks) in resolved {
        output.push_str(&format!("    ({tag_id:?}, &[\n"));
        for block_id in blocks {
            output.push_str(&format!("        {block_id:?},\n"));
        }
        output.push_str("    ]),\n");
    }
    output.push_str(
        "];\n\
         \n\
         pub(crate) fn contains_java_26_2_block_tag(tag_id: &str, block_id: &str) -> bool {\n\
             let Ok(index) = GENERATED_JAVA_26_2_BLOCK_TAGS\n\
                 .binary_search_by(|(candidate, _)| (*candidate).cmp(tag_id))\n\
             else {\n\
                 return false;\n\
             };\n\
             GENERATED_JAVA_26_2_BLOCK_TAGS[index].1.binary_search(&block_id).is_ok()\n\
         }\n\
         \n\
",
    );
    output
}

pub fn generate_java_26_2_block_tag_table(root: &Path) -> String {
    let raw_tags = load_java_26_2_block_tags(root);
    let ordered: Vec<(String, Vec<String>)> = raw_tags.into_iter().collect();
    let resolved = resolve_all_in_input_order(&ordered);
    emit_tag_table_sorted(&resolved)
}
