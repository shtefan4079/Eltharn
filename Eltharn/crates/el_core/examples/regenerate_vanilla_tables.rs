use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

mod p5_tag_gen;

#[derive(Debug)]
struct JavaBlockMiningEntry {
    block_id: String,
    destroy_speed: f32,
    requires_correct_tool: bool,
    is_model_render_shape: bool,
    offset_type: JavaBlockOffsetType,
    max_horizontal_offset: f32,
    max_vertical_offset: f32,
    offset_probe: [f32; 3],
    model_seed_type: JavaBlockModelSeedType,
}

#[derive(Debug)]
struct JavaBlockLightingEntry {
    state_id: usize,
    block_id: String,
    light_emission: u8,
    emissive_rendering: bool,
    solid_render: bool,
    light_dampening: u8,
    view_blocking: bool,
    shade_brightness: f32,
}

#[derive(Clone, Copy, Debug)]
enum JavaBlockOffsetType {
    None,
    Xz,
    Xyz,
}

#[derive(Clone, Copy, Debug)]
enum JavaBlockModelSeedType {
    Standard,
    DoubleBlockLower,
    BedSource,
}

#[derive(Debug)]
enum JavaToolSelector {
    Tag(String),
    DirectBlocks(Vec<String>),
}

#[derive(Debug)]
struct JavaToolRule {
    selector: JavaToolSelector,
    speed: Option<f32>,
    correct_for_drops: Option<bool>,
}

#[derive(Debug)]
struct JavaToolDefinition {
    default_mining_speed: f32,
    rules: BTreeMap<usize, JavaToolRule>,
    saw_no_rules_row: bool,
}

fn read_oracle_csv(path: &Path, expected_header: &str) -> Vec<Vec<String>> {
    let source = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "failed to read Java 26.2 mining oracle at {}: {error}",
            path.display()
        )
    });
    let mut lines = source.lines();
    let header = lines
        .next()
        .unwrap_or_else(|| panic!("Java 26.2 mining oracle at {} is empty", path.display()));
    assert_eq!(
        header,
        expected_header,
        "unexpected Java 26.2 mining oracle schema at {}",
        path.display()
    );

    lines
        .enumerate()
        .filter_map(|(line_index, line)| {
            if line.is_empty() {
                return None;
            }
            let fields = line.split(',').map(str::to_owned).collect::<Vec<_>>();
            Some((line_index + 2, fields))
        })
        .map(|(line_index, fields)| {
            assert!(
                !fields.iter().any(|field| field.contains('\r')),
                "unexpected carriage return in Java 26.2 mining oracle at {} line {line_index}",
                path.display()
            );
            fields
        })
        .collect()
}

fn parse_oracle_f32(value: &str, path: &Path, field: &str, line_index: usize) -> f32 {
    let parsed = value.parse::<f32>().unwrap_or_else(|error| {
        panic!(
            "invalid Java 26.2 mining oracle {field} `{value}` at {} line {line_index}: {error}",
            path.display()
        )
    });
    assert!(
        parsed.is_finite(),
        "non-finite Java 26.2 mining oracle {field} `{value}` at {} line {line_index}",
        path.display()
    );
    parsed
}

fn parse_oracle_bool(value: &str, path: &Path, field: &str, line_index: usize) -> bool {
    match value {
        "true" => true,
        "false" => false,
        _ => panic!(
            "invalid Java 26.2 mining oracle {field} `{value}` at {} line {line_index}",
            path.display()
        ),
    }
}

fn assert_minecraft_location(value: &str, path: &Path, field: &str, line_index: usize) {
    assert!(
        value.starts_with("minecraft:") && !value.contains(',') && !value.is_empty(),
        "unsupported Java 26.2 vanilla {field} `{value}` at {} line {line_index}",
        path.display()
    );
}

fn load_java_26_2_block_mining(path: &Path) -> Vec<JavaBlockMiningEntry> {
    const HEADER: &str = "block,default_state_id,destroy_speed,requires_correct_tool,render_shape,offset_type,max_horizontal_offset,max_vertical_offset,offset_probe_x,offset_probe_y,offset_probe_z,model_seed_type";
    let mut entries = BTreeMap::new();
    for (index, fields) in read_oracle_csv(path, HEADER).into_iter().enumerate() {
        let line_index = index + 2;
        assert_eq!(
            fields.len(),
            12,
            "unexpected Java 26.2 block-mining field count at {} line {line_index}",
            path.display()
        );
        let block_id = &fields[0];
        assert_minecraft_location(block_id, path, "block id", line_index);
        fields[1].parse::<usize>().unwrap_or_else(|error| {
            panic!(
                "invalid Java 26.2 block-mining default_state_id `{}` at {} line {line_index}: {error}",
                fields[1],
                path.display()
            )
        });
        let offset_type = match fields[5].as_str() {
            "NONE" => JavaBlockOffsetType::None,
            "XZ" => JavaBlockOffsetType::Xz,
            "XYZ" => JavaBlockOffsetType::Xyz,
            offset_type => panic!(
                "unsupported Java 26.2 block offset_type `{offset_type}` at {} line {line_index}",
                path.display()
            ),
        };
        let max_horizontal_offset =
            parse_oracle_f32(&fields[6], path, "max_horizontal_offset", line_index);
        let max_vertical_offset =
            parse_oracle_f32(&fields[7], path, "max_vertical_offset", line_index);
        let offset_probe = [
            parse_oracle_f32(&fields[8], path, "offset_probe_x", line_index),
            parse_oracle_f32(&fields[9], path, "offset_probe_y", line_index),
            parse_oracle_f32(&fields[10], path, "offset_probe_z", line_index),
        ];
        assert!(
            max_horizontal_offset >= 0.0 && max_vertical_offset >= 0.0,
            "negative Java 26.2 block offset bound at {} line {line_index}",
            path.display()
        );
        if matches!(offset_type, JavaBlockOffsetType::None) {
            assert!(
                max_horizontal_offset.to_bits() == 0
                    && max_vertical_offset.to_bits() == 0
                    && offset_probe.iter().all(|value| value.to_bits() == 0),
                "Java 26.2 NONE block offset row carries non-zero data at {} line {line_index}",
                path.display()
            );
        }
        let model_seed_type = match fields[11].as_str() {
            "STANDARD" => JavaBlockModelSeedType::Standard,
            "DOUBLE_BLOCK_LOWER" => JavaBlockModelSeedType::DoubleBlockLower,
            "BED_SOURCE" => JavaBlockModelSeedType::BedSource,
            model_seed_type => panic!(
                "unsupported Java 26.2 block model_seed_type `{model_seed_type}` at {} line {line_index}",
                path.display()
            ),
        };

        let entry = JavaBlockMiningEntry {
            block_id: block_id.clone(),
            destroy_speed: parse_oracle_f32(&fields[2], path, "destroy_speed", line_index),
            requires_correct_tool: parse_oracle_bool(
                &fields[3],
                path,
                "requires_correct_tool",
                line_index,
            ),
            is_model_render_shape: match fields[4].as_str() {
                "MODEL" => true,
                "INVISIBLE" => false,
                render_shape => panic!(
                    "unsupported Java 26.2 block render_shape `{render_shape}` at {} line {line_index}",
                    path.display()
                ),
            },
            offset_type,
            max_horizontal_offset,
            max_vertical_offset,
            offset_probe,
            model_seed_type,
        };
        assert!(
            entries.insert(block_id.clone(), entry).is_none(),
            "duplicate Java 26.2 block-mining row for `{block_id}` at {} line {line_index}",
            path.display()
        );
    }
    assert!(
        !entries.is_empty(),
        "Java 26.2 block-mining oracle at {} had no data rows",
        path.display()
    );
    entries.into_values().collect()
}

fn parse_oracle_u8(value: &str, path: &Path, field: &str, line_index: usize) -> u8 {
    value.parse::<u8>().unwrap_or_else(|error| {
        panic!(
            "invalid Java 26.2 oracle {field} `{value}` at {} line {line_index}: {error}",
            path.display()
        )
    })
}

fn load_java_26_2_block_lighting(path: &Path) -> Vec<JavaBlockLightingEntry> {
    const HEADER: &str = "id,block,light_emission,emissive_rendering,solid_render,light_dampening,view_blocking,shade_brightness";
    let mut entries = BTreeMap::<usize, JavaBlockLightingEntry>::new();
    for (index, fields) in read_oracle_csv(path, HEADER).into_iter().enumerate() {
        let line_index = index + 2;
        assert_eq!(
            fields.len(),
            8,
            "unexpected Java 26.2 block-lighting field count at {} line {line_index}",
            path.display()
        );
        let state_id = fields[0].parse::<usize>().unwrap_or_else(|error| {
            panic!(
                "invalid Java 26.2 block-lighting state id `{}` at {} line {line_index}: {error}",
                fields[0],
                path.display()
            )
        });
        assert_minecraft_location(&fields[1], path, "block-lighting block id", line_index);
        let light_emission = parse_oracle_u8(&fields[2], path, "light_emission", line_index);
        let light_dampening = parse_oracle_u8(&fields[5], path, "light_dampening", line_index);
        assert!(
            light_emission <= 15 && light_dampening <= 15,
            "out-of-range Java 26.2 block-lighting nibble at {} line {line_index}",
            path.display()
        );
        let shade_brightness = parse_oracle_f32(&fields[7], path, "shade_brightness", line_index);
        assert!(
            (0.0..=1.0).contains(&shade_brightness),
            "out-of-range Java 26.2 block-lighting shade brightness at {} line {line_index}",
            path.display()
        );
        let entry = JavaBlockLightingEntry {
            state_id,
            block_id: fields[1].clone(),
            light_emission,
            emissive_rendering: parse_oracle_bool(
                &fields[3],
                path,
                "emissive_rendering",
                line_index,
            ),
            solid_render: parse_oracle_bool(&fields[4], path, "solid_render", line_index),
            light_dampening,
            view_blocking: parse_oracle_bool(&fields[6], path, "view_blocking", line_index),
            shade_brightness,
        };
        assert!(
            entries.insert(state_id, entry).is_none(),
            "duplicate Java 26.2 block-lighting state id {state_id} at {} line {line_index}",
            path.display()
        );
    }
    assert!(
        !entries.is_empty(),
        "Java 26.2 block-lighting oracle at {} had no data rows",
        path.display()
    );
    for (expected, actual) in entries.keys().enumerate() {
        assert_eq!(
            expected,
            *actual,
            "non-contiguous Java 26.2 block-lighting state id at {}",
            path.display()
        );
    }
    entries.into_values().collect()
}

struct JavaBlockFluidSupportEntry {
    state_id: usize,
    block_id: String,
    solid: bool,
    blocks_motion: bool,
    face_sturdy: [bool; 6],
    fluid_side_overlay: bool,
    ice_block: bool,
}

fn load_java_26_2_block_fluid_support(path: &Path) -> Vec<JavaBlockFluidSupportEntry> {
    const HEADER: &str = "id,block,solid,blocks_motion,face_sturdy_down,face_sturdy_up,\
face_sturdy_north,face_sturdy_south,face_sturdy_west,face_sturdy_east,fluid_side_overlay,ice_block";
    let mut entries = BTreeMap::<usize, JavaBlockFluidSupportEntry>::new();
    for (index, fields) in read_oracle_csv(path, HEADER).into_iter().enumerate() {
        let line_index = index + 2;
        assert_eq!(
            fields.len(),
            12,
            "unexpected Java 26.2 block fluid-support field count at {} line {line_index}",
            path.display()
        );
        let state_id = fields[0].parse::<usize>().unwrap_or_else(|error| {
            panic!(
                "invalid Java 26.2 block fluid-support state id `{}` at {} line {line_index}: {error}",
                fields[0],
                path.display()
            )
        });
        assert_minecraft_location(&fields[1], path, "fluid-support block id", line_index);

        let solid = parse_oracle_bool(&fields[2], path, "solid", line_index);
        let blocks_motion = parse_oracle_bool(&fields[3], path, "blocks_motion", line_index);
        assert!(
            solid || !blocks_motion,
            "Java 26.2 block fluid-support state {state_id} blocks motion while not solid at {} line {line_index}",
            path.display()
        );

        let face_sturdy = [
            parse_oracle_bool(&fields[4], path, "face_sturdy_down", line_index),
            parse_oracle_bool(&fields[5], path, "face_sturdy_up", line_index),
            parse_oracle_bool(&fields[6], path, "face_sturdy_north", line_index),
            parse_oracle_bool(&fields[7], path, "face_sturdy_south", line_index),
            parse_oracle_bool(&fields[8], path, "face_sturdy_west", line_index),
            parse_oracle_bool(&fields[9], path, "face_sturdy_east", line_index),
        ];

        let entry = JavaBlockFluidSupportEntry {
            state_id,
            block_id: fields[1].clone(),
            solid,
            blocks_motion,
            face_sturdy,
            fluid_side_overlay: parse_oracle_bool(
                &fields[10],
                path,
                "fluid_side_overlay",
                line_index,
            ),
            ice_block: parse_oracle_bool(&fields[11], path, "ice_block", line_index),
        };
        assert!(
            entries.insert(state_id, entry).is_none(),
            "duplicate Java 26.2 block fluid-support state id {state_id} at {} line {line_index}",
            path.display()
        );
    }
    assert!(
        !entries.is_empty(),
        "Java 26.2 block fluid-support oracle at {} had no data rows",
        path.display()
    );
    for (expected, actual) in entries.keys().enumerate() {
        assert_eq!(
            expected,
            *actual,
            "non-contiguous Java 26.2 block fluid-support state id at {}",
            path.display()
        );
    }
    entries.into_values().collect()
}

struct JavaBlockSoundTypeEntry {
    state_id: usize,
    block_id: String,
    volume: f32,
    pitch: f32,
    sounds: [String; 5],
}

fn load_java_26_2_block_sound_type(path: &Path) -> Vec<JavaBlockSoundTypeEntry> {
    const HEADER: &str =
        "id,block,volume,pitch,break_sound,step_sound,place_sound,hit_sound,fall_sound";
    let mut entries = BTreeMap::<usize, JavaBlockSoundTypeEntry>::new();
    for (index, fields) in read_oracle_csv(path, HEADER).into_iter().enumerate() {
        let line_index = index + 2;
        assert_eq!(
            fields.len(),
            9,
            "unexpected Java 26.2 block sound-type field count at {} line {line_index}",
            path.display()
        );
        let state_id = fields[0].parse::<usize>().unwrap_or_else(|error| {
            panic!(
                "invalid Java 26.2 block sound-type state id `{}` at {} line {line_index}: {error}",
                fields[0],
                path.display()
            )
        });
        assert_minecraft_location(&fields[1], path, "sound-type block id", line_index);
        let volume = parse_oracle_f32(&fields[2], path, "volume", line_index);
        let pitch = parse_oracle_f32(&fields[3], path, "pitch", line_index);
        assert!(
            volume > 0.0 && volume.is_finite() && pitch > 0.0 && pitch.is_finite(),
            "out-of-range Java 26.2 sound-type volume/pitch at {} line {line_index}",
            path.display()
        );
        for field in &fields[4..9] {
            assert_minecraft_location(field, path, "sound event id", line_index);
        }
        let entry = JavaBlockSoundTypeEntry {
            state_id,
            block_id: fields[1].clone(),
            volume,
            pitch,
            sounds: [
                fields[4].clone(),
                fields[5].clone(),
                fields[6].clone(),
                fields[7].clone(),
                fields[8].clone(),
            ],
        };
        assert!(
            entries.insert(state_id, entry).is_none(),
            "duplicate Java 26.2 block sound-type state id {state_id} at {} line {line_index}",
            path.display()
        );
    }
    assert!(
        !entries.is_empty(),
        "Java 26.2 block sound-type oracle at {} had no data rows",
        path.display()
    );
    for (expected, actual) in entries.keys().enumerate() {
        assert_eq!(
            expected,
            *actual,
            "non-contiguous Java 26.2 block sound-type state id at {}",
            path.display()
        );
    }
    entries.into_values().collect()
}

fn generate_java_26_2_block_sound_type_table(oracle: &Path) -> String {
    let entries = load_java_26_2_block_sound_type(oracle);
    let blocks = canonical_state_blocks(oracle.parent().expect("sound input directory"));
    let mut kinds = Vec::new();
    let mut indices = Vec::with_capacity(entries.len());
    for entry in &entries {
        assert_eq!(
            blocks[entry.state_id], entry.block_id,
            "sound state/block identity"
        );
        let key = (
            entry.volume.to_bits(),
            entry.pitch.to_bits(),
            entry.sounds.clone(),
        );
        let index = kinds
            .iter()
            .position(|candidate| candidate == &key)
            .unwrap_or_else(|| {
                kinds.push(key);
                kinds.len() - 1
            });
        indices.push(index);
    }
    let mut output = String::from(
        r#"#[derive(Clone, Copy)]
pub(crate) struct GeneratedJavaBlockSoundType {
    pub(crate) volume: f32,
    pub(crate) pitch: f32,
    pub(crate) sounds: [&'static str; 5],
}
static GENERATED_JAVA_26_2_SOUND_TYPES: &[GeneratedJavaBlockSoundType] = &[
"#,
    );
    for (volume, pitch, sounds) in &kinds {
        output.push_str(&format!("    GeneratedJavaBlockSoundType {{ volume: f32::from_bits({volume}), pitch: f32::from_bits({pitch}), sounds: [{:?}, {:?}, {:?}, {:?}, {:?}] }},\n", sounds[0], sounds[1], sounds[2], sounds[3], sounds[4]));
    }
    output.push_str("];\n");
    write_index_table(
        &mut output,
        "GENERATED_JAVA_26_2_BLOCK_SOUND_TYPE_INDEX",
        &indices,
    );
    output.push_str(r#"
pub(crate) fn java_26_2_block_sound_type(state_id: usize, block_id: &str) -> Option<GeneratedJavaBlockSoundType> {
    if crate::registry::RAW_BLOCK_STATES.get(state_id)?.0 != block_id { return None; }
    let index = usize::from(*GENERATED_JAVA_26_2_BLOCK_SOUND_TYPE_INDEX.get(state_id)?);
    GENERATED_JAVA_26_2_SOUND_TYPES.get(index).copied()
}
"#);
    output
}

fn generate_java_26_2_block_fluid_support_table(oracle: &Path) -> String {
    let entries = load_java_26_2_block_fluid_support(oracle);
    let blocks = canonical_state_blocks(oracle.parent().expect("fluid input directory"));
    let mut kinds = Vec::new();
    let mut indices = Vec::with_capacity(entries.len());
    for entry in &entries {
        assert_eq!(
            blocks[entry.state_id], entry.block_id,
            "fluid state/block identity"
        );
        let key = (
            entry.solid,
            entry.blocks_motion,
            entry.face_sturdy,
            entry.fluid_side_overlay,
            entry.ice_block,
        );
        let index = kinds
            .iter()
            .position(|candidate| candidate == &key)
            .unwrap_or_else(|| {
                kinds.push(key);
                kinds.len() - 1
            });
        indices.push(index);
    }
    let mut output = String::from(
        r#"#[derive(Clone, Copy)]
pub(crate) struct GeneratedJavaBlockFluidSupport {
    pub(crate) state_id: usize,
    pub(crate) block_id: &'static str,
    pub(crate) solid: bool,
    pub(crate) blocks_motion: bool,
    pub(crate) face_sturdy: [bool; 6],
    pub(crate) fluid_side_overlay: bool,
    pub(crate) ice_block: bool,
}
static GENERATED_JAVA_26_2_FLUID_VALUES: &[(bool, bool, [bool; 6], bool, bool)] = &[
"#,
    );
    for (solid, motion, faces, overlay, ice) in &kinds {
        output.push_str(&format!(
            "    ({solid}, {motion}, {faces:?}, {overlay}, {ice}),\n"
        ));
    }
    output.push_str("];\n");
    write_index_table(
        &mut output,
        "GENERATED_JAVA_26_2_BLOCK_FLUID_SUPPORT_INDEX",
        &indices,
    );
    output.push_str(r#"
pub(crate) fn java_26_2_block_fluid_support(state_id: usize, block_id: &str) -> Option<GeneratedJavaBlockFluidSupport> {
    let canonical_block = crate::registry::RAW_BLOCK_STATES.get(state_id)?.0;
    if canonical_block != block_id { return None; }
    let index = usize::from(*GENERATED_JAVA_26_2_BLOCK_FLUID_SUPPORT_INDEX.get(state_id)?);
    let &(solid, blocks_motion, face_sturdy, fluid_side_overlay, ice_block) = GENERATED_JAVA_26_2_FLUID_VALUES.get(index)?;
    Some(GeneratedJavaBlockFluidSupport { state_id, block_id: canonical_block, solid, blocks_motion, face_sturdy, fluid_side_overlay, ice_block })
}
"#);
    output
}

fn generate_java_26_2_block_lighting_table(oracle: &Path) -> String {
    let entries = load_java_26_2_block_lighting(oracle);
    let blocks = canonical_state_blocks(oracle.parent().expect("lighting input directory"));
    let mut kinds = Vec::new();
    let mut indices = Vec::with_capacity(entries.len());
    for entry in &entries {
        assert_eq!(
            blocks[entry.state_id], entry.block_id,
            "lighting state/block identity"
        );
        let key = (
            entry.light_emission,
            entry.emissive_rendering,
            entry.solid_render,
            entry.light_dampening,
            entry.view_blocking,
            entry.shade_brightness.to_bits(),
        );
        let index = kinds
            .iter()
            .position(|candidate| candidate == &key)
            .unwrap_or_else(|| {
                kinds.push(key);
                kinds.len() - 1
            });
        indices.push(index);
    }
    let mut output = String::from(
        r#"#[derive(Clone, Copy)]
pub(crate) struct GeneratedJavaBlockLighting {
    pub(crate) state_id: usize,
    pub(crate) block_id: &'static str,
    pub(crate) light_emission: u8,
    pub(crate) emissive_rendering: bool,
    pub(crate) solid_render: bool,
    pub(crate) light_dampening: u8,
    pub(crate) view_blocking: bool,
    pub(crate) shade_brightness: f32,
}
static GENERATED_JAVA_26_2_LIGHTING_VALUES: &[(u8, bool, bool, u8, bool, f32)] = &[
"#,
    );
    for (emission, emissive, solid, dampening, view, brightness) in &kinds {
        output.push_str(&format!("    ({emission}, {emissive}, {solid}, {dampening}, {view}, f32::from_bits({brightness})),\n"));
    }
    output.push_str("];\n");
    write_index_table(
        &mut output,
        "GENERATED_JAVA_26_2_BLOCK_LIGHTING_INDEX",
        &indices,
    );
    output.push_str(r#"
pub(crate) fn java_26_2_block_lighting(state_id: usize, block_id: &str) -> Option<GeneratedJavaBlockLighting> {
    let canonical_block = crate::registry::RAW_BLOCK_STATES.get(state_id)?.0;
    if canonical_block != block_id { return None; }
    let index = usize::from(*GENERATED_JAVA_26_2_BLOCK_LIGHTING_INDEX.get(state_id)?);
    let &(light_emission, emissive_rendering, solid_render, light_dampening, view_blocking, shade_brightness) = GENERATED_JAVA_26_2_LIGHTING_VALUES.get(index)?;
    Some(GeneratedJavaBlockLighting { state_id, block_id: canonical_block, light_emission, emissive_rendering, solid_render, light_dampening, view_blocking, shade_brightness })
}
"#);
    output
}

fn load_java_26_2_vanilla_tools(path: &Path) -> BTreeMap<String, JavaToolDefinition> {
    const HEADER: &str =
        "item,default_mining_speed,rule_index,selector_kind,selector,speed,correct_for_drops";
    let mut tools = BTreeMap::<String, JavaToolDefinition>::new();
    for (index, fields) in read_oracle_csv(path, HEADER).into_iter().enumerate() {
        let line_index = index + 2;
        assert_eq!(
            fields.len(),
            7,
            "unexpected Java 26.2 tool-oracle field count at {} line {line_index}",
            path.display()
        );
        let item_id = &fields[0];
        assert_minecraft_location(item_id, path, "item id", line_index);
        let default_mining_speed =
            parse_oracle_f32(&fields[1], path, "default_mining_speed", line_index);
        let selector_kind = fields[3].as_str();
        let definition = tools
            .entry(item_id.clone())
            .or_insert_with(|| JavaToolDefinition {
                default_mining_speed,
                rules: BTreeMap::new(),
                saw_no_rules_row: false,
            });
        assert_eq!(
            definition.default_mining_speed.to_bits(),
            default_mining_speed.to_bits(),
            "inconsistent Java 26.2 default_mining_speed for `{item_id}` at {} line {line_index}",
            path.display()
        );

        if selector_kind == "none" {
            assert_eq!(
                fields[2],
                "-1",
                "Java 26.2 no-rule tool row for `{item_id}` has a rule index at {} line {line_index}",
                path.display()
            );
            assert!(
                fields[4].is_empty() && fields[5].is_empty() && fields[6].is_empty(),
                "Java 26.2 no-rule tool row for `{item_id}` carries rule data at {} line {line_index}",
                path.display()
            );
            assert!(
                definition.rules.is_empty() && !definition.saw_no_rules_row,
                "duplicate or mixed no-rule Java 26.2 tool rows for `{item_id}` at {} line {line_index}",
                path.display()
            );
            definition.saw_no_rules_row = true;
            continue;
        }

        assert!(
            !definition.saw_no_rules_row,
            "mixed no-rule and rule Java 26.2 tool rows for `{item_id}` at {} line {line_index}",
            path.display()
        );
        let rule_index = fields[2].parse::<usize>().unwrap_or_else(|error| {
            panic!(
                "invalid Java 26.2 tool rule_index `{}` for `{item_id}` at {} line {line_index}: {error}",
                fields[2],
                path.display()
            )
        });
        let selector = match selector_kind {
            "tag" => {
                assert_minecraft_location(&fields[4], path, "tool tag selector", line_index);
                JavaToolSelector::Tag(fields[4].clone())
            }
            "blocks" => {
                let block_ids = fields[4].split('|').map(str::to_owned).collect::<Vec<_>>();
                assert!(
                    !block_ids.is_empty() && !block_ids.iter().any(String::is_empty),
                    "empty Java 26.2 direct tool selector for `{item_id}` at {} line {line_index}",
                    path.display()
                );
                for block_id in &block_ids {
                    assert_minecraft_location(block_id, path, "tool block selector", line_index);
                }
                let mut sorted = block_ids;
                sorted.sort();
                sorted.dedup();
                JavaToolSelector::DirectBlocks(sorted)
            }
            _ => panic!(
                "unsupported Java 26.2 tool selector kind `{selector_kind}` for `{item_id}` at {} line {line_index}",
                path.display()
            ),
        };
        let speed = if fields[5].is_empty() {
            None
        } else {
            Some(parse_oracle_f32(&fields[5], path, "tool speed", line_index))
        };
        let correct_for_drops = if fields[6].is_empty() {
            None
        } else {
            Some(parse_oracle_bool(
                &fields[6],
                path,
                "tool correct_for_drops",
                line_index,
            ))
        };
        assert!(
            definition
                .rules
                .insert(
                    rule_index,
                    JavaToolRule {
                        selector,
                        speed,
                        correct_for_drops,
                    },
                )
                .is_none(),
            "duplicate Java 26.2 tool rule index {rule_index} for `{item_id}` at {} line {line_index}",
            path.display()
        );
    }

    assert!(
        !tools.is_empty(),
        "Java 26.2 item-tool oracle at {} had no tool rows",
        path.display()
    );
    for (item_id, definition) in &tools {
        if definition.saw_no_rules_row {
            continue;
        }
        for (expected, actual) in definition.rules.keys().enumerate() {
            assert_eq!(
                expected,
                *actual,
                "non-contiguous Java 26.2 tool rule index for `{item_id}` at {}",
                path.display()
            );
        }
    }
    tools
}

struct JavaBlockFlammabilityEntry {
    block_id: String,
    ignited_by_lava: bool,
    ignite_odds: u16,
    burn_odds: u16,
}

fn load_java_26_2_flammability(path: &Path) -> Vec<JavaBlockFlammabilityEntry> {
    const HEADER: &str = "block,ignited_by_lava,ignite_odds,burn_odds";
    let mut entries = BTreeMap::new();
    for (index, fields) in read_oracle_csv(path, HEADER).into_iter().enumerate() {
        let line_index = index + 2;
        assert_eq!(
            fields.len(),
            4,
            "unexpected Java 26.2 flammability field count at {} line {line_index}",
            path.display()
        );
        let block_id = &fields[0];
        assert_minecraft_location(block_id, path, "flammability block id", line_index);
        let ignite_odds = parse_oracle_u16(&fields[2], path, "ignite_odds", line_index);
        let burn_odds = parse_oracle_u16(&fields[3], path, "burn_odds", line_index);
        assert!(
            (ignite_odds == 0 && burn_odds == 0) || (ignite_odds > 0 && burn_odds > 0),
            "Java 26.2 flammability row for `{block_id}` has a lone fire odds value at {} line {line_index}",
            path.display()
        );
        let entry = JavaBlockFlammabilityEntry {
            block_id: block_id.clone(),
            ignited_by_lava: parse_oracle_bool_or_zero_one(
                &fields[1],
                path,
                "ignited_by_lava",
                line_index,
            ),
            ignite_odds,
            burn_odds,
        };
        assert!(
            entries.insert(block_id.clone(), entry).is_none(),
            "duplicate Java 26.2 flammability row for `{block_id}` at {} line {line_index}",
            path.display()
        );
    }
    assert!(
        !entries.is_empty(),
        "Java 26.2 flammability oracle at {} had no data rows",
        path.display()
    );
    entries.into_values().collect()
}

fn parse_oracle_u16(value: &str, path: &Path, field: &str, line_index: usize) -> u16 {
    value.parse::<u16>().unwrap_or_else(|error| {
        panic!(
            "invalid Java 26.2 oracle {field} `{value}` at {} line {line_index}: {error}",
            path.display()
        )
    })
}

fn parse_oracle_bool_or_zero_one(value: &str, path: &Path, field: &str, line_index: usize) -> bool {
    match value {
        "1" => true,
        "0" => false,
        _ => panic!(
            "invalid Java 26.2 oracle {field} `{value}` at {} line {line_index}",
            path.display()
        ),
    }
}

fn generate_java_26_2_flammability_table(oracle: &Path) -> String {
    let entries = load_java_26_2_flammability(oracle);
    let mut output = String::from(
        r#"#[derive(Clone, Copy)]
pub(crate) struct GeneratedJavaBlockFlammability {
    pub(crate) block_id: &'static str,
    pub(crate) ignited_by_lava: bool,
    pub(crate) ignite_odds: u16,
    pub(crate) burn_odds: u16,
}

static GENERATED_JAVA_26_2_FLAMMABILITY: &[GeneratedJavaBlockFlammability] = &[
"#,
    );
    for entry in &entries {
        output.push_str(&format!(
            "    GeneratedJavaBlockFlammability {{ block_id: {:?}, ignited_by_lava: {}, ignite_odds: {}, burn_odds: {} }},\n",
            entry.block_id,
            entry.ignited_by_lava,
            entry.ignite_odds,
            entry.burn_odds,
        ));
    }
    output.push_str(
        r#"];

pub(crate) fn java_26_2_block_ignited_by_lava(block_id: &str) -> Option<bool> {
    Some(
        GENERATED_JAVA_26_2_FLAMMABILITY
            .binary_search_by(|entry| entry.block_id.cmp(block_id))
            .ok()
            .map(|index| GENERATED_JAVA_26_2_FLAMMABILITY[index].ignited_by_lava)?,
    )
}

pub(crate) fn java_26_2_block_fire_odds(block_id: &str) -> Option<(u16, u16)> {
    let index = GENERATED_JAVA_26_2_FLAMMABILITY
        .binary_search_by(|entry| entry.block_id.cmp(block_id))
        .ok()?;
    let entry = GENERATED_JAVA_26_2_FLAMMABILITY[index];
    (entry.ignite_odds > 0).then_some((entry.ignite_odds, entry.burn_odds))
}

"#,
    );
    output
}

fn rust_f32_expression(value: f32) -> String {
    assert!(value.is_finite(), "attempted to emit non-finite f32");
    format!("f32::from_bits(0x{:08X})", value.to_bits())
}

fn generate_java_26_2_mining_table(block_oracle: &Path, tool_oracle: &Path) -> String {
    let blocks = load_java_26_2_block_mining(block_oracle);
    let tools = load_java_26_2_vanilla_tools(tool_oracle);
    let mut output = String::from(
        "#[derive(Clone, Copy)]\n\
         enum GeneratedJavaToolSelector {\n\
             Tag(&'static str),\n\
             DirectBlocks(&'static [&'static str]),\n\
         }\n\n\
         #[derive(Clone, Copy)]\n\
         struct GeneratedJavaToolRule {\n\
             selector: GeneratedJavaToolSelector,\n\
             speed: Option<f32>,\n\
             correct_for_drops: Option<bool>,\n\
         }\n\n\
         #[derive(Clone, Copy)]\n\
         struct GeneratedJavaToolDefinition {\n\
             item_id: &'static str,\n\
             default_mining_speed: f32,\n\
             rules: &'static [GeneratedJavaToolRule],\n\
         }\n\n\
         #[derive(Clone, Copy)]\n\
         enum GeneratedJavaBlockOffsetType {\n\
             None,\n\
             Xz,\n\
             Xyz,\n\
         }\n\n\
         #[derive(Clone, Copy)]\n\
         enum GeneratedJavaBlockModelSeedType {\n\
             Standard,\n\
             DoubleBlockLower,\n\
             BedSource,\n\
         }\n\n\
         #[derive(Clone, Copy)]\n\
         struct GeneratedJavaBlockMining {\n\
             block_id: &'static str,\n\
             destroy_speed: f32,\n\
             requires_correct_tool: bool,\n\
             is_model_render_shape: bool,\n\
             offset_type: GeneratedJavaBlockOffsetType,\n\
             max_horizontal_offset: f32,\n\
             max_vertical_offset: f32,\n\
             offset_probe: [f32; 3],\n\
             model_seed_type: GeneratedJavaBlockModelSeedType,\n\
         }\n\n\
         static GENERATED_JAVA_26_2_BLOCK_MINING: &[GeneratedJavaBlockMining] = &[\n",
    );
    for entry in &blocks {
        let offset_type = match entry.offset_type {
            JavaBlockOffsetType::None => "GeneratedJavaBlockOffsetType::None",
            JavaBlockOffsetType::Xz => "GeneratedJavaBlockOffsetType::Xz",
            JavaBlockOffsetType::Xyz => "GeneratedJavaBlockOffsetType::Xyz",
        };
        let model_seed_type = match entry.model_seed_type {
            JavaBlockModelSeedType::Standard => "GeneratedJavaBlockModelSeedType::Standard",
            JavaBlockModelSeedType::DoubleBlockLower => {
                "GeneratedJavaBlockModelSeedType::DoubleBlockLower"
            }
            JavaBlockModelSeedType::BedSource => "GeneratedJavaBlockModelSeedType::BedSource",
        };
        output.push_str(&format!(
            "    GeneratedJavaBlockMining {{ block_id: {:?}, destroy_speed: {}, requires_correct_tool: {}, is_model_render_shape: {}, offset_type: {offset_type}, max_horizontal_offset: {}, max_vertical_offset: {}, offset_probe: [{}, {}, {}], model_seed_type: {model_seed_type} }},\n",
            entry.block_id,
            rust_f32_expression(entry.destroy_speed),
            entry.requires_correct_tool,
            entry.is_model_render_shape,
            rust_f32_expression(entry.max_horizontal_offset),
            rust_f32_expression(entry.max_vertical_offset),
            rust_f32_expression(entry.offset_probe[0]),
            rust_f32_expression(entry.offset_probe[1]),
            rust_f32_expression(entry.offset_probe[2]),
        ));
    }
    output.push_str(
        "];\n\n\
         fn generated_java_tool_selector_matches(selector: GeneratedJavaToolSelector, block_id: &str) -> bool {\n\
             match selector {\n\
                 GeneratedJavaToolSelector::Tag(tag_id) => super::is_java_26_2_vanilla_block_in_tag(block_id, tag_id),\n\
                 GeneratedJavaToolSelector::DirectBlocks(blocks) => blocks\n\
                     .binary_search_by(|candidate| (*candidate).cmp(block_id))\n\
                     .is_ok(),\n\
             }\n\
         }\n\n",
    );

    for (index, definition) in tools.values().enumerate() {
        output.push_str(&format!(
            "static GENERATED_JAVA_26_2_TOOL_RULES_{index}: &[GeneratedJavaToolRule] = &[\n"
        ));
        for rule in definition.rules.values() {
            let selector = match &rule.selector {
                JavaToolSelector::Tag(tag_id) => {
                    format!("GeneratedJavaToolSelector::Tag({tag_id:?})")
                }
                JavaToolSelector::DirectBlocks(blocks) => {
                    let members = blocks
                        .iter()
                        .map(|block_id| format!("{block_id:?}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("GeneratedJavaToolSelector::DirectBlocks(&[{members}])")
                }
            };
            let speed = rule
                .speed
                .map(rust_f32_expression)
                .map(|value| format!("Some({value})"))
                .unwrap_or_else(|| "None".to_owned());
            let correct_for_drops = rule
                .correct_for_drops
                .map(|value| format!("Some({value})"))
                .unwrap_or_else(|| "None".to_owned());
            output.push_str(&format!(
                "    GeneratedJavaToolRule {{ selector: {selector}, speed: {speed}, correct_for_drops: {correct_for_drops} }},\n"
            ));
        }
        output.push_str("];\n\n");
    }

    output.push_str(
        "static GENERATED_JAVA_26_2_TOOL_DEFINITIONS: &[GeneratedJavaToolDefinition] = &[\n",
    );
    for (index, (item_id, definition)) in tools.iter().enumerate() {
        output.push_str(&format!(
            "    GeneratedJavaToolDefinition {{ item_id: {item_id:?}, default_mining_speed: {}, rules: GENERATED_JAVA_26_2_TOOL_RULES_{index} }},\n",
            rust_f32_expression(definition.default_mining_speed)
        ));
    }
    output.push_str(
        "];\n\n\
         fn java_26_2_block_mining_entry(block_id: &str) -> Option<GeneratedJavaBlockMining> {\n\
             let index = GENERATED_JAVA_26_2_BLOCK_MINING\n\
                 .binary_search_by(|candidate| candidate.block_id.cmp(block_id))\n\
                 .ok()?;\n\
             Some(GENERATED_JAVA_26_2_BLOCK_MINING[index])\n\
         }\n\n\
         pub(crate) fn java_26_2_block_mining(block_id: &str) -> Option<(f32, bool)> {\n\
             let entry = java_26_2_block_mining_entry(block_id)?;\n\
             Some((entry.destroy_speed, entry.requires_correct_tool))\n\
         }\n\n\
         pub(crate) fn java_26_2_block_render_shape_is_model(block_id: &str) -> Option<bool> {\n\
             Some(java_26_2_block_mining_entry(block_id)?.is_model_render_shape)\n\
         }\n\n\
         pub(crate) fn java_26_2_block_model_seed_type(block_id: &str) -> Option<&'static str> {\n\
             Some(match java_26_2_block_mining_entry(block_id)?.model_seed_type {\n\
                 GeneratedJavaBlockModelSeedType::Standard => \"STANDARD\",\n\
                 GeneratedJavaBlockModelSeedType::DoubleBlockLower => \"DOUBLE_BLOCK_LOWER\",\n\
                 GeneratedJavaBlockModelSeedType::BedSource => \"BED_SOURCE\",\n\
             })\n\
         }\n\n\
         fn generated_java_26_2_offset_seed(x: i32, z: i32) -> i64 {\n\
             let x_component = x.wrapping_mul(3_129_871);\n\
             let mut seed = i64::from(x_component)\n\
                 ^ i64::from(z).wrapping_mul(116_129_781);\n\
             seed = seed\n\
                 .wrapping_mul(seed)\n\
                 .wrapping_mul(42_317_861)\n\
                 .wrapping_add(seed.wrapping_mul(11));\n\
             seed >> 16\n\
         }\n\n\
         fn generated_java_26_2_block_model_offset(\n\
             offset_type: GeneratedJavaBlockOffsetType,\n\
             max_horizontal_offset: f32,\n\
             max_vertical_offset: f32,\n\
             pos: [i32; 3],\n\
         ) -> [f32; 3] {\n\
             if matches!(offset_type, GeneratedJavaBlockOffsetType::None) {\n\
                 return [0.0, 0.0, 0.0];\n\
             }\n\
             let seed = generated_java_26_2_offset_seed(pos[0], pos[2]);\n\
             let horizontal_x = (f64::from((seed & 15) as f32 / 15.0) - 0.5) * 0.5;\n\
             let horizontal_z = (f64::from(((seed >> 8) & 15) as f32 / 15.0) - 0.5) * 0.5;\n\
             let min_horizontal = -f64::from(max_horizontal_offset);\n\
             let max_horizontal = f64::from(max_horizontal_offset);\n\
             let x = horizontal_x.clamp(min_horizontal, max_horizontal) as f32;\n\
             let z = horizontal_z.clamp(min_horizontal, max_horizontal) as f32;\n\
             let y = match offset_type {\n\
                 GeneratedJavaBlockOffsetType::Xz => 0.0,\n\
                 GeneratedJavaBlockOffsetType::Xyz => {\n\
                     ((f64::from(((seed >> 4) & 15) as f32 / 15.0) - 1.0)\n\
                         * f64::from(max_vertical_offset)) as f32\n\
                 }\n\
                 GeneratedJavaBlockOffsetType::None => unreachable!(\"NONE returned above\"),\n\
             };\n\
             [x, y, z]\n\
         }\n\n\
         pub(crate) fn java_26_2_block_model_offset(\n\
             block_id: &str,\n\
             pos: [i32; 3],\n\
         ) -> Option<[f32; 3]> {\n\
             let entry = java_26_2_block_mining_entry(block_id)?;\n\
             Some(generated_java_26_2_block_model_offset(\n\
                 entry.offset_type,\n\
                 entry.max_horizontal_offset,\n\
                 entry.max_vertical_offset,\n\
                 pos,\n\
             ))\n\
         }\n\n\
         fn java_26_2_tool_definition(item_id: &str) -> Option<GeneratedJavaToolDefinition> {\n\
             let index = GENERATED_JAVA_26_2_TOOL_DEFINITIONS\n\
                 .binary_search_by(|candidate| candidate.item_id.cmp(item_id))\n\
                 .ok()?;\n\
             Some(GENERATED_JAVA_26_2_TOOL_DEFINITIONS[index])\n\
         }\n\n\
         pub(crate) fn java_26_2_tool_mining_speed(item_id: &str, block_id: &str) -> Option<f32> {\n\
             let tool = java_26_2_tool_definition(item_id)?;\n\
             for rule in tool.rules {\n\
                 if let Some(speed) = rule.speed\n\
                     && generated_java_tool_selector_matches(rule.selector, block_id)\n\
                 {\n\
                     return Some(speed);\n\
                 }\n\
             }\n\
             Some(tool.default_mining_speed)\n\
         }\n\n\
         pub(crate) fn java_26_2_tool_correct_for_drops(item_id: &str, block_id: &str) -> Option<bool> {\n\
             let tool = java_26_2_tool_definition(item_id)?;\n\
             for rule in tool.rules {\n\
                 if let Some(correct_for_drops) = rule.correct_for_drops\n\
                     && generated_java_tool_selector_matches(rule.selector, block_id)\n\
                 {\n\
                     return Some(correct_for_drops);\n\
                 }\n\
             }\n\
             Some(false)\n\
         }\n\n\
",
    );
    output
}

fn main() {
    let (data_dir, output_dir, check_only) = arguments();
    if !check_only {
        fs::create_dir_all(&output_dir).expect("create generated output directory");
    }
    let source_path = data_dir.join("azalea_blocks.rs");

    let source = fs::read_to_string(&source_path).unwrap_or_else(|error| {
        panic!(
            "failed to read generated Azalea collision shapes at {}: {error}",
            source_path.display()
        )
    });
    let shapes_start = source
        .find("static SHAPE0")
        .expect("Azalea collision shape source has no SHAPE0");
    let map_start = source
        .find("static COLLISION_SHAPES_MAP")
        .expect("Azalea collision shape source has no collision map");
    let implementation_start = source
        .find("impl BlockWithShape for BlockState")
        .expect("Azalea collision shape source has no BlockWithShape implementation");
    let map_end = source[map_start..]
        .find("];\n")
        .map(|offset| map_start + offset + 3)
        .expect("Azalea collision shape map is not terminated");

    let shape_definitions = source[shapes_start..implementation_start]
        .replace("collision::box_shape", "box_shape")
        .replace("collision::EMPTY_SHAPE", "EMPTY_SHAPE")
        .replace("Shapes::or", "or_shape");
    let collision_map = &source[map_start..map_end];
    let generated = format!(
        "{header}\n{shape_definitions}\n{collision_map}\n{footer}\n",
        header = r#"use el_math::{AABB, VoxelShape};
use std::sync::LazyLock;

static EMPTY_SHAPE: LazyLock<VoxelShape> = LazyLock::new(VoxelShape::empty);

fn box_shape(min_x: f64, min_y: f64, min_z: f64, max_x: f64, max_y: f64, max_z: f64) -> VoxelShape {
    if max_x - min_x < 1.0e-7 && max_y - min_y < 1.0e-7 && max_z - min_z < 1.0e-7 {
        VoxelShape::empty()
    } else {
        VoxelShape::from_aabb(AABB::new(min_x, min_y, min_z, max_x, max_y, max_z))
    }
}

fn or_shape(left: VoxelShape, right: VoxelShape) -> VoxelShape {
    if left.is_empty() {
        return right;
    }
    if right.is_empty() {
        return left;
    }
    let mut boxes = left.to_aabbs().to_vec();
    boxes.extend_from_slice(right.to_aabbs());
    VoxelShape::from_aabbs(boxes)
}"#,
        footer = r#"pub(crate) fn shape_for_state_id(id: usize) -> Option<VoxelShape> {
    COLLISION_SHAPES_MAP.get(id).map(|shape| (**shape).clone())
}"#,
    );

    let output_path = output_dir.clone().join("generated_collision_shapes.rs");
    write_generated(&output_path, &generated, check_only);

    let java_block_tags = data_dir.join("tags/block");

    let generated_block_tags = p5_tag_gen::generate_java_26_2_block_tag_table(&java_block_tags);
    let output_path = output_dir.clone().join("generated_java_26_2_block_tags.rs");
    write_generated(&output_path, &generated_block_tags, check_only);

    let java_block_mining = data_dir.join("block_mining.csv");
    let java_item_tools = data_dir.join("item_tool_components.csv");

    let generated_mining = generate_java_26_2_mining_table(&java_block_mining, &java_item_tools);
    let output_path = output_dir.clone().join("generated_java_26_2_mining.rs");
    write_generated(&output_path, &generated_mining, check_only);

    let java_block_lighting = data_dir.join("block_lighting.csv");

    let generated_lighting = generate_java_26_2_block_lighting_table(&java_block_lighting);
    let output_path = output_dir
        .clone()
        .join("generated_java_26_2_block_lighting.rs");
    write_generated(&output_path, &generated_lighting, check_only);

    let java_block_fluid_support = data_dir.join("block_fluid_support.csv");

    let generated_fluid_support =
        generate_java_26_2_block_fluid_support_table(&java_block_fluid_support);
    let output_path = output_dir
        .clone()
        .join("generated_java_26_2_block_fluid_support.rs");
    write_generated(&output_path, &generated_fluid_support, check_only);

    let java_block_sound_type = data_dir.join("block_sound_type.csv");

    let generated_sound_type = generate_java_26_2_block_sound_type_table(&java_block_sound_type);
    let output_path = output_dir
        .clone()
        .join("generated_java_26_2_block_sound_type.rs");
    write_generated(&output_path, &generated_sound_type, check_only);

    let java_block_flammability = data_dir.join("flammability.csv");

    let generated_flammability = generate_java_26_2_flammability_table(&java_block_flammability);
    let output_path = output_dir
        .clone()
        .join("generated_java_26_2_flammability.rs");
    write_generated(&output_path, &generated_flammability, check_only);

    let generated_registry = generate_registry_data(&data_dir);
    let output_path = output_dir.clone().join("generated_registry_data.rs");
    write_generated(&output_path, &generated_registry, check_only);
    let provenance = generate_provenance(&data_dir);
    write_generated(&output_dir.join("PROVENANCE.txt"), &provenance, check_only);
}

fn generate_registry_data(oracle_dir: &Path) -> String {
    let mut out = String::new();

    let block_states_path = oracle_dir.join("block_states.csv");

    let content = fs::read_to_string(&block_states_path)
        .unwrap_or_else(|_| panic!("failed to read {}", block_states_path.display()));
    assert!(
        content
            .lines()
            .next()
            .is_some_and(|header| header == "id,block,properties"),
        "unexpected registry schema at {}",
        block_states_path.display()
    );

    out.push_str("pub static RAW_BLOCK_STATES: &[(&'static str, &'static [(&'static str, &'static str)])] = &[\n");
    for (idx, line) in content.lines().enumerate() {
        if idx == 0 || line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, ',').collect();
        let block_name = parts[1];
        let mut props_code = String::from("&[");
        if parts.len() == 3 && !parts[2].is_empty() {
            let mut props_entries = Vec::new();
            for prop in parts[2].split(',') {
                let kv: Vec<&str> = prop.splitn(2, '=').collect();
                if kv.len() == 2 {
                    props_entries.push(format!("(\"{}\", \"{}\")", kv[0], kv[1]));
                }
            }
            props_code.push_str(&props_entries.join(", "));
        }
        props_code.push(']');
        out.push_str(&format!("    (\"{block_name}\", {props_code}),\n"));
    }
    out.push_str("];\n\n");

    let blocks_path = oracle_dir.join("blocks.csv");

    let content = fs::read_to_string(&blocks_path)
        .unwrap_or_else(|_| panic!("failed to read {}", blocks_path.display()));
    assert!(
        content
            .lines()
            .next()
            .is_some_and(|header| header == "id,block,default_state_id"),
        "unexpected registry schema at {}",
        blocks_path.display()
    );
    out.push_str("pub static RAW_BLOCKS: &[(usize, &'static str, usize)] = &[\n");
    for (idx, line) in content.lines().enumerate() {
        if idx == 0 || line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        let id: usize = parts[0].parse().unwrap();
        let name = parts[1];
        let default_state_id: usize = parts[2].parse().unwrap();
        out.push_str(&format!("    ({id}, \"{name}\", {default_state_id}),\n"));
    }
    out.push_str("];\n\n");

    let items_path = oracle_dir.join("items.csv");

    let content = fs::read_to_string(&items_path)
        .unwrap_or_else(|_| panic!("failed to read {}", items_path.display()));
    assert!(
        content
            .lines()
            .next()
            .is_some_and(|header| header
                == "id,item,max_stack_size,max_damage,nutrition,saturation,always_eat"),
        "unexpected registry schema at {}",
        items_path.display()
    );
    out.push_str("#[derive(Clone, Copy, Debug)]\npub struct RawItemData {\n    pub id: usize,\n    pub name: &'static str,\n    pub max_stack_size: i32,\n    pub max_damage: Option<i32>,\n    pub nutrition: Option<i32>,\n    pub saturation: Option<f32>,\n    pub always_eat: bool,\n}\n\npub static RAW_ITEMS: &[RawItemData] = &[\n");
    for (idx, line) in content.lines().enumerate() {
        if idx == 0 || line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        let id: usize = parts[0].parse().unwrap();
        let name = parts[1];
        let max_stack_size: i32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(64);
        let max_damage = parts.get(3).and_then(|s| {
            if s.is_empty() {
                None
            } else {
                s.parse::<i32>().ok()
            }
        });
        let max_damage_code = match max_damage {
            Some(d) => format!("Some({d})"),
            None => "None".to_string(),
        };
        let nutrition = parts.get(4).and_then(|s| {
            if s.is_empty() {
                None
            } else {
                s.parse::<i32>().ok()
            }
        });
        let nutrition_code = match nutrition {
            Some(n) => format!("Some({n})"),
            None => "None".to_string(),
        };
        let saturation = parts.get(5).and_then(|s| {
            if s.is_empty() {
                None
            } else {
                s.parse::<f32>().ok()
            }
        });
        let saturation_code = match saturation {
            Some(s) => format!("Some({s:.2})"),
            None => "None".to_string(),
        };
        let always_eat = parts
            .get(6)
            .map_or(false, |s| s.trim() == "true" || s.trim() == "1");

        out.push_str(&format!(
            "    RawItemData {{ id: {id}, name: \"{name}\", max_stack_size: {max_stack_size}, max_damage: {max_damage_code}, nutrition: {nutrition_code}, saturation: {saturation_code}, always_eat: {always_eat} }},\n"
        ));
    }
    out.push_str("];\n\n");

    let entities_path = oracle_dir.join("entities.csv");

    let content = fs::read_to_string(&entities_path)
        .unwrap_or_else(|_| panic!("failed to read {}", entities_path.display()));
    assert!(
        content
            .lines()
            .next()
            .is_some_and(|header| header == "id,entity"),
        "unexpected registry schema at {}",
        entities_path.display()
    );
    out.push_str("pub static RAW_ENTITIES: &[(usize, &'static str)] = &[\n");
    for (idx, line) in content.lines().enumerate() {
        if idx == 0 || line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        let id: usize = parts[0].parse().unwrap();
        let name = parts[1];
        out.push_str(&format!("    ({id}, \"{name}\"),\n"));
    }
    out.push_str("];\n\n");

    let biomes_path = oracle_dir.join("biomes.csv");

    let content = fs::read_to_string(&biomes_path)
        .unwrap_or_else(|_| panic!("failed to read {}", biomes_path.display()));
    assert!(
        content
            .lines()
            .next()
            .is_some_and(|header| header == "id,biome"),
        "unexpected registry schema at {}",
        biomes_path.display()
    );
    out.push_str("pub static RAW_BIOMES: &[(usize, &'static str)] = &[\n");
    for (idx, line) in content.lines().enumerate() {
        if idx == 0 || line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        let id: usize = parts[0].parse().unwrap();
        let name = parts[1];
        out.push_str(&format!("    ({id}, \"{name}\"),\n"));
    }
    out.push_str("];\n");

    out
}

fn arguments() -> (PathBuf, PathBuf, bool) {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if !(args.len() == 2 || (args.len() == 3 && args[2] == "--check")) {
        panic!(
            "Usage: cargo run -p el_core --example regenerate_vanilla_tables -- INPUT_DIR OUTPUT_DIR [--check]"
        );
    }
    (
        PathBuf::from(&args[0]),
        PathBuf::from(&args[1]),
        args.len() == 3,
    )
}

fn write_generated(path: &Path, source: &str, check_only: bool) {
    let source = if path.extension().is_some_and(|extension| extension == "rs") {
        source
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    } else {
        source.to_owned()
    };
    if check_only {
        let existing =
            fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        assert!(
            existing == source.as_bytes(),
            "generated table differs: {}",
            path.display()
        );
    } else {
        fs::write(path, source).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    }
}

fn generate_provenance(data_dir: &Path) -> String {
    use sha2::{Digest, Sha256};
    let mut paths = vec![
        "azalea_blocks.rs",
        "biomes.csv",
        "block_fluid_support.csv",
        "block_lighting.csv",
        "block_mining.csv",
        "block_sound_type.csv",
        "block_states.csv",
        "blocks.csv",
        "entities.csv",
        "flammability.csv",
        "item_tool_components.csv",
        "items.csv",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect::<Vec<_>>();
    fn visit(root: &Path, directory: &Path, paths: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(directory).expect("read input tag directory") {
            let entry = entry.expect("read tag directory entry");
            if entry.file_type().expect("read tag file type").is_dir() {
                visit(root, &entry.path(), paths);
            } else if entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                paths.push(
                    entry
                        .path()
                        .strip_prefix(root)
                        .expect("tag input stays below root")
                        .to_path_buf(),
                );
            }
        }
    }
    visit(data_dir, &data_dir.join("tags/block"), &mut paths);
    paths.sort();
    let mut output = include_str!("registry_provenance.txt").to_owned();
    output.push_str("\nInput SHA-256 (relative to the explicitly selected input directory):\n");
    for relative in paths {
        let bytes = fs::read(data_dir.join(&relative))
            .unwrap_or_else(|error| panic!("read {}: {error}", relative.display()));
        let hash = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        output.push_str(&format!(
            "{hash}  {}\n",
            relative.to_string_lossy().replace('\\', "/")
        ));
    }
    output
}

fn canonical_state_blocks(data_dir: &Path) -> Vec<String> {
    let path = data_dir.join("block_states.csv");
    let rows = read_oracle_csv(&path, "id,block,properties");
    assert_eq!(rows.len(), 32366, "Java 26.2 state count");
    rows.into_iter()
        .enumerate()
        .map(|(index, row)| {
            assert_eq!(
                row[0].parse::<usize>().expect("state ID"),
                index,
                "contiguous state IDs"
            );
            row[1].clone()
        })
        .collect()
}

fn write_index_table(output: &mut String, name: &str, indices: &[usize]) {
    output.push_str(&format!("static {name}: &[u8] = &[\n"));
    for chunk in indices.chunks(24) {
        output.push_str("    ");
        for (position, index) in chunk.iter().enumerate() {
            assert!(
                *index <= usize::from(u8::MAX),
                "value pool exceeds u8 index domain"
            );
            output.push_str(&format!("{index},"));
            if position + 1 < chunk.len() {
                output.push(' ');
            }
        }
        output.push('\n');
    }
    output.push_str("];\n");
}
