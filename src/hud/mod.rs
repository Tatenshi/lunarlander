use std::fmt::format;
use std::sync::Arc;

use crate::draw;
use crate::vecmath::Vec2d;
use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::render::Canvas;
use sdl2::video::Window;

pub struct Hud {
    position: Vec2d,
    direction: Vec2d,
    acceleration: Vec2d,
    angle: f32,
    score: u32,
    boost: f32,
    asteroids: u32,
    frames_since_last_fps_update: u32,
}

impl Hud {
    pub fn new() -> Self {
        Self {
            position: Vec2d::new(0.0, 0.0),
            direction: Vec2d::new(0.0, 0.0),
            acceleration: Vec2d::new(0.0, 0.0),
            angle: 0.0,
            score: 0,
            boost: 0.0,
            asteroids: 0,
            frames_since_last_fps_update: 0,
        }
    }

    pub fn tick(&mut self, time_in_ms: f32) {
        self.frames_since_last_fps_update = (1000.0 / time_in_ms) as u32;
    }

    pub fn update(
        &mut self,
        position: Vec2d,
        direction: Vec2d,
        acceleration: Vec2d,
        angle: f32,
        score: u32,
        boost: f32,
        asteroids: u32,
    ) {
        self.position = position;
        self.direction = direction;
        self.acceleration = acceleration;
        self.angle = angle;
        self.score = score;
        self.boost = boost;
        self.asteroids = asteroids;
    }

    pub fn render(&self, canvas: &mut Canvas<Window>) {
        let hud_position = format!("Position: x = {}, y = {}", self.position.x, self.position.y);
        let hud_direction = format!(
            "Direction: x = {}, y = {}",
            self.direction.x, self.direction.y
        );
        let hud_acceleration = format!(
            "Acceleration: x = {}, y = {}",
            self.acceleration.x, self.acceleration.y
        );
        let hud_angle = format!("Angle: {}", self.angle);
        let hud_score = format!("Score: {}", self.score);
        let hud_boost_fuel = format!("Boost fuel: {}", self.boost);
        let hud_asteroids = format!("Enemies: {}", self.asteroids);
        let hud_fps = format!("FPS: {}", self.frames_since_last_fps_update);

        let mut position = 0;
        for line in vec![
            hud_position,
            hud_direction,
            hud_acceleration,
            hud_angle,
            hud_score,
            hud_boost_fuel,
            hud_asteroids,
            hud_fps,
        ] {
            draw::draw_text(
                canvas,
                &line,
                10,
                Point::new(0, position),
                Color::RGB(0, 255, 0),
            )
            .unwrap();
            position += 10;
        }
    }
}
