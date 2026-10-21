#![allow(dead_code)]
use crate::render::baked_model::Vertex;
use el_math::random::{RandomSource, SingleThreadedRandomSource};
use glam::Vec3;
pub const RAIN_TEXTURE_LOCATION: &str = "minecraft:environment/rain";
pub const SNOW_TEXTURE_LOCATION: &str = "minecraft:environment/snow";
const RAIN_TABLE_SIZE: i32 = 32;
const HALF_RAIN_TABLE_SIZE: i32 = 16;
const INDICES_PER_COLUMN: u32 = 6;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WeatherPrecipitation {
    #[default]
    None,
    Rain,
    Snow,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeatherColumnInstance {
    pub x: i32,
    pub z: i32,
    pub bottom_y: i32,
    pub top_y: i32,
    pub u_offset: f32,
    pub v_offset: f32,
    pub light_coords: u32,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WeatherRenderState {
    pub rain_columns: Vec<WeatherColumnInstance>,
    pub snow_columns: Vec<WeatherColumnInstance>,
    pub intensity: f32,
    pub radius: i32,
}
impl WeatherRenderState {
    pub fn reset(&mut self) {
        self.rain_columns.clear();
        self.snow_columns.clear();
        self.intensity = 0.0;
        self.radius = 0;
    }
}
pub trait WeatherColumnSampler {
    fn motion_blocking_height(&self, x: i32, z: i32) -> Option<i32>;
    fn precipitation_at(&self, x: i32, y: i32, z: i32) -> WeatherPrecipitation;
    fn light_levels_at(&self, x: i32, y: i32, z: i32) -> [u8; 2];
}
#[derive(Clone, Debug)]
pub struct WeatherEffectRenderer {
    column_size_x: [f32; (RAIN_TABLE_SIZE * RAIN_TABLE_SIZE) as usize],
    column_size_z: [f32; (RAIN_TABLE_SIZE * RAIN_TABLE_SIZE) as usize],
}
impl Default for WeatherEffectRenderer {
    fn default() -> Self {
        Self::new()
    }
}
impl WeatherEffectRenderer {
    pub fn new() -> Self {
        let mut column_size_x = [0.0; (RAIN_TABLE_SIZE * RAIN_TABLE_SIZE) as usize];
        let mut column_size_z = [0.0; (RAIN_TABLE_SIZE * RAIN_TABLE_SIZE) as usize];
        for z in 0..RAIN_TABLE_SIZE {
            for x in 0..RAIN_TABLE_SIZE {
                let delta_x = (x - HALF_RAIN_TABLE_SIZE) as f32;
                let delta_z = (z - HALF_RAIN_TABLE_SIZE) as f32;
                let distance = (delta_x * delta_x + delta_z * delta_z).sqrt();
                let index = (z * RAIN_TABLE_SIZE + x) as usize;
                column_size_x[index] = -delta_z / distance;
                column_size_z[index] = delta_x / distance;
            }
        }
        Self {
            column_size_x,
            column_size_z,
        }
    }
    pub fn extract_render_state(
        &self,
        sampler: &impl WeatherColumnSampler,
        game_time: i64,
        partial_ticks: f32,
        camera_pos: Vec3,
        intensity: f32,
        radius: i32,
        render_state: &mut WeatherRenderState,
    ) {
        render_state.intensity = intensity;
        if intensity <= 0.0 {
            return;
        }
        render_state.radius = radius;
        let camera_block_x = camera_pos.x.floor() as i32;
        let camera_block_y = camera_pos.y.floor() as i32;
        let camera_block_z = camera_pos.z.floor() as i32;
        let mut random = SingleThreadedRandomSource::new(0);
        for z in (camera_block_z - radius)..=(camera_block_z + radius) {
            for x in (camera_block_x - radius)..=(camera_block_x + radius) {
                let Some(terrain_height) = sampler.motion_blocking_height(x, z) else {
                    continue;
                };
                let bottom_y = (camera_block_y - radius).max(terrain_height);
                let top_y = (camera_block_y + radius).max(terrain_height);
                if top_y - bottom_y == 0 {
                    continue;
                }
                let precipitation = sampler.precipitation_at(x, camera_block_y, z);
                if precipitation == WeatherPrecipitation::None {
                    continue;
                }
                let seed = weather_column_seed(x, z);
                random.set_seed(i64::from(seed));
                let light_sample_y = camera_block_y.max(terrain_height);
                let [block_light, sky_light] = sampler.light_levels_at(x, light_sample_y, z);
                let light_coords = pack_light_coords(block_light, sky_light);
                match precipitation {
                    WeatherPrecipitation::Rain => {
                        render_state.rain_columns.push(create_rain_column_instance(
                            &mut random,
                            game_time,
                            x,
                            bottom_y,
                            top_y,
                            z,
                            light_coords,
                            partial_ticks,
                        ))
                    }
                    WeatherPrecipitation::Snow => {
                        render_state.snow_columns.push(create_snow_column_instance(
                            &mut random,
                            game_time,
                            x,
                            bottom_y,
                            top_y,
                            z,
                            light_coords,
                            partial_ticks,
                        ))
                    }
                    WeatherPrecipitation::None => unreachable!("filtered above"),
                }
            }
        }
    }
    pub fn build_mesh(&self, camera_pos: Vec3, render_state: &WeatherRenderState) -> WeatherMesh {
        let column_count = render_state.rain_columns.len() + render_state.snow_columns.len();
        let mut mesh = WeatherMesh::default();
        mesh.vertices.reserve(column_count * 4);
        mesh.indices
            .reserve(column_count * INDICES_PER_COLUMN as usize);
        self.build_mesh_into(camera_pos, render_state, &mut mesh);
        mesh
    }
    pub fn build_mesh_into(
        &self,
        camera_pos: Vec3,
        render_state: &WeatherRenderState,
        mesh: &mut WeatherMesh,
    ) {
        let column_count = render_state.rain_columns.len() + render_state.snow_columns.len();
        mesh.vertices.clear();
        mesh.indices.clear();
        mesh.vertices.reserve(column_count.saturating_mul(4));
        mesh.indices
            .reserve(column_count.saturating_mul(INDICES_PER_COLUMN as usize));
        mesh.rain_index_count = 0;
        mesh.snow_index_count = 0;
        self.append_instances(
            mesh,
            &render_state.rain_columns,
            camera_pos,
            1.0,
            render_state.radius,
            render_state.intensity,
        );
        mesh.rain_index_count = mesh.indices.len() as u32;
        self.append_instances(
            mesh,
            &render_state.snow_columns,
            camera_pos,
            0.8,
            render_state.radius,
            render_state.intensity,
        );
        mesh.snow_index_count = mesh.indices.len() as u32 - mesh.rain_index_count;
    }
    fn append_instances(
        &self,
        mesh: &mut WeatherMesh,
        columns: &[WeatherColumnInstance],
        camera_pos: Vec3,
        max_alpha: f32,
        radius: i32,
        intensity: f32,
    ) {
        if columns.is_empty() {
            return;
        }
        let radius_sq = (radius * radius) as f32;
        let camera_floor_x = camera_pos.x.floor() as i32;
        let camera_floor_z = camera_pos.z.floor() as i32;
        for column in columns {
            let relative_x = column.x as f32 + 0.5 - camera_pos.x;
            let relative_z = column.z as f32 + 0.5 - camera_pos.z;
            let distance_sq = relative_x * relative_x + relative_z * relative_z;
            let alpha = lerp((distance_sq / radius_sq).min(1.0), max_alpha, 0.5) * intensity;
            let index = (column.z - camera_floor_z + HALF_RAIN_TABLE_SIZE) * RAIN_TABLE_SIZE
                + column.x
                - camera_floor_x
                + HALF_RAIN_TABLE_SIZE;
            let half_size_x = self.column_size_x[index as usize] / 2.0;
            let half_size_z = self.column_size_z[index as usize] / 2.0;
            let x0 = relative_x - half_size_x;
            let x1 = relative_x + half_size_x;
            let y1 = column.top_y as f32 - camera_pos.y;
            let y0 = column.bottom_y as f32 - camera_pos.y;
            let z0 = relative_z - half_size_z;
            let z1 = relative_z + half_size_z;
            let u0 = column.u_offset;
            let u1 = column.u_offset + 1.0;
            let v0 = column.bottom_y as f32 * 0.25 + column.v_offset;
            let v1 = column.top_y as f32 * 0.25 + column.v_offset;
            let color = [1.0, 1.0, 1.0, alpha];
            let light = light_coords_to_uv(column.light_coords);
            mesh.vertices.extend([
                Vertex {
                    position: [x0, y1, z0],
                    tex_coords: [u0, v0],
                    normal: [0.0; 3],
                    color,
                    light,
                },
                Vertex {
                    position: [x1, y1, z1],
                    tex_coords: [u1, v0],
                    normal: [0.0; 3],
                    color,
                    light,
                },
                Vertex {
                    position: [x1, y0, z1],
                    tex_coords: [u1, v1],
                    normal: [0.0; 3],
                    color,
                    light,
                },
                Vertex {
                    position: [x0, y0, z0],
                    tex_coords: [u0, v1],
                    normal: [0.0; 3],
                    color,
                    light,
                },
            ]);
            let first_vertex = mesh.vertices.len() as u32 - 4;
            mesh.indices.extend([
                first_vertex,
                first_vertex + 1,
                first_vertex + 2,
                first_vertex + 2,
                first_vertex + 3,
                first_vertex,
            ]);
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct WeatherMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub rain_index_count: u32,
    pub snow_index_count: u32,
}
impl WeatherMesh {
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
    pub fn snow_index_start(&self) -> u32 {
        self.rain_index_count
    }
}
pub const fn pack_light_coords(block: u8, sky: u8) -> u32 {
    ((block as u32) << 4) | ((sky as u32) << 20)
}
pub const fn block_light(light_coords: u32) -> u8 {
    ((light_coords >> 4) & 15) as u8
}
pub const fn sky_light(light_coords: u32) -> u8 {
    ((light_coords >> 20) & 15) as u8
}
fn light_coords_to_uv(light_coords: u32) -> [f32; 2] {
    [
        block_light(light_coords) as f32 / 16.0,
        sky_light(light_coords) as f32 / 16.0,
    ]
}
fn weather_column_seed(x: i32, z: i32) -> i32 {
    let x_part = x
        .wrapping_mul(x)
        .wrapping_mul(3121)
        .wrapping_add(x.wrapping_mul(45_238_971));
    let z_part = z
        .wrapping_mul(z)
        .wrapping_mul(418_711)
        .wrapping_add(z.wrapping_mul(13_761));
    x_part ^ z_part
}
fn rain_tick_offset(x: i32, z: i32) -> i32 {
    x.wrapping_mul(x)
        .wrapping_mul(3121)
        .wrapping_add(x.wrapping_mul(45_238_971))
        .wrapping_add(z.wrapping_mul(z).wrapping_mul(418_711))
        .wrapping_add(z.wrapping_mul(13_761))
        & 0xff
}
fn create_rain_column_instance(
    random: &mut impl RandomSource,
    ticks: i64,
    x: i32,
    bottom_y: i32,
    top_y: i32,
    z: i32,
    light_coords: u32,
    partial_ticks: f32,
) -> WeatherColumnInstance {
    let wrapped_ticks = (ticks & 131_071) as i32;
    let tick_offset = rain_tick_offset(x, z);
    let block_pos_rain_speed = 3.0 + random.next_float();
    let texture_offset = -((wrapped_ticks as f32 + tick_offset as f32 + partial_ticks) / 32.0)
        * block_pos_rain_speed;
    let wrapped_texture_offset = texture_offset % 32.0;
    WeatherColumnInstance {
        x,
        z,
        bottom_y,
        top_y,
        u_offset: 0.0,
        v_offset: wrapped_texture_offset,
        light_coords,
    }
}
fn create_snow_column_instance(
    random: &mut impl RandomSource,
    ticks: i64,
    x: i32,
    bottom_y: i32,
    top_y: i32,
    z: i32,
    light_coords: u32,
    partial_ticks: f32,
) -> WeatherColumnInstance {
    let wrapped_ticks = (ticks & 131_071) as i32;
    let time = wrapped_ticks as f32 + partial_ticks;
    let u = (random.next_double() + (time * 0.01 * random.next_gaussian() as f32) as f64) as f32;
    let v = (random.next_double() + (time * random.next_gaussian() as f32 * 0.001) as f64) as f32;
    let v_offset = -(((ticks & 511) as f32 + partial_ticks) / 512.0);
    let brightened_light_coords = pack_light_coords(
        (u16::from(block_light(light_coords)) * 3 + 15) as u8 / 4,
        (u16::from(sky_light(light_coords)) * 3 + 15) as u8 / 4,
    );
    WeatherColumnInstance {
        x,
        z,
        bottom_y,
        top_y,
        u_offset: u,
        v_offset: v_offset + v,
        light_coords: brightened_light_coords,
    }
}
fn lerp(delta: f32, start: f32, end: f32) -> f32 {
    start + delta * (end - start)
}
