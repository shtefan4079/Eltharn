use super::*;
use el_mod_content::tooltip::{
    Canvas, Contents, Context, Flow, Item, LayoutContext, MAX_ITEMS, Placement, Stack, Text, Widget,
};
#[derive(Clone, Debug)]
pub(super) struct Presentation {
    pub canvas: Canvas,
    items: Vec<ItemStack>,
    textures: BTreeMap<String, usize>,
}
impl Presentation {
    pub fn inline_size(&self) -> (i32, i32) {
        match self.canvas.flow {
            Flow::Inline => (self.canvas.width as i32, self.canvas.height as i32),
            Flow::Overlay => (0, 0),
        }
    }
    pub fn render(
        &self,
        renderer: &mut GuiRenderer,
        font: &FontRenderer,
        x: i32,
        y: i32,
        available_width: i32,
    ) {
        let extra = (available_width - self.canvas.width as i32).max(0);
        let x = x + match self.canvas.align {
            el_mod_content::tooltip::Alignment::Left => 0,
            el_mod_content::tooltip::Alignment::Center => extra / 2,
            el_mod_content::tooltip::Alignment::Right => extra,
        };
        self.render_at(renderer, font, x, y);
    }
    pub fn render_at(&self, renderer: &mut GuiRenderer, font: &FontRenderer, x: i32, y: i32) {
        let previous_scissor = renderer.active_scissor;
        let mut scissor = [
            x as f32,
            y as f32,
            x as f32 + self.canvas.width as f32,
            y as f32 + self.canvas.height as f32,
        ];
        if let Some([left, top, right, bottom]) = previous_scissor {
            scissor = [
                scissor[0].max(left),
                scissor[1].max(top),
                scissor[2].min(right),
                scissor[3].min(bottom),
            ];
        }
        renderer.active_scissor = Some(scissor);
        for widget in &self.canvas.widgets {
            match widget {
                Widget::Panel {
                    rect: [left, top, w, h],
                    color,
                } => renderer.draw_rect(
                    (x + left) as f32,
                    (y + top) as f32,
                    *w as f32,
                    *h as f32,
                    edit_box_argb_to_rgba(*color as i32),
                ),
                Widget::Text {
                    position: [left, top],
                    text,
                    color,
                } => renderer.draw_text_top_with_shadow(
                    font,
                    text,
                    (x + left) as f32,
                    (y + top) as f32,
                    edit_box_argb_to_rgba(*color as i32),
                    1.0,
                ),
                Widget::Item {
                    reference,
                    position: [left, top],
                    count,
                    quantity,
                    components,
                } => {
                    if let Some(item) = self.items.get(*reference as usize) {
                        let count = if quantity.is_some() {
                            1
                        } else {
                            count.unwrap_or(item.count)
                        };
                        let item = if *components {
                            item.copy_with_count(count)
                        } else {
                            ItemStack::new(item.item.clone(), count)
                        };
                        draw_gui_item(
                            renderer,
                            font,
                            &item,
                            (x + left) as f32,
                            (y + top) as f32,
                            0.0,
                            true,
                        );
                        if let Some(text) = quantity {
                            renderer.draw_text_top_with_shadow(
                                font,
                                text,
                                (x + left + 17) as f32 - font.measure_width(text, 1.0),
                                (y + top + 9) as f32,
                                [1.0; 4],
                                1.0,
                            );
                        }
                    }
                }
                Widget::Image {
                    rect: [left, top, w, h],
                    texture,
                    uv,
                    color,
                    nine_slice,
                } => {
                    if let Some(index) = self.textures.get(texture) {
                        if let Some(slice) = nine_slice {
                            renderer.draw_custom_texture_nine_slice(
                                [(x + left) as f32, (y + top) as f32, *w as f32, *h as f32],
                                *index,
                                slice,
                                edit_box_argb_to_rgba(*color as i32),
                            );
                        } else {
                            renderer.draw_custom_texture_region(
                                (x + left) as f32,
                                (y + top) as f32,
                                *w as f32,
                                *h as f32,
                                uv[0] as f32 / 10000.0,
                                uv[1] as f32 / 10000.0,
                                uv[2] as f32 / 10000.0,
                                uv[3] as f32 / 10000.0,
                                *index,
                                edit_box_argb_to_rgba(*color as i32),
                            );
                        }
                    }
                }
                Widget::Sprite {
                    rect: [left, top, w, h],
                    sprite,
                    color,
                } => {
                    if let Ok(id) = sprite.parse::<el_mod_content::ContentId>() {
                        renderer.draw_sprite(
                            (x + left) as f32,
                            (y + top) as f32,
                            *w as f32,
                            *h as f32,
                            &ResourceLocation::new(id.namespace(), id.path()),
                            edit_box_argb_to_rgba(*color as i32),
                        );
                    }
                }
            }
        }
        renderer.active_scissor = previous_scissor;
    }
}
fn validate_sprites(canvas: &Canvas, renderer: &GuiRenderer) -> bool {
    canvas_quad_count(canvas, renderer).is_some_and(|count| count <= 4096)
}
fn canvas_quad_count(canvas: &Canvas, renderer: &GuiRenderer) -> Option<u32> {
    let mut total = 0u32;
    for widget in &canvas.widgets {
        let count = match widget {
            Widget::Sprite {
                rect: [_, _, w, h],
                sprite,
                ..
            } => {
                let Ok(id) = sprite.parse::<el_mod_content::ContentId>() else {
                    return None;
                };
                if id.namespace() != "minecraft" || id.path().ends_with(".png") {
                    return None;
                }
                let (Ok(w), Ok(h)) = (u32::try_from(*w), u32::try_from(*h)) else {
                    return None;
                };
                let Some(count) = renderer.sprite_quad_count(
                    &ResourceLocation::new(id.namespace(), id.path()),
                    w,
                    h,
                ) else {
                    return None;
                };
                count
            }
            Widget::Image {
                rect: [_, _, w, h],
                nine_slice,
                ..
            } => {
                let (Ok(w), Ok(h)) = (u32::try_from(*w), u32::try_from(*h)) else {
                    return None;
                };
                if w == 0 || h == 0 {
                    return None;
                }
                if let Some(slice) = nine_slice {
                    let [sw, sh] = slice.size.map(u32::from);
                    let [left, top, right, bottom] = slice.border.map(u32::from);
                    let (Some(inner_w), Some(inner_h)) =
                        (sw.checked_sub(left + right), sh.checked_sub(top + bottom))
                    else {
                        return None;
                    };
                    if inner_w == 0 || inner_h == 0 {
                        return None;
                    }
                    if slice.stretch_inner {
                        9
                    } else {
                        let Some(count) = w.div_ceil(inner_w).checked_add(2).and_then(|columns| {
                            h.div_ceil(inner_h)
                                .checked_add(2)
                                .and_then(|rows| columns.checked_mul(rows))
                        }) else {
                            return None;
                        };
                        count
                    }
                } else {
                    1
                }
            }
            _ => 0,
        };
        let Some(next) = total.checked_add(count) else {
            return None;
        };
        total = next;
        if total > 4096 {
            return None;
        }
    }
    Some(total)
}
pub(super) fn overlay_placements(
    tooltip: &DeferredTooltip,
    font: &FontRenderer,
    renderer: &GuiRenderer,
    layout: TooltipLayout,
    pointer: [i32; 2],
    viewport: [i32; 2],
) -> Option<BTreeMap<usize, Placement>> {
    let mut placements = BTreeMap::new();
    let mut owners = std::collections::BTreeSet::new();
    let mut component_y = layout.text_y;
    let mut quads = 0u32;
    for (index, component) in tooltip.components.iter().enumerate() {
        if let TooltipComponent::Image(TooltipImage::Extension(view)) = component {
            if view.canvas.flow == Flow::Overlay {
                if !owners.insert(view.canvas.owner.as_str()) {
                    return None;
                }
                let context = LayoutContext {
                    version: 1,
                    owner: view.canvas.owner.clone(),
                    viewport: [
                        u16::try_from(viewport[0]).ok()?,
                        u16::try_from(viewport[1]).ok()?,
                    ],
                    pointer,
                    tooltip: [
                        layout.text_x,
                        layout.text_y,
                        layout.text_width,
                        layout.text_height,
                    ],
                    component_origin: [layout.text_x, component_y],
                    canvas_size: [view.canvas.width, view.canvas.height],
                    align: view.canvas.align,
                    placement: None,
                };
                let placement = crate::modding::hooks::item_tooltip_layout(context)?;
                quads = quads.checked_add(canvas_quad_count(&view.canvas, renderer)?)?;
                if placement.background {
                    let [_, _, width, height] = tooltip_background_bounds(
                        placement.origin,
                        [view.canvas.width as i32, view.canvas.height as i32],
                    );
                    for part in ["background", "frame"] {
                        quads = quads.checked_add(renderer.sprite_quad_count(
                            &tooltip_sprite(None, part),
                            u32::try_from(width).ok()?,
                            u32::try_from(height).ok()?,
                        )?)?;
                    }
                }
                if quads > 4096 {
                    return None;
                }
                placements.insert(index, placement);
            }
        }
        component_y += component.height(font);
        if index == 0 {
            component_y += JAVA_TOOLTIP_FIRST_LINE_GAP;
        }
    }
    Some(placements)
}
fn same_components(left: &ItemStack, right: &ItemStack) -> bool {
    if left.item.location != right.item.location {
        return false;
    }
    left.components
        .prototype
        .keys()
        .chain(left.components.patch.map.keys())
        .chain(right.components.prototype.keys())
        .chain(right.components.patch.map.keys())
        .all(|key| {
            let left = left.components.get(key);
            let right = right.components.get(key);
            match (left, right) {
                (
                    Some(DataComponent::Generic {
                        type_name: left_type,
                        value: left @ Tag::Compound(_),
                    }),
                    Some(DataComponent::Generic {
                        type_name: right_type,
                        value: right @ Tag::Compound(_),
                    }),
                ) if key == "minecraft:custom_data"
                    && left_type == "minecraft:custom_data"
                    && right_type == "minecraft:custom_data" =>
                {
                    custom_data_tree_equal(left, right)
                }
                _ => left == right,
            }
        })
}
fn custom_data_tree_equal(left: &Tag, right: &Tag) -> bool {
    match (left, right) {
        (Tag::Compound(left), Tag::Compound(right)) => {
            let left: BTreeMap<&str, &Tag> = left
                .iter()
                .map(|(name, value)| (name.as_str(), value))
                .collect();
            let right: BTreeMap<&str, &Tag> = right
                .iter()
                .map(|(name, value)| (name.as_str(), value))
                .collect();
            left.len() == right.len()
                && left.into_iter().zip(right).all(
                    |((left_name, left_value), (right_name, right_value))| {
                        left_name == right_name && custom_data_tree_equal(left_value, right_value)
                    },
                )
        }
        (Tag::List(left), Tag::List(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| custom_data_tree_equal(left, right))
        }
        _ => left == right,
    }
}
const MAX_CONTENT_TAG_BYTES: usize = 64 * 1024;
const MAX_CONTENT_TAG_NODES: usize = 4096;
const MAX_CONTENT_TAG_DEPTH: usize = 16;
fn contents_tag_within_budget(tag: &Tag) -> bool {
    #[derive(Default)]
    struct Budget {
        nodes: usize,
        bytes: usize,
    }
    impl Budget {
        fn charge(&mut self, bytes: usize) -> bool {
            let Some(total) = self.bytes.checked_add(bytes) else {
                return false;
            };
            self.bytes = total;
            total <= MAX_CONTENT_TAG_BYTES
        }
        fn visit(&mut self, tag: &Tag, depth: usize) -> bool {
            if depth > MAX_CONTENT_TAG_DEPTH || self.nodes >= MAX_CONTENT_TAG_NODES {
                return false;
            }
            self.nodes += 1;
            if !self.charge(1) {
                return false;
            }
            match tag {
                Tag::End => true,
                Tag::Byte(_) => self.charge(1),
                Tag::Short(_) => self.charge(2),
                Tag::Int(_) | Tag::Float(_) => self.charge(4),
                Tag::Long(_) | Tag::Double(_) => self.charge(8),
                Tag::ByteArray(values) => self.charge(4) && self.charge(values.len()),
                Tag::IntArray(values) => {
                    self.charge(4)
                        && values
                            .len()
                            .checked_mul(4)
                            .is_some_and(|bytes| self.charge(bytes))
                }
                Tag::LongArray(values) => {
                    self.charge(4)
                        && values
                            .len()
                            .checked_mul(8)
                            .is_some_and(|bytes| self.charge(bytes))
                }
                Tag::String(value) => self.charge(2) && self.charge(value.len()),
                Tag::List(values) => {
                    values.len() <= MAX_CONTENT_TAG_NODES - self.nodes
                        && self.charge(5)
                        && values.iter().all(|value| self.visit(value, depth + 1))
                }
                Tag::Compound(fields) => {
                    fields.len() <= MAX_CONTENT_TAG_NODES - self.nodes
                        && self.charge(1)
                        && fields.iter().all(|(name, value)| {
                            self.charge(2)
                                && self.charge(name.len())
                                && self.visit(value, depth + 1)
                        })
                }
            }
        }
    }
    Budget::default().visit(tag, 0)
}
fn contents(item: &ItemStack) -> Option<(Contents, Vec<ItemStack>)> {
    let (component, value) = if let Some(value) = item.components.get("minecraft:container") {
        ("minecraft:container", value)
    } else {
        (
            "minecraft:bundle_contents",
            item.components.get("minecraft:bundle_contents")?,
        )
    };
    let DataComponent::Generic { value, .. } = value else {
        return None;
    };
    let Tag::List(entries) = value else {
        return None;
    };
    if entries.len() > 256
        || tooltip_component_hidden(item, component)
        || !contents_tag_within_budget(value)
    {
        return None;
    }
    let slots = if component == "minecraft:bundle_contents" {
        entries
            .iter()
            .map(bundle_item_stack_template_from_component_tag)
            .collect::<Option<Vec<_>>>()?
    } else if entries.iter().all(looks_like_persistent_container_slot) {
        persistent_container_slots_for_tooltip(entries)?
    } else {
        streamed_container_slots_for_tooltip(entries)?
    };
    let size = slots.len();
    let mut items = Vec::new();
    let mut references: Vec<ItemStack> = Vec::new();
    let mut classes: Vec<usize> = Vec::new();
    let total = slots.iter().filter(|item| !item.is_empty()).count();
    for (slot, item) in slots
        .into_iter()
        .enumerate()
        .filter(|(_, item)| !item.is_empty())
        .take(MAX_ITEMS)
    {
        let class = classes
            .iter()
            .copied()
            .find(|index| same_components(&references[*index], &item));
        let reference = references.len();
        let class = class.unwrap_or_else(|| {
            classes.push(reference);
            reference
        });
        items.push(Item {
            reference: reference as u16,
            slot: slot as u16,
            id: item.item.location.to_string(),
            count: item.count,
            component_key: class.to_string(),
        });
        references.push(item);
    }
    Some((
        Contents {
            component: component.into(),
            size: size as u16,
            truncated: total > MAX_ITEMS,
            selected_slot: None,
            items,
        },
        references,
    ))
}
fn input(
    tooltip: &DeferredTooltip,
    item: &ItemStack,
    mut modifiers: [bool; 3],
) -> (Context, Vec<ItemStack>) {
    let pressed = crate::input::button_state::snapshot();
    for (index, names) in [
        ["ShiftLeft", "ShiftRight"],
        ["ControlLeft", "ControlRight"],
        ["AltLeft", "AltRight"],
    ]
    .into_iter()
    .enumerate()
    {
        if pressed.iter().any(|key| names.contains(&key.as_str())) {
            modifiers[index] = true;
        }
    }
    let mut components: Vec<_> = item
        .components
        .prototype
        .keys()
        .chain(item.components.patch.map.keys())
        .filter(|key| !key.starts_with("eltharn:") && item.components.get(key).is_some())
        .cloned()
        .collect();
    components.sort();
    components.dedup();
    let contents = contents(item);
    let (mut contents, items) = match contents {
        Some((content, items)) => (Some(content), items),
        None => (None, Vec::new()),
    };
    if let Some(contents) = &mut contents {
        if contents.component == "minecraft:bundle_contents" {
            contents.selected_slot = tooltip
                .components
                .iter()
                .find_map(|component| match component {
                    TooltipComponent::Image(TooltipImage::Bundle(bundle)) => bundle.selected_item,
                    _ => None,
                })
                .filter(|&slot| slot < contents.size as usize)
                .map(|slot| slot as u16);
        }
    }
    (
        Context {
            version: 1,
            stack: Stack {
                id: item.item.location.to_string(),
                count: item.count,
                components,
            },
            contents,
            modifiers,
            buttons: el_mod_content::tooltip::Buttons {
                pressed,
                complete: !crate::input::button_state::overflowed(),
            },
            native_images: true,
            lines: tooltip
                .components
                .iter()
                .filter_map(|component| match component {
                    TooltipComponent::Text(line) => {
                        let [r, g, b, a] = line.color;
                        Some(Text {
                            text: line.styled_text(),
                            color: ((a * 255.0).round() as u32) << 24
                                | ((r * 255.0).round() as u32) << 16
                                | ((g * 255.0).round() as u32) << 8
                                | (b * 255.0).round() as u32,
                        })
                    }
                    _ => None,
                })
                .collect(),
            canvas: None,
        },
        items,
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn compose(
    tooltip: Option<DeferredTooltip>,
    item: &ItemStack,
    modifiers: [bool; 3],
    renderer: &mut GuiRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    resources: &crate::resources::manager::ClientResourceManager,
) -> Option<DeferredTooltip> {
    let tooltip = tooltip?;
    if !crate::modding::hooks::item_tooltip_active() {
        return Some(tooltip);
    }
    let (input, items) = input(&tooltip, item, modifiers);
    let result = match crate::modding::hooks::item_tooltip(&input) {
        crate::modding::hooks::ItemTooltipOutcome::Native => return Some(tooltip),
        crate::modding::hooks::ItemTooltipOutcome::Cancelled => return None,
        crate::modding::hooks::ItemTooltipOutcome::Presented(result) => result,
    };
    if result == input {
        return Some(tooltip);
    }
    let needs_late_fallback = result
        .canvas
        .as_ref()
        .is_some_and(|canvas| canvas.flow == Flow::Overlay);
    if let Some(canvas) = &result.canvas {
        if !validate_sprites(canvas, renderer) {
            return Some(tooltip);
        }
    }
    let mut components = if result.lines == input.lines {
        tooltip.components.clone()
    } else {
        let original_lines: Vec<_> = tooltip
            .components
            .iter()
            .filter_map(|component| match component {
                TooltipComponent::Text(line) => Some(line),
                _ => None,
            })
            .collect();
        let mut components: Vec<_> = result
            .lines
            .into_iter()
            .enumerate()
            .map(|(index, line)| {
                if input.lines.get(index) == Some(&line) {
                    return TooltipComponent::Text(original_lines[index].clone());
                }
                TooltipComponent::Text(
                    TooltipLine::new(
                        crate::ui::chat::chat_component::plain_chat_text(&line.text),
                        edit_box_argb_to_rgba(line.color as i32),
                    )
                    .with_styled_text(line.text)
                    .with_color_override(),
                )
            })
            .collect();
        let mut image_index = usize::from(!components.is_empty());
        for image in tooltip
            .components
            .iter()
            .filter(|component| matches!(component, TooltipComponent::Image(_)))
        {
            components.insert(image_index, image.clone());
            image_index += 1;
        }
        components
    };
    if !result.native_images {
        components.retain(|component| !matches!(component, TooltipComponent::Image(_)));
    }
    if let Some(canvas) = result.canvas {
        let mut textures = BTreeMap::new();
        for widget in &canvas.widgets {
            if let Widget::Image { texture, .. } = widget {
                let Ok(id) = texture.parse::<el_mod_content::ContentId>() else {
                    return Some(tooltip);
                };
                if crate::modding::client_content::require_owned_image(&canvas.owner, &id).is_err()
                {
                    return Some(tooltip);
                }
                let location = ResourceLocation::new(id.namespace(), id.path());
                let index = renderer
                    .get_or_load_custom_texture(device, queue, layout, resources, &location);
                if !renderer.custom_texture_registry.contains_key(&location) {
                    return Some(tooltip);
                }
                textures.insert(texture.clone(), index);
            }
        }
        components.insert(
            usize::from(!components.is_empty()),
            TooltipComponent::Image(TooltipImage::Extension(Presentation {
                canvas,
                items,
                textures,
            })),
        );
    }
    (!components.is_empty()).then_some(DeferredTooltip {
        components,
        style: tooltip.style.clone(),
        fallback: needs_late_fallback.then(|| Box::new(tooltip)),
    })
}
