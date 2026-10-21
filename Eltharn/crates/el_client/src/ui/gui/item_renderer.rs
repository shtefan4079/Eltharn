use crate::ui::gui::font::FontRenderer;
use crate::ui::gui::renderer::GuiRenderer;
use el_core::item::ItemStack;
use el_gui::framework::render_state::{
    GuiElementRenderState, GuiItemRenderState, RenderPipeline, TextureSetup, element_cmp,
};
use el_math::Matrix3x2f;
pub const GUI_ITEM_TINT: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
pub fn flow_item_atlas_marker() -> TextureSetup {
    TextureSetup::single_texture(el_core::resource_location::ResourceLocation::from_string(
        "eltharn:gui/item_atlas",
    ))
}
pub fn draw_gui_item(
    renderer: &mut GuiRenderer,
    font: &FontRenderer,
    item: &ItemStack,
    x: f32,
    y: f32,
    cooldown_percent: f32,
    decorations: bool,
) {
    if item.is_empty() {
        return;
    }
    let item_path = item.item.location.path.as_str();
    let fallback_sprite_id = el_core::resource_location::ResourceLocation::new(
        &item.item.location.namespace,
        &format!("item/{}", item_path),
    );
    let block_sprite_id = el_core::resource_location::ResourceLocation::new(
        &item.item.location.namespace,
        &format!("block/{}", item_path),
    );
    let tint_color = GUI_ITEM_TINT;
    renderer.set_item_model_cooldown(cooldown_percent);
    let foil_marker = renderer.begin_item_foil(item, x, y);
    let rendered = renderer.draw_player_head_item(x, y, item, tint_color)
        || renderer.draw_dynamic_item(x, y, item, tint_color)
        || renderer.draw_special_item_model(x, y, &item.item.location, tint_color)
        || renderer.draw_flat_item_model(x, y, item, tint_color)
        || renderer.draw_item_sprite(x, y, 16.0, 16.0, &fallback_sprite_id, tint_color)
        || renderer.draw_block_item_model(x, y, &item.item.location, tint_color)
        || (crate::ui::gui::is_flat_block(item_path)
            && renderer.draw_item_sprite(x, y, 16.0, 16.0, &block_sprite_id, tint_color))
        || renderer.draw_block_as_isometric(x, y, item_path, tint_color)
        || renderer.draw_item_sprite(x, y, 16.0, 16.0, &block_sprite_id, tint_color);
    if !rendered {
        renderer.draw_missing_item(x, y);
    }
    renderer.finish_item_foil(foil_marker);
    renderer.set_item_model_cooldown(0.0);
    if decorations {
        draw_gui_item_decorations(renderer, font, item, x, y, cooldown_percent);
    }
}
pub fn draw_gui_item_decorations(
    renderer: &mut GuiRenderer,
    font: &FontRenderer,
    item: &ItemStack,
    x: f32,
    y: f32,
    cooldown_percent: f32,
) {
    draw_gui_item_bar_and_cooldown(renderer, item, x, y, cooldown_percent);
    draw_gui_item_count(font, renderer, item, x, y, None, [1.0; 4]);
}
pub fn draw_gui_item_decorations_with_count_text(
    renderer: &mut GuiRenderer,
    font: &FontRenderer,
    item: &ItemStack,
    x: f32,
    y: f32,
    cooldown_percent: f32,
    count_text: &str,
    count_color: [f32; 4],
) {
    draw_gui_item_bar_and_cooldown(renderer, item, x, y, cooldown_percent);
    draw_gui_item_count(
        font,
        renderer,
        item,
        x,
        y,
        Some(count_text.to_string()),
        count_color,
    );
}
fn draw_gui_item_bar_and_cooldown(
    renderer: &mut GuiRenderer,
    item: &ItemStack,
    x: f32,
    y: f32,
    cooldown_percent: f32,
) {
    if item.is_damageable_item() && item.is_damaged() {
        let max_damage = item.get_max_damage().max(1);
        let damage = item.get_damage_value();
        let bar_width = java_bar_width(damage, max_damage);
        renderer.draw_rect(x + 2.0, y + 13.0, 13.0, 2.0, [0.0, 0.0, 0.0, 1.0]);
        let remaining = (max_damage - damage).max(0) as f32 / max_damage as f32;
        let bar_color = java_durability_bar_color(remaining);
        if bar_width > 0.0 {
            renderer.draw_rect(x + 2.0, y + 13.0, bar_width, 1.0, bar_color);
        }
    }
    if cooldown_percent > 0.0 {
        let top = y + (16.0 * (1.0 - cooldown_percent)).floor();
        let bottom = top + (16.0 * cooldown_percent).ceil();
        renderer.draw_rect(x, top, 16.0, bottom - top, [1.0, 1.0, 1.0, 127.0 / 255.0]);
    }
}
fn draw_gui_item_count(
    font: &FontRenderer,
    renderer: &mut GuiRenderer,
    item: &ItemStack,
    x: f32,
    y: f32,
    count_text: Option<String>,
    color: [f32; 4],
) {
    if !(count_text.is_some() || item.count != 1) {
        return;
    }
    let count_str = count_text.unwrap_or_else(|| item.count.to_string());
    let (count_x, count_y) = java_item_count_origin(font.measure_width(&count_str, 1.0), x, y);
    renderer.draw_text_with_shadow(font, &count_str, count_x, count_y, color, 1.0);
    trace_item_quantity(font, item, &count_str);
}
fn trace_item_quantity(font: &FontRenderer, item: &ItemStack, count_text: &str) {
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Mutex, OnceLock};
    const LIMIT: usize = 32;
    static EMITTED: AtomicUsize = AtomicUsize::new(0);
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    if !el_protocol_backend::diagnostics::file_enabled() || EMITTED.load(Ordering::Relaxed) >= LIMIT
    {
        return;
    }
    let glyphs: Vec<_> = count_text
        .chars()
        .map(|ch| {
            let glyph = font.glyph(ch);
            (
                ch,
                glyph.texture_index,
                glyph.width,
                glyph.height,
                glyph.advance,
                font.glyphs.contains_key(&ch),
                glyph.width > 0.0
                    && glyph.height > 0.0
                    && glyph.texture_index < font.bind_groups.len(),
            )
        })
        .collect();
    let line = format!(
        "[ItemQuantity] stage=draw item={} count={} decorators=true font_sheets={} digit_glyphs={glyphs:?}",
        item.item.location,
        item.count,
        font.bind_groups.len(),
    );
    let Ok(mut seen) = SEEN.get_or_init(|| Mutex::new(HashSet::new())).lock() else {
        return;
    };
    if seen.len() >= LIMIT || !seen.insert(line.clone()) {
        return;
    }
    EMITTED.store(seen.len(), Ordering::Relaxed);
    drop(seen);
    el_protocol_backend::diagnostics::write_line(&line);
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GuiFlowItemOp<'a> {
    Element(&'a GuiElementRenderState),
    Item(usize),
}
pub fn merged_item_element_ops<'a>(
    elements: &'a [GuiElementRenderState],
    items: &'a [GuiItemRenderState],
) -> Vec<GuiFlowItemOp<'a>> {
    if items.is_empty() {
        return elements.iter().map(GuiFlowItemOp::Element).collect();
    }
    let item_probes: Vec<GuiElementRenderState> = items
        .iter()
        .map(|item| GuiElementRenderState::Blit {
            pipeline: RenderPipeline::GuiTexturedPremultipliedAlpha,
            texture_setup: flow_item_atlas_marker(),
            pose: Matrix3x2f::new(),
            x0: item.x,
            y0: item.y,
            x1: item.x + 16,
            y1: item.y + 16,
            u0: 0.0,
            u1: 1.0,
            v0: 0.0,
            v1: 1.0,
            color: -1,
            scissor: item.scissor.clone(),
            bounds: item.bounds.clone(),
        })
        .collect();
    let mut ops: Vec<GuiFlowItemOp> = elements
        .iter()
        .map(GuiFlowItemOp::Element)
        .chain((0..items.len()).into_iter().map(GuiFlowItemOp::Item))
        .collect();
    ops.sort_by(|a, b| match (a, b) {
        (GuiFlowItemOp::Element(x), GuiFlowItemOp::Element(y)) => element_cmp(x, y),
        (GuiFlowItemOp::Element(x), GuiFlowItemOp::Item(probe)) => {
            element_cmp(x, &item_probes[*probe])
        }
        (GuiFlowItemOp::Item(probe), GuiFlowItemOp::Element(x)) => {
            element_cmp(&item_probes[*probe], x)
        }
        (GuiFlowItemOp::Item(i), GuiFlowItemOp::Item(j)) => {
            element_cmp(&item_probes[*i], &item_probes[*j])
        }
    });
    ops
}
fn java_item_count_origin(measured_width: f32, x: f32, y: f32) -> (f32, f32) {
    (x + 17.0 - measured_width.ceil(), y + 16.0)
}
fn java_durability_bar_color(remaining: f32) -> [f32; 4] {
    let hue = (remaining.clamp(0.0, 1.0) / 3.0) * 6.0;
    let sector = hue.floor() as i32;
    let fraction = hue - sector as f32;
    let (r, g, b) = match sector.rem_euclid(6) {
        0 => (1.0, fraction, 0.0),
        1 => (1.0 - fraction, 1.0, 0.0),
        2 => (0.0, 1.0, fraction),
        3 => (0.0, 1.0 - fraction, 1.0),
        4 => (fraction, 0.0, 1.0),
        _ => (1.0, 0.0, 1.0 - fraction),
    };
    [r, g, b, 1.0]
}
fn java_bar_width(damage: i32, max_damage: i32) -> f32 {
    (13.0 - damage as f32 * 13.0 / max_damage as f32)
        .round()
        .clamp(0.0, 13.0)
}
