use std::collections::HashMap;

use rand::{thread_rng, Rng};
use sdl2::render::Texture;

use crate::{
    draw,
    graphics::{
        self, BLACK_HOLE_ENEMY, MINIRECT_ENEMY, MINIRECT_ENEMY_COLOR, RECT_ENEMY, RECT_ENEMY_COLOR,
        ROMBUS_ENEMY, ROMBUS_ENEMY_COLOR, WANDERER_ENEMY, WANDERER_ENEMY_COLOR,
    },
    vecmath::{self, TransformationMatrix, Vec2d},
};

use super::{
    entity::Entity,
    objectstore::{ObjectDefault, ObjectStore},
    vertexgrid::{CircularEffectType, VertexGrid},
    Missile,
};

#[derive(Clone, Copy, PartialEq)]
pub enum EnemyType {
    Rect,
    Rombus,
    Wanderer,
    SpawningRect,
    MiniRect,
    BlackHole,
    Invalid,
}

#[derive(Clone)]
pub struct Enemy<'a> {
    pub entity_id: usize,
    pub ty: EnemyType,
    pub hull: &'a [Vec2d],
    pub num_ticks: u32,
    pub hitpoints: u32,
}

impl ObjectDefault for Enemy<'_> {
    fn default() -> Self {
        Enemy {
            entity_id: 0,
            ty: EnemyType::Invalid,
            hull: &[],
            num_ticks: 0,
            hitpoints: 1,
        }
    }
}

impl Enemy<'_> {
    pub fn get_score(&self) -> u32 {
        return match self.ty {
            EnemyType::Invalid => 0,
            EnemyType::Rombus => 100,
            EnemyType::Rect => 300,
            EnemyType::Wanderer => 200,
            EnemyType::SpawningRect => 200,
            EnemyType::MiniRect => 50,
            EnemyType::BlackHole => 500,
        };
    }

    pub fn tick(
        &mut self,
        entities: &ObjectStore<Entity>,
        player_id: usize,
        missiles: &ObjectStore<Missile>,
        grid: &mut super::vertexgrid::VertexGrid,
    ) {
        let ty = self.ty;
        let playerpos = entities.get_object(player_id).position().clone();
        self.num_ticks += 1;

        match ty {
            EnemyType::Rect => self.rect_tick(entities, playerpos, missiles),
            EnemyType::Rombus => self.rombus_tick(entities, playerpos),
            EnemyType::Wanderer => self.wanderer_tick(entities),
            EnemyType::SpawningRect => self.rect_tick(entities, playerpos, missiles),
            EnemyType::MiniRect => self.rect_tick(entities, playerpos, missiles),
            EnemyType::BlackHole => self.black_hole_tick(entities, grid),
            EnemyType::Invalid => todo!(),
        }
    }

    pub fn render(
        &self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        screen_space_transform: TransformationMatrix,
        textures: &HashMap<String, Texture<'_>>,
        source_entity: &Entity,
    ) {
        let items: &[Vec2d];
        let col;
        match self.ty {
            EnemyType::Rect => {
                items = &RECT_ENEMY;
                col = RECT_ENEMY_COLOR;
            }
            EnemyType::Rombus => {
                items = &ROMBUS_ENEMY;
                col = ROMBUS_ENEMY_COLOR;
            }
            EnemyType::Wanderer => {
                items = &WANDERER_ENEMY;
                col = WANDERER_ENEMY_COLOR
            }
            EnemyType::SpawningRect => {
                items = &RECT_ENEMY;
                col = MINIRECT_ENEMY_COLOR;
            }
            EnemyType::MiniRect => {
                items = &MINIRECT_ENEMY;
                col = MINIRECT_ENEMY_COLOR;
            }
            EnemyType::BlackHole => {
                items = &BLACK_HOLE_ENEMY;
                col = MINIRECT_ENEMY_COLOR;
            }
            EnemyType::Invalid => todo!(),
        }
        let scale = vecmath::TransformationMatrix::scale(
            graphics::ENTITY_SCALE.x,
            graphics::ENTITY_SCALE.y,
        );
        let entity_trans = source_entity.get_screenspace_transform(screen_space_transform) * scale;
        let texture = textures.get("neon").unwrap();
        let geometry = entity_trans.transform_many(&items.to_vec());
        draw::neon_draw_lines(canvas, &geometry, col, true, texture).unwrap();
    }

    fn rombus_tick(&self, world: &ObjectStore<Entity>, player_pos: Vec2d) {
        world.with(self.entity_id, |ent| {
            const ROMBUS_VEL: f32 = 120.0f32;
            let new_dir = (player_pos - ent.position()).normalized() * ROMBUS_VEL;
            ent.set_acceleration(new_dir);
            ent.set_direction(new_dir);
            ent.set_max_velocity(ROMBUS_VEL);
        });
    }

    fn rect_tick(
        &self,
        world: &ObjectStore<Entity>,
        player_pos: Vec2d,
        missiles: &ObjectStore<Missile>,
    ) {
        // for every 1000 sicks, minirects will get 5% faster:
        let vel_factor = (self.num_ticks / 1000) as f32 * 0.15;

        let vel = match self.ty {
            EnemyType::Rect => 120f32,
            EnemyType::Rombus => todo!(),
            EnemyType::Wanderer => todo!(),
            EnemyType::SpawningRect => 100f32,
            EnemyType::MiniRect => 250f32 * (1.0f32 + vel_factor),
            EnemyType::BlackHole => todo!(),
            EnemyType::Invalid => todo!(),
        };
        let current_pos;
        let mut new_dir;
        {
            current_pos = world.get_object(self.entity_id).position();
            new_dir = (player_pos - current_pos).normalized() * vel;
        }

        missiles.for_each(|missile, _| {
            let missile_ent = world.get_object(missile.entity_id);
            let missile_pos = missile_ent.position();
            // each missile will apply a force on the rect enemy, that
            // is inversely proportional to the distance
            let missile_dist = current_pos - missile_pos;
            const FORCE_RANGE: f32 = 128f32;
            let missile_dist_units = missile_dist.len();
            let relative_force_strength = 1.0f32 - (missile_dist_units / FORCE_RANGE);

            if relative_force_strength > 1.0f32 || relative_force_strength < 0.0f32 {
                return;
            }

            new_dir = new_dir + ((missile_dist.normalized()) * relative_force_strength * vel);
        });

        world.with(self.entity_id, |ent| {
            ent.set_acceleration(new_dir);
            ent.set_direction(new_dir);
            ent.set_max_velocity(vel);
        });
    }

    fn wanderer_tick(&self, world: &ObjectStore<Entity>) {
        world.with(self.entity_id, |ent| {
            const MAX_VEL: f32 = 80f32;

            //also, increas velocity by 20 % every 500 ticks
            let vel_factor = 1.0f32 + (self.num_ticks / 500) as f32 * 0.2;

            // the wanderer changes direction every 500 ticks: (use mod 1 to have it change direction right away at the first tick!)
            if self.num_ticks % 500 == 1 {
                let new_dir = Vec2d {
                    x: thread_rng().gen_range(-1.0..1.0) as f32,
                    y: thread_rng().gen_range(-1.0..1.0) as f32,
                };
                ent.set_direction(new_dir.normalized() * MAX_VEL * vel_factor);
                ent.set_acceleration(ent.direction() * MAX_VEL * vel_factor);
                ent.set_max_velocity(MAX_VEL * vel_factor);
            }
        });
    }

    fn black_hole_tick(&self, world: &ObjectStore<Entity>, grid: &mut VertexGrid) {
        // slightly pull all entities towards the black hole:
        let my_pos = world.get_object(self.entity_id).position();
        let num_growth_cycles = self.num_ticks as f32 / 500.0f32;
        // adjust pullstrength depending on age of black hole:
        let pull_factor = 1.0f32 + num_growth_cycles * 0.35;

        // have pull strenght falloff at distance:
        let mut falloff_range = 100f32 * num_growth_cycles;
        if falloff_range > 900f32 {
            falloff_range = 900f32;
        }

        grid.add_circular_effect(
            my_pos,
            falloff_range,
            0.25,
            0.1,
            CircularEffectType::Implosion,
        );

        world.with(self.entity_id, |ent| {
            ent.set_acceleration(Vec2d::new(0f32, 0f32));
            ent.set_direction(Vec2d::new(0f32, 0f32));
            ent.set_max_velocity(0.0f32);
        });

        world.for_each(|ent: &mut Entity, id: usize| {
            if id == self.entity_id {
                return;
            }
            let pos = ent.position();
            let dist = (pos - my_pos).len();
            let dir = (my_pos - pos).normalized() * 800f32;
            let mut dist_falloff = 1.0f32 - (dist / falloff_range);
            if dist_falloff < 0.0f32 {
                dist_falloff = 0.0f32;
            }

            let gravity = ent.gravity();
            ent.set_gravity(gravity + ((dir * pull_factor) * dist_falloff));
        });
    }
}
