use core::f32;
use std::collections::HashMap;
use std::f32::consts::PI;

use obstacles::Obstacle;
use rand::{thread_rng, Rng};
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::Texture;
use vertexgrid::CircularEffectType;

use crate::graphics::{
    self, render_game_over, BLACK_HOLE_ENEMY, CANNON_ENEMY, ENTITY_SCALE, MINIRECT_ENEMY, MISSILE,
    RECT_ENEMY, ROMBUS_ENEMY, STARSHIP, STARSHIP_COLOR,
};
use crate::sound;
use crate::vecmath::TransformationMatrix;
use crate::{
    collision, draw, hud,
    vecmath::{self, Vec2d},
};

use self::enemy::{Enemy, EnemyType};
use self::entity::Entity;
use self::explosion::Explosion;
use self::missile::Missile;

mod enemy;
mod entity;
mod explosion;
mod missile;
mod objectstore;
mod obstacles;
mod vertex;
mod vertexgrid;

use self::objectstore::{ObjectDefault, ObjectStore};
use self::vertexgrid::VertexGrid;

const MAX_ACCELERATION: f32 = 400.0;
const VELOCITY_SPACESHIP: f32 = 450.0;
const VELOCITY_MISSILE: f32 = 800.0;

const MAX_SHOOT_COOLDOWN: f32 = 0.15;
const MIN_SHOOT_COOLDOWN: f32 = 0.08;

const NUM_EXPLOSION_FARMES: u32 = 50;

#[derive(Clone, PartialEq, Debug)]
pub enum BorderBehavior {
    Dismiss,
    Bounce,
    BounceSlowdown,
}

#[derive(Clone, PartialEq, Debug)]
pub enum ActiveControleScheme {
    Keyboard,
    Gamepad,
}

pub const BIT_LEFT: u16 = 0b1;
pub const BIT_RIGHT: u16 = 0b10;
pub const BIT_UP: u16 = 0b100;
pub const BIT_DOWN: u16 = 0b1000;
pub const BIT_SHOOT_LEFT: u16 = 0b10000;
pub const BIT_SHOOT_RIGHT: u16 = 0b100000;
pub const BIT_SHOOT_UP: u16 = 0b1000000;
pub const BIT_SHOOT_DOWN: u16 = 0b10000000;
pub const MOVEMENT_MASK: u16 = BIT_LEFT | BIT_RIGHT | BIT_UP | BIT_DOWN;

pub struct Starship {
    entity_id: usize,
    drive_enabled: bool,
    shoot_direction: Vec2d,
    shoot_cooldown: f32,       // expected cooldown time (sec)
    shoot_cooldown_count: f32, // current cooldown value (sec)
}

impl ObjectDefault for Missile {
    fn default() -> Self {
        Missile {
            entity_id: 0,
            time_to_live: 5.0f32,
            hostile: false,
        }
    }
}

#[derive(Clone)]
pub struct FloatingText {
    position: Vec2d,
    time_to_live: f32, // in seconds
    text: String,
}
impl FloatingText {
    fn new(position: Vec2d, text: String) -> Self {
        Self {
            position: position,
            time_to_live: 1.0,
            text: text,
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum State {
    Running,
    Lost,
    WaitingForRespawn(f32),
}

pub struct World {
    game_control_bits: u16,
    entities: ObjectStore<Entity>,
    missiles: ObjectStore<Missile>,
    grid: VertexGrid,
    enemies: ObjectStore<Enemy<'static>>,
    obstacles: ObjectStore<Obstacle<'static>>,
    texts: Vec<FloatingText>,
    explosions: Vec<Explosion>,
    starship: Starship,
    hud: hud::Hud,
    game_state: State,
    score: u32,
    lifes: u32,
    multiplier: f32,
    kills_this_life: u32,
    screen_size: Vec2d,
    sound: sound::Sound,
    axis_l_x: i16,
    axis_l_y: i16,
    axis_r_x: i16,
    axis_r_y: i16,

    active_controle_scheme: ActiveControleScheme,
}

const WORLD_SIZE: Vec2d = Vec2d {
    x: 1.5 * 800.0,
    y: 1.5 * 600.0,
};

const SIDE_BORDER_LEFT: f32 = 0.0;

const SIDE_BORDER_RIGHT: f32 = WORLD_SIZE.x;

const ENTITY_TOP_BORDER: f32 = -WORLD_SIZE.y;

const PLAYER_TOP_BORDER: f32 = 0.0;

const BOTTOM_BORDER: f32 = WORLD_SIZE.y;

const GRID_DISTANCE: f32 = 20.0;

impl World {
    pub fn new(window_width: u32, window_height: u32) -> Self {
        let mut lander = Starship {
            entity_id: 0,
            drive_enabled: false,
            shoot_direction: Vec2d::default(),
            shoot_cooldown: MAX_SHOOT_COOLDOWN,
            shoot_cooldown_count: 0.0,
        };

        let store: ObjectStore<Entity> = ObjectStore::<Entity>::new();
        store.with_new(|player_entity, entity_index| {
            let start_position = Vec2d {
                x: WORLD_SIZE.x / 2.0,
                y: WORLD_SIZE.y - 100.0,
            };
            player_entity.set_position(start_position);
            player_entity.set_max_velocity(VELOCITY_SPACESHIP);
            player_entity.set_border_behavior(BorderBehavior::BounceSlowdown);
            lander.entity_id = entity_index;
        });

        let w = World {
            game_control_bits: 0,
            entities: store,
            starship: lander,
            enemies: ObjectStore::new(),
            obstacles: ObjectStore::new(),
            texts: Vec::new(),
            explosions: Vec::new(),
            hud: hud::Hud::new(),
            game_state: State::Running,
            missiles: ObjectStore::new(),
            grid: VertexGrid::new(),
            score: 0,
            lifes: 3,
            kills_this_life: 0,
            multiplier: 1.0,
            sound: sound::Sound::new(),
            screen_size: Vec2d {
                x: window_width as f32,
                y: window_height as f32,
            },
            axis_l_x: 0,
            axis_l_y: 0,
            axis_r_x: 0,
            axis_r_y: 0,

            active_controle_scheme: ActiveControleScheme::Keyboard,
        };

        w
    }

    pub fn create_missile(&mut self, pos: Vec2d, direction: Vec2d) {
        let id = self.entities.create_object();
        self.entities.with(id, |entity| {
            self.missiles
                .insert_object(Missile::new(id, entity, pos, direction, false));
        });
    }

    fn garbage_collect_entities(&mut self, ids_to_remove: &Vec<usize>) {
        self.entities.garbage_collect(ids_to_remove);
    }

    pub fn dismiss_dead_missiles(&mut self) {
        let entities_to_remove = self.missiles.filter_map(|x| {
            if x.time_to_live <= 0.0 {
                Some(x.entity_id)
            } else {
                None
            }
        });
        self.garbage_collect_entities(&entities_to_remove);
        self.missiles
            .garbage_collect_filter(|m| m.time_to_live <= 0.0);
    }

    fn texts_tick(&mut self, time_in_ms: f32) {
        let delta = time_in_ms / 1000.0f32;
        for txt in self.texts.iter_mut() {
            txt.time_to_live -= delta;
            txt.position = txt.position + (Vec2d { x: 0.0, y: -40.0 } * delta);
        }
        self.texts.retain(|t| t.time_to_live > 0.0);
    }

    fn explosion_tick(&mut self) {
        for exp in &mut self.explosions {
            exp.frame_count += 2;
        }
        self.explosions
            .retain(|e| e.frame_count < NUM_EXPLOSION_FARMES);
    }

    pub fn apply_control(&mut self) {
        if self.active_controle_scheme != ActiveControleScheme::Keyboard {
            return;
        }

        self.entities
            .with(self.starship.entity_id, |e: &mut Entity| {
                // shooting:
                self.starship.shoot_direction = Vec2d::default();
                let mut new_shoot_dir = Vec2d::default();
                if self.game_control_bits & BIT_SHOOT_LEFT != 0 {
                    new_shoot_dir = new_shoot_dir + Vec2d { x: -1.0, y: 0.0 };
                }

                if self.game_control_bits & BIT_SHOOT_RIGHT != 0 {
                    new_shoot_dir = new_shoot_dir + Vec2d { x: 1.0, y: 0.0 };
                }

                if self.game_control_bits & BIT_SHOOT_UP != 0 {
                    new_shoot_dir = new_shoot_dir + Vec2d { x: 0.0, y: -1.0 };
                }

                if self.game_control_bits & BIT_SHOOT_DOWN != 0 {
                    new_shoot_dir = new_shoot_dir + Vec2d { x: 0.0, y: 1.0 };
                }
                self.starship.shoot_direction = new_shoot_dir;

                // Movement:
                // Nothing set, break immediately!
                if self.game_control_bits & MOVEMENT_MASK == 0 {
                    e.set_direction(Vec2d::default());
                    e.set_acceleration(Vec2d::default());
                    self.starship.drive_enabled = false;
                    return;
                }

                let mut new_dir = Vec2d::default();
                if self.game_control_bits & BIT_LEFT != 0 {
                    new_dir = new_dir + Vec2d { x: -1.0, y: 0.0 };
                }

                if self.game_control_bits & BIT_RIGHT != 0 {
                    new_dir = new_dir + Vec2d { x: 1.0, y: 0.0 };
                }

                if self.game_control_bits & BIT_UP != 0 {
                    new_dir = new_dir + Vec2d { x: 0.0, y: -1.0 };
                }

                if self.game_control_bits & BIT_DOWN != 0 {
                    new_dir = new_dir + Vec2d { x: 0.0, y: 1.0 };
                }
                new_dir = new_dir * MAX_ACCELERATION;
                e.set_direction(e.direction() + new_dir);

                let dir_vec = e.direction();
                let accel_factor = dir_vec * MAX_ACCELERATION;
                e.set_acceleration(accel_factor);

                self.starship.drive_enabled = e.direction().is_not_zero();

                if accel_factor.len() > 0.0 {
                    let new_angle = accel_factor.angle_360();
                    // + pi because drawing is upside down
                    e.set_angle(new_angle + PI);
                } else {
                    // no direction do not change angle
                }
            })
    }

    pub fn tick(&mut self, time_in_ms: f32, tick_resolution_in_ms: f32) {
        let sim_time_in_seconds = time_in_ms / 1000.0;
        let mut num_ticks = (sim_time_in_seconds / tick_resolution_in_ms) as usize;
        if num_ticks == 0 {
            num_ticks = 1;
        }

        self.game_state = match self.game_state {
            State::Running => {
                self.apply_control();
                self.do_gameplay_ticks(sim_time_in_seconds, num_ticks, time_in_ms);
                self.game_state
            }
            State::WaitingForRespawn(time) => {
                if time <= 0.0 {
                    let id = self.starship.entity_id;
                    self.entities.with(id, |e: &mut Entity| {
                        e.set_position(Vec2d::new(WORLD_SIZE.x / 2.0, WORLD_SIZE.y / 2.0));
                        e.set_direction(Vec2d::default());
                        e.set_acceleration(Vec2d::default());
                        e.set_angle(0.0);
                    });
                    self.reset_control();
                    State::Running
                } else {
                    State::WaitingForRespawn(time - sim_time_in_seconds)
                }
            }
            _ => return,
        };

        self.texts_tick(time_in_ms);
        self.explosion_tick();
        self.grid.tick(time_in_ms);

        self.sound.play_background_music();
    }

    fn do_gameplay_ticks(&mut self, sim_time_in_seconds: f32, num_ticks: usize, time_in_ms: f32) {
        self.entities
            .for_each(|e: &mut Entity, _: usize| e.physics_tick(sim_time_in_seconds, num_ticks));

        self.entities
            .with(self.starship.entity_id, |e: &mut Entity| {
                self.grid.intersect_grid(e.position());
            });

        self.missile_tick(time_in_ms);
        self.dismiss_dead_missiles();
        self.enemy_tick();

        self.do_collision_detection();
    }

    pub(crate) fn render(
        &mut self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        textures: &HashMap<String, Texture>,
    ) {
        match self.game_state {
            State::Lost => render_game_over(canvas, self.screen_size / 2.0),
            State::Running => (),
            State::WaitingForRespawn(_) => {}
        }

        let starship_entity = self.entities.get_object(self.starship.entity_id);

        let mut screen_space_transform = TransformationMatrix::unit();
        screen_space_transform = screen_space_transform
            * TransformationMatrix::translation_v(starship_entity.position() * -1.0)
            * TransformationMatrix::translation_v(self.screen_size / 2.0); // center to screen

        self.render_grid(canvas, screen_space_transform);
        self.render_world_border(canvas, screen_space_transform);
        self.render_enemies(canvas, screen_space_transform, textures);
        self.render_explosions(canvas, screen_space_transform, textures);
        self.render_texts(canvas, screen_space_transform);

        if self.game_state == State::Running {
            self.render_starship(&starship_entity, screen_space_transform, canvas, textures);
        }

        self.render_missiles(screen_space_transform, canvas, textures);
        self.render_hud(canvas);
    }

    fn render_missiles(
        &mut self,
        screen_space_transform: TransformationMatrix,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        textures: &HashMap<String, Texture>,
    ) {
        self.missiles.for_each(|missile, _| {
            let entity = self.entities.get_object(missile.entity_id);
            let scale = vecmath::TransformationMatrix::scale(7f32, 7f32);
            let entity_trans = entity.get_screenspace_transform(screen_space_transform) * scale;
            let vecs = entity_trans.transform_many(&MISSILE.to_vec());
            let texture = textures.get("neon").unwrap();
            let color = if missile.hostile {
                Color::RGBA(255, 0, 0, 255)
            } else {
                Color::RGBA(255, 255, 255, 255)
            };
            let _ = draw::neon_draw_lines(canvas, &vecs, color, true, texture);
        });
    }

    pub(crate) fn update_window_size(&mut self, width: f32, height: f32) {
        self.screen_size.x = width;
        self.screen_size.y = height;
    }

    pub(crate) fn modify_control_bit(&mut self, dir: u16, enable: bool) {
        if self.game_state != State::Running {
            return;
        }

        self.active_controle_scheme = ActiveControleScheme::Keyboard;

        if enable {
            self.game_control_bits |= dir;
        } else {
            self.game_control_bits &= !dir;
        }
    }

    pub fn set_player_direction(&mut self, direction: Vec2d) {
        self.entities.with(self.starship.entity_id, |entity| {
            let last_len = if entity.direction().is_not_zero() {
                entity.direction().len()
            } else {
                1.0f32
            };
            entity.set_direction(direction * last_len);
            let dir_vec = entity.direction();
            let accel_factor = dir_vec * MAX_ACCELERATION;
            entity.set_acceleration(accel_factor);

            self.starship.drive_enabled = entity.direction().is_not_zero();

            if accel_factor.len() > 0.0 {
                let new_angle = accel_factor.angle_360();
                // + pi because drawing is upside down
                entity.set_angle(new_angle + PI);
            } else {
                // no direction do not change angle
            }
        })
    }

    fn missile_tick(&mut self, time_in_ms: f32) {
        let time_delta = time_in_ms / 1000.0f32;
        if self.starship.shoot_cooldown_count > 0.0 {
            self.starship.shoot_cooldown_count -= time_delta;
        }

        if self.starship.shoot_direction.len() > 0.0 && self.starship.shoot_cooldown_count <= 0.0 {
            self.starship.shoot_cooldown_count = self.starship.shoot_cooldown;
            let id = self.starship.entity_id;
            let lander_entity = self.entities.get_object(id);
            let position = lander_entity.position();
            let init_velocity = 40.0 * (lander_entity.velocity() + 1.0);
            let direction = self.starship.shoot_direction * init_velocity;
            self.sound.shoot();
            self.create_missile(position, direction);
        }

        self.missiles.for_each(|missile, _| {
            missile.time_to_live -= time_delta;
            let id = missile.entity_id;
            let entity = self.entities.get_object(id);
            let position = entity.position();
            Self::intersect_grid(position, &mut self.grid);
        });
    }

    fn intersect_grid(missile_pos: Vec2d, grid: &mut VertexGrid) {
        grid.intersect_grid(missile_pos);
    }

    fn make_safe_enemy_position(&self) -> Vec2d {
        loop {
            let pos = Vec2d {
                x: thread_rng().gen_range(0..(WORLD_SIZE.x as usize)) as f32,
                y: -(thread_rng().gen_range(0..(WORLD_SIZE.y as usize)) as f32),
            };
            let id = self.starship.entity_id;
            let player_pos = self.entities.get_object(id).position();
            if (player_pos - pos).len() > 300f32 {
                return pos;
            }
        }
    }

    fn enemy_tick(&mut self) {
        // check if we have enough enemies:
        self.spawn_enemies();
        self.spawn_obstacles();

        self.enemies.for_each(|enemy, _| {
            enemy.tick(
                &self.entities,
                self.starship.entity_id,
                &self.missiles,
                &mut self.grid,
            );
        });
    }
    fn spawn_obstacles(&mut self) {
        let max_obstacles: usize = 3;
        //limit amount of obstacles
        if self.obstacles.len() >= max_obstacles {
            return;
        }

        let pos = self.make_safe_enemy_position();

        self.entities.with_new(|new_obstacle, entity_index| {
            let obstacle: Obstacle = Obstacle {
                entity_id: (entity_index),
                obstacle_type: (obstacles::ObstacleType::Island),
                hull: (&BLACK_HOLE_ENEMY),
            };

            new_obstacle.set_max_velocity(100.0);
            new_obstacle.set_position(pos);
            new_obstacle.set_border_behavior(BorderBehavior::Bounce);

            self.obstacles.insert_object(obstacle);
        });
    }

    fn spawn_enemies(&mut self) {
        //return;
        const ENEMY_DISTRIBUTION: [f32; 7] = [0.2, 0.4, 0.6, 0.8, 0.9, 0.0, 1.0];
        //const ENEMY_DISTRIBUTION: [f32; 7] = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

        let rn: f64 = thread_rng().gen_range(0.0..=1.0);

        if self.enemies.len() < 1 || (1.0 / self.enemies.len() as f64) > rn {
            let should_spawn = thread_rng().gen_ratio(1, 100);

            if should_spawn {
                let num_to_spawn = thread_rng().gen_range(1..5);
                //let num_to_spawn = 1;

                for _ in 0..num_to_spawn {
                    let pos = self.make_safe_enemy_position();
                    self.entities.with_new(|the_entity, entity_index| {
                        //let enemy_type = thread_rng().gen_range(0..EnemyType::Invalid as usize);

                        //select actual enemy type based on distribution
                        let rnd: f32 = thread_rng().gen_range(0.0..=1.0);
                        let enemy_type =
                            if let Some(index) = ENEMY_DISTRIBUTION.iter().position(|&x| rnd < x) {
                                index
                            } else {
                                panic!("no valid distribution: {}", rnd)
                            };

                        let enemy;
                        match enemy_type {
                            0 => {
                                enemy = Enemy {
                                    ty: EnemyType::Rombus,
                                    entity_id: entity_index,
                                    hull: &ROMBUS_ENEMY,
                                    num_ticks: 0,
                                    hitpoints: 1,
                                    is_triggered: false,
                                }
                            }
                            1 => {
                                enemy = Enemy {
                                    ty: EnemyType::Rect,
                                    entity_id: entity_index,
                                    hull: &RECT_ENEMY,
                                    num_ticks: 0,
                                    hitpoints: 1,
                                    is_triggered: false,
                                }
                            }
                            2 => {
                                enemy = Enemy {
                                    ty: EnemyType::Cannon,
                                    entity_id: entity_index,
                                    hull: &CANNON_ENEMY,
                                    num_ticks: 0,
                                    hitpoints: 1,
                                    is_triggered: false,
                                }
                            }
                            3 => {
                                enemy = Enemy {
                                    ty: EnemyType::Wanderer,
                                    entity_id: entity_index,
                                    hull: &RECT_ENEMY,
                                    num_ticks: 0,
                                    hitpoints: 1,
                                    is_triggered: false,
                                }
                            }
                            4 => {
                                enemy = Enemy {
                                    ty: EnemyType::SpawningRect,
                                    entity_id: entity_index,
                                    hull: &RECT_ENEMY,
                                    num_ticks: 0,
                                    hitpoints: 1,
                                    is_triggered: false,
                                }
                            }
                            5 => {
                                // We don't spawn minirects directly
                                enemy = Enemy {
                                    ty: EnemyType::SpawningRect,
                                    entity_id: entity_index,
                                    hull: &MINIRECT_ENEMY,
                                    num_ticks: 0,
                                    hitpoints: 1,
                                    is_triggered: false,
                                }
                            }
                            6 => {
                                enemy = Enemy {
                                    ty: EnemyType::BlackHole,
                                    entity_id: entity_index,
                                    hull: &BLACK_HOLE_ENEMY,
                                    num_ticks: 0,
                                    hitpoints: 10,
                                    is_triggered: false,
                                }
                            }
                            _ => {
                                enemy = Enemy {
                                    ty: EnemyType::Wanderer,
                                    entity_id: entity_index,
                                    hull: &RECT_ENEMY,
                                    num_ticks: 0,
                                    hitpoints: 1,
                                    is_triggered: false,
                                }
                            }
                        }

                        the_entity.set_max_velocity(100.0);
                        the_entity.set_position(pos);
                        the_entity.set_border_behavior(BorderBehavior::Bounce);
                        self.enemies.insert_object(enemy);
                    });
                }
            }
        }
    }

    fn do_collision_detection(&mut self) {
        if self.game_state != State::Running {
            return;
        }

        let id = self.starship.entity_id;
        let player_entity = self.entities.get_object(id);
        let player_position = player_entity.position();
        let player_transform = make_entity_transform(player_entity);
        let player_hull = player_transform.transform_many_slice(&STARSHIP);

        let mut enemies_to_delete = Vec::<usize>::new();
        let mut missiles_to_delete = Vec::<usize>::new();
        let mut minirect_spawns = Vec::<usize>::new();

        let mut new_score: u32 = 0;
        let mut new_texts: Vec<FloatingText> = Vec::new();

        let mut swapped_enemies = ObjectStore::new();
        std::mem::swap(&mut self.enemies, &mut swapped_enemies);

        let mut swapped_missiles = ObjectStore::new();
        std::mem::swap(&mut self.missiles, &mut swapped_missiles);

        let mut player_died = false;

        swapped_missiles.for_each_immutable(|missile, _| {
            if !missile.hostile || player_died {
                return;
            }

            let missile_entity = self.entities.get_object(missile.entity_id);

            player_died = collision::hit_test(missile_entity.position(), &player_hull);
        });

        swapped_enemies.for_each(|enemy, _| {
            if player_died {
                return;
            }
            // create collidable hull for entity:
            let enemy_ent = self.entities.get_object(enemy.entity_id);
            let enemy_pos = enemy_ent.position();
            let enemy_transform = make_entity_transform(enemy_ent);
            let enemy_hull = enemy_transform.transform_many_slice(enemy.hull);

            player_died = collision::hit_test(player_position, &enemy_hull);
            if player_died {
                return;
            }

            // Check collision against missiles
            swapped_missiles.for_each_immutable(|missile, _| {
                if missile.hostile {
                    return;
                }
                let missile_entity = self.entities.get_object(missile.entity_id);

                // make sure each missile can only hit once!
                if missiles_to_delete.contains(&missile.entity_id) {
                    return;
                }

                let projectile_collision =
                    collision::hit_test(missile_entity.position(), &enemy_hull);

                if projectile_collision {
                    enemy.hitpoints -= 1;
                    if enemy.hitpoints == 0 {
                        self.kill_enemy(
                            &mut enemies_to_delete,
                            enemy,
                            enemy_pos,
                            &mut missiles_to_delete,
                            missile,
                            &mut new_score,
                            &mut minirect_spawns,
                            &mut new_texts,
                        );
                    } else {
                        missiles_to_delete.push(missile.entity_id);
                    }
                }
            });
        });

        if player_died {
            self.sound.die();
            // Ideally, make a huge explosion.
            if self.lifes > 0 && self.game_state == State::Running {
                self.lifes -= 1;
                self.kills_this_life = 0;
                self.multiplier = 1.0;
                self.game_state = State::WaitingForRespawn(4.0);
            } else {
                self.game_state = State::Lost;
            }
            // destroy all enemies:
            swapped_enemies.for_each_immutable(|enemy, _| enemies_to_delete.push(enemy.entity_id));
        }

        // swap back:
        std::mem::swap(&mut self.enemies, &mut swapped_enemies);

        // swap back:
        std::mem::swap(&mut self.missiles, &mut swapped_missiles);

        self.update_score(new_score);
        self.texts.extend(new_texts);

        self.spawn_minirects(minirect_spawns);

        self.missiles
            .garbage_collect_filter(|x| missiles_to_delete.contains(&x.entity_id));
        self.garbage_collect_entities(&missiles_to_delete);
        self.garbage_collect_entities(&enemies_to_delete);
        self.enemies
            .garbage_collect_filter(|a| enemies_to_delete.contains(&a.entity_id))
    }

    #[inline]
    fn kill_enemy(
        &mut self,
        enemies_to_delete: &mut Vec<usize>,
        enemy: &Enemy<'_>,
        enemy_pos: Vec2d,
        missiles_to_delete: &mut Vec<usize>,
        missile: &Missile,
        new_hit_points: &mut u32,
        minirect_spawns: &mut Vec<usize>,
        new_texts: &mut Vec<FloatingText>,
    ) {
        self.sound.explode();
        self.kills_this_life += 1;
        enemies_to_delete.push(enemy.entity_id);
        self.grid.add_circular_effect(
            enemy_pos,
            32.0,
            1.0f32,
            250.0f32,
            CircularEffectType::Explosion,
        );

        if !missiles_to_delete.contains(&missile.entity_id) {
            missiles_to_delete.push(missile.entity_id);
        }

        *new_hit_points += enemy.get_score();
        if enemy.ty == EnemyType::SpawningRect {
            minirect_spawns.push(missile.entity_id);
        }
        new_texts.push(FloatingText::new(
            enemy_pos,
            format!("{}", enemy.get_score()),
        ));
        self.explosions.push(Explosion::new(enemy_pos));
    }

    fn spawn_minirects(&mut self, minirect_spawns: Vec<usize>) {
        for id in minirect_spawns.iter() {
            let spawnpos;
            {
                let entity = self.entities.get_object(*id);
                spawnpos = entity.position();
            }

            for i in 0..2 {
                self.entities.with_new(|the_entity, entity_index| {
                    the_entity.set_position(spawnpos.clone() + Vec2d::new(16f32, 16f32) * i as f32);
                    the_entity.set_border_behavior(BorderBehavior::Bounce);
                    let minirect = Enemy {
                        entity_id: entity_index,
                        ty: EnemyType::MiniRect,
                        hull: &MINIRECT_ENEMY,
                        num_ticks: 0,
                        hitpoints: 1,
                        is_triggered: false,
                    };
                    self.enemies.insert_object(minirect);
                });
            }
        }
    }

    fn update_score(&mut self, hit_points: u32) {
        let new_score = self.score + hit_points;
        const COOLDOWN_STEP: f32 = (MAX_SHOOT_COOLDOWN - MIN_SHOOT_COOLDOWN) / 3.0;
        for (level_score, shoot_cooldown_idx) in vec![(5_000, 1), (10_000, 2), (15_000, 3)] {
            if self.score <= level_score && level_score < new_score {
                self.starship.shoot_cooldown =
                    MAX_SHOOT_COOLDOWN - (shoot_cooldown_idx as f32 * COOLDOWN_STEP);
            }
        }
        self.score = new_score;
    }

    fn render_hud(&mut self, canvas: &mut sdl2::render::Canvas<sdl2::video::Window>) {
        let id = self.starship.entity_id;
        let entity = self.entities.get_object(id);
        self.hud.update(
            entity.position(),
            entity.direction(),
            entity.acceleration(),
            entity.angle(),
            self.score,
            self.enemies.len() as u32,
        );
        self.hud.render(canvas);
    }

    fn render_starship(
        &self,
        lander_entity: &Entity,
        screen_space_transform: TransformationMatrix,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        textures: &HashMap<String, Texture>,
    ) {
        let scale = vecmath::TransformationMatrix::scale(
            graphics::ENTITY_SCALE.x,
            graphics::ENTITY_SCALE.y,
        );
        let entity_trans = lander_entity.get_screenspace_transform(screen_space_transform);
        // fix orientation of lander and rotate 90 deg
        let offset = vecmath::TransformationMatrix::rotate(PI / 2.0);
        let transform = entity_trans * scale * offset;
        let items = [&graphics::STARSHIP];
        let texture = textures.get("neon").unwrap();
        for lander_part in items.iter() {
            let geometry = transform.transform_many(&lander_part.to_vec());
            draw::neon_draw_lines(canvas, &geometry, STARSHIP_COLOR, true, texture).unwrap();
        }

        // if self.starship.drive_enabled {
        //     let geometry;
        //     geometry = transform.transform_many(&graphics::FLAME_A.to_vec());
        //     draw::draw_lines(canvas, &geometry, Color::RGB(255, 255, 255), true).unwrap();
        // }

        let lander_center = screen_space_transform.transform(&lander_entity.position());
        let _ = canvas.copy(
            textures.get("halo").unwrap(),
            None,
            sdl2::rect::Rect::new(
                lander_center.x as i32 - 32,
                lander_center.y as i32 - 32,
                64,
                64,
            ),
        );
    }

    fn render_enemies(
        &self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        screen_space_transform: TransformationMatrix,
        textures: &HashMap<String, Texture<'_>>,
    ) {
        self.enemies.for_each(|enemy, _| {
            let entity = self.entities.get_object(enemy.entity_id);
            enemy.render(canvas, screen_space_transform, textures, &entity);
        });
        self.obstacles.for_each(|obstacles, _| {
            let entity = self.entities.get_object(obstacles.entity_id);
            obstacles.render(canvas, screen_space_transform, textures, &entity);
        });
    }

    fn render_explosions(
        &self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        screen_space_transform: TransformationMatrix,
        textures: &HashMap<String, Texture<'_>>,
    ) {
        let texture = textures.get("star").unwrap();
        for exp in &self.explosions {
            let pos = exp.position;
            let pos_screen = screen_space_transform.transform(&pos);
            for sparc_dir in &exp.sparc_dir {
                let pos_screen = pos_screen + (sparc_dir.clone() * exp.frame_count as f32);

                _ = canvas.copy(
                    texture,
                    None,
                    sdl2::rect::Rect::new(pos_screen.x as i32, pos_screen.y as i32, 12, 12),
                );
            }
        }
    }

    fn render_texts(
        &self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        screen_space_transform: TransformationMatrix,
    ) {
        for text_ele in &self.texts {
            let screen_coordinate = screen_space_transform.transform(&text_ele.position);
            let screen_coordinate =
                Point::new(screen_coordinate.x as i32, screen_coordinate.y as i32);
            draw::draw_text(
                canvas,
                &text_ele.text,
                15,
                screen_coordinate,
                Color::RGB(0, 255, 0),
            )
            .unwrap();
        }
    }

    fn render_world_border(
        &self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        screen_space_transform: TransformationMatrix,
    ) {
        let width: u32 = WORLD_SIZE.x as u32;
        let height: u32 = WORLD_SIZE.y as u32;
        let rect: Rect = sdl2::rect::Rect::new(0, 0, width, height);

        let mut top_left: Vec2d =
            Vec2d::new((rect.top_left()).x as f32, (rect.top_left()).y as f32);

        let mut top_right: Vec2d =
            Vec2d::new((rect.top_right()).x as f32, (rect.top_right()).y as f32);

        let mut bot_left: Vec2d =
            Vec2d::new((rect.bottom_left()).x as f32, (rect.bottom_left()).y as f32);

        let mut bot_right: Vec2d = Vec2d::new(
            (rect.bottom_right()).x as f32,
            (rect.bottom_right()).y as f32,
        );

        top_left = screen_space_transform.transform(&top_left);
        top_right = screen_space_transform.transform(&top_right);
        bot_left = screen_space_transform.transform(&bot_left);
        bot_right = screen_space_transform.transform(&bot_right);

        let _ = draw::draw_line(canvas, &top_left, &top_right, Color::WHITE);
        let _ = draw::draw_line(canvas, &top_left, &bot_left, Color::WHITE);
        let _ = draw::draw_line(canvas, &top_right, &bot_right, Color::WHITE);
        let _ = draw::draw_line(canvas, &bot_left, &bot_right, Color::WHITE);
    }

    fn render_grid(
        &self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        screen_space_transform: TransformationMatrix,
    ) {
        self.grid.render(canvas, screen_space_transform);
    }

    pub fn toggle_background_music(&mut self) {
        self.sound.toggle_background_music();
    }

    pub(crate) fn modify_axis(&mut self, axis: sdl2::controller::Axis, value: i16) {
        self.active_controle_scheme = ActiveControleScheme::Gamepad;
        match axis {
            sdl2::controller::Axis::LeftX => {
                self.axis_l_x = value;
            }
            sdl2::controller::Axis::LeftY => {
                self.axis_l_y = value;
            }
            sdl2::controller::Axis::RightX => {
                self.axis_r_x = value;
            }
            sdl2::controller::Axis::RightY => {
                self.axis_r_y = value;
            }
            sdl2::controller::Axis::TriggerLeft => {}
            sdl2::controller::Axis::TriggerRight => {}
        }

        let new_dir = Vec2d::new(self.axis_l_x as f32, self.axis_l_y as f32);

        if new_dir.is_not_zero() {
            if new_dir.len() < 1500.0 {
                self.set_player_direction(Vec2d::default());
            } else {
                self.set_player_direction(new_dir.normalized());
            }
        }

        let new_shoot_dir = Vec2d::new(self.axis_r_x as f32, self.axis_r_y as f32);
        if new_shoot_dir.is_not_zero() {
            if new_shoot_dir.len() < 1500.0 {
                self.starship.shoot_direction = Vec2d::default();
            } else {
                self.starship.shoot_direction = new_shoot_dir.normalized();
            }
        }
    }

    fn reset_control(&mut self) {
        self.active_controle_scheme = ActiveControleScheme::Keyboard;
        self.axis_l_x = 0;
        self.axis_l_y = 0;
        self.axis_r_x = 0;
        self.axis_r_y = 0;
        self.game_control_bits = 0;
    }
}

fn make_entity_transform(enemy_ent: Entity) -> TransformationMatrix {
    let enemy_transform = enemy_ent.get_transform();
    let scale_transform = vecmath::TransformationMatrix::scale(ENTITY_SCALE.x, ENTITY_SCALE.y);
    let hull_transform = enemy_transform * scale_transform;
    hull_transform
}
