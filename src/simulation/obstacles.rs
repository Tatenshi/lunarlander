use super::{
    entity::Entity,
    objectstore::{ObjectDefault, ObjectStore},
};

use std::collections::HashMap;

use rand::{thread_rng, Rng};
use sdl2::render::Texture;

use crate::{
    draw,
    graphics::{self, ISALAND, RECT_ENEMY_COLOR},
    vecmath::{self, TransformationMatrix, Vec2d},
};

#[derive(Clone, Copy, PartialEq)]
pub struct Obstacle<'a> {
    pub entity_id: usize,
    pub obstacle_type: ObstacleType,
    pub hull: &'a [Vec2d],
}

impl ObjectDefault for Obstacle<'_> {
    fn default() -> Self {
        Obstacle {
            entity_id: 0,
            obstacle_type: ObstacleType::Island,
            hull: &[],
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ObstacleType {
    Island,
}

impl Obstacle<'_> {
    pub fn render(
        &self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        screen_space_transform: TransformationMatrix,
        textures: &HashMap<String, Texture<'_>>,
        source_entity: &Entity,
        screen_scale: f32,
    ) {
        let items: &[Vec2d];
        let col;
        match self.obstacle_type {
            ObstacleType::Island => {
                items = &ISALAND;
                col = RECT_ENEMY_COLOR;
            }
        }
        let scale = vecmath::TransformationMatrix::scale(
            graphics::ENTITY_SCALE.x,
            graphics::ENTITY_SCALE.y,
        );
        let entity_trans =
            source_entity.get_screenspace_transform(screen_space_transform, screen_scale) * scale;
        let texture = textures.get("neon").unwrap();
        let geometry = entity_trans.transform_many(&items.to_vec());
        draw::neon_draw_lines(canvas, &geometry, col, true, texture).unwrap();
    }
}
