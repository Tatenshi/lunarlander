use core::panic;

use sdl2::pixels::Color;

use crate::{
    draw,
    vecmath::{TransformationMatrix, Vec2d},
};

use super::{
    objectstore::{ObjectDefault, ObjectStore},
    vertex::Vertex,
    GRID_DISTANCE, WORLD_SIZE,
};
#[derive(Debug, Clone, Copy)]
pub enum CircularEffectType {
    Explosion,
    Implosion,
}

#[derive(Clone, Copy)]
struct CircularEffect {
    center: Vec2d,
    radius: f32,
    time_to_live: f32,
    expansion_speed: f32,
    effect_type: CircularEffectType,
}

#[derive(Clone, Copy)]
enum Effect {
    Circular(CircularEffect),
}

impl ObjectDefault for Effect {
    fn default() -> Self {
        Effect::Circular(CircularEffect {
            center: Vec2d::default(),
            radius: 0.0,
            time_to_live: 0.0,
            expansion_speed: 0.0,
            effect_type: CircularEffectType::Explosion,
        })
    }
}

pub struct VertexGrid {
    grid: Vec<Vertex>,
    effects: ObjectStore<Effect>,
}

impl VertexGrid {
    pub fn new() -> Self {
        let mut grid: Vec<Vertex> = Vec::new();

        let mut y = 0.0;
        while y < WORLD_SIZE.y + 1.0 {
            let mut x = 0.0;
            while x < WORLD_SIZE.x + 1.0 {
                let pos: Vec2d = Vec2d::new(x, y);
                let vertex: Vertex = Vertex::new(pos);
                grid.push(vertex);
                x += GRID_DISTANCE;
            }
            y += GRID_DISTANCE;
        }

        Self {
            grid,
            effects: ObjectStore::new(),
        }
    }

    pub fn add_circular_effect(
        &mut self,
        center: Vec2d,
        radius: f32,
        time_to_live: f32,
        expansion_speed: f32,
        effect_type: CircularEffectType,
    ) {
        self.effects.insert_object(Effect::Circular(CircularEffect {
            center,
            radius,
            time_to_live,
            expansion_speed,
            effect_type,
        }));
    }

    pub fn intersect_grid(&mut self, missile_pos: Vec2d) {
        let x = missile_pos.x;
        let y = missile_pos.y;

        if x > WORLD_SIZE.x || x < 0.0 || y < 0.0 || y > WORLD_SIZE.y {
            return;
        }

        let num_coll = (WORLD_SIZE.x / GRID_DISTANCE + 1.0) as usize;
        let w_off = (x / GRID_DISTANCE) as usize;
        let y_off = (y / GRID_DISTANCE) as usize;
        let index = y_off * num_coll + w_off;

        let mut indices: Vec<usize> = Vec::new();
        indices.push(index);
        if index > num_coll {
            indices.push(index - num_coll);
        }
        if index + num_coll < self.grid.len() {
            indices.push(index + num_coll);
        }
        if index % num_coll != 0 {
            indices.push(index + 1);
            indices.push(index - 1);
        }

        for &i in indices.iter() {
            let dir = missile_pos - self.grid[i].position();
            if dir.len() > 0.01 {
                self.grid[i].add_to_dir(dir.normalized() * 0.06);
            }
        }
    }

    pub fn apply_force(&mut self, force: Vec2d, pos: Vec2d, delta_t: f32) {
        let x = ((pos.x as i32) / GRID_DISTANCE as i32) as i32;
        let y = ((pos.y as i32) / GRID_DISTANCE as i32) as i32;

        if x > WORLD_SIZE.x as i32 || y > WORLD_SIZE.y as i32 || x < 0 || y < 0 {
            return;
        }

        let num_coll = (WORLD_SIZE.x / GRID_DISTANCE + 1.0) as i32;
        let index = (y * num_coll + x) as usize;
        // Stupid defensive programming here...
        if index < self.grid.len() {
            // clip max force to 500 units
            let current_dir = self.grid[index].direction();
            let next_dir = /*current_dir +*/ force * delta_t;
            // if next_dir.is_not_zero() {
            //     if next_dir.len() > 250.0 {
            //         next_dir = next_dir.normalized() * 250.0;
            //     }
            // }
            self.grid[index].add_to_dir(next_dir);
        }
    }

    pub fn tick(&mut self, time_in_ms: f32) {
        let delta_t = time_in_ms / 1000.0;
        let mut forces_to_apply: Vec<(Vec2d, Vec2d)> = Vec::new();
        self.effects
            .for_each(|effect: &mut Effect, _: usize| match effect {
                Effect::Circular(e) => circle_effect_tick(e, delta_t, &mut forces_to_apply),
            });

        self.effects.garbage_collect_filter(|x| match x {
            Effect::Circular(x) => x.time_to_live <= 0.0,
        });

        for (force, pos) in forces_to_apply {
            self.apply_force(force, pos, delta_t);
        }

        for elem in self.grid.iter_mut() {
            elem.mov();
            elem.set_dir_back(delta_t);
        }
    }

    pub fn render(
        &self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        screen_space_transform: TransformationMatrix,
    ) {
        let row_count: usize = ((WORLD_SIZE.x / GRID_DISTANCE) + 1.0) as usize;
        let col_count = ((WORLD_SIZE.y / GRID_DISTANCE) + 1.0) as usize;

        //draw horizontal
        let mut current_row: usize = 0;
        let mut j: usize = 0;
        while j < col_count {
            let mut i: usize = 0;
            while i < (row_count - 1) {
                let p1 = screen_space_transform
                    .transform(&self.grid[i + (current_row * row_count)].position());
                let p2: Vec2d = screen_space_transform
                    .transform(&self.grid[i + 1 + (current_row * row_count)].position());
                let _ = draw::draw_line(canvas, &p1, &p2, Color::BLUE);
                i += 1;
            }
            j += 1;
            current_row += 1;
        }

        //draw vertical
        let mut current_col: usize = 0;
        while current_col < row_count {
            let mut i: usize = 0;
            while i < (col_count - 1) {
                let p1 = screen_space_transform
                    .transform(&self.grid[i * row_count + current_col].position());
                let p2: Vec2d = screen_space_transform
                    .transform(&self.grid[(i + 1) * row_count + current_col].position());
                let _ = draw::draw_line(canvas, &p1, &p2, Color::BLUE);
                i += 1;
            }
            current_col += 1;
        }
    }
}

fn circle_effect_tick(
    e: &mut CircularEffect,
    delta_t: f32,
    forces_to_apply: &mut Vec<(Vec2d, Vec2d)>,
) {
    e.time_to_live -= delta_t;
    if e.time_to_live > 0.0 {
        e.radius += e.expansion_speed * delta_t;

        // all vertices within the radius of the effect
        // are affected
        let start_pos = e.center - (Vec2d::new(e.radius, e.radius) * 0.5f32);
        let num_x_steps = (e.radius / GRID_DISTANCE) as usize;
        let num_y_steps = (e.radius / GRID_DISTANCE) as usize;

        // This is obviously not quite circular but rectangular. However in the final
        // effect that is not too noticeable
        for i in 0..=num_y_steps {
            let y = i as f32 * GRID_DISTANCE;
            let mut current_x = 0.0f32;
            for _ in 0..=num_x_steps {
                let current_pos = start_pos + Vec2d::new(current_x, y);
                current_x += GRID_DISTANCE as f32;
                let vec_to_center = e.center - current_pos;

                if vec_to_center.is_zero() {
                    continue;
                }
                if (vec_to_center).len() > e.radius * 0.5 {
                    continue;
                }

                // Calculate force to apply:
                // The force gets weaker as a function of the distance to the center
                // The force gets weaker, as the effect's ttl gets closer to 0

                let distance_to_center = vec_to_center.len();
                let fragment_of_radius = distance_to_center / (e.radius * 0.5);

                // have the strength decrease in a sinusoidal fashion in the last 25% of the range:
                let force_strength = if fragment_of_radius > 0.75 {
                    // this will produce a value in 0..1.0, where:
                    // 1.0 = vertex is at the outermost position
                    // 0.0 = vertex is at the 75% mark
                    let outer_fragment = (fragment_of_radius - 0.75) / 0.25;

                    // create sine shaped falloff: Falloff is zero at 0.75 and 1 at 1.0:
                    let falloff = ((1.0 - outer_fragment) * std::f32::consts::PI).sin();
                    falloff
                } else if fragment_of_radius < 0.25 {
                    // this will produce a value in 0..1.0, where:
                    // 0.0 = vertex is at the innermost position
                    // 1.0 = vertex is at the 25% mark
                    let inner_fragment = 1.0 - fragment_of_radius / 0.25;
                    // create sine shaped falloff: Falloff is zero at 0.75 and 1 at 1.0:
                    let falloff = ((inner_fragment) * std::f32::consts::PI).sin();
                    falloff
                } else {
                    1.0f32
                };

                // Adding the radius makes the effect too strong, so we scale it down a bit
                let mut output_force =
                    vec_to_center.normalized() * e.radius * 0.25 * force_strength * delta_t;

                match e.effect_type {
                    CircularEffectType::Explosion => {
                        output_force = output_force * -1.5;
                    }
                    CircularEffectType::Implosion => {}
                };

                forces_to_apply.push((output_force, current_pos));
            }
        }
    }
}
