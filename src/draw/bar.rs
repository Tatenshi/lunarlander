use std::{collections::HashMap, f64::consts::E};

use sdl2::{pixels::Color, render::Texture};

use crate::{draw, vecmath::Vec2d};

pub struct Bar;

impl Bar {
    pub fn render(
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        textures: &HashMap<String, Texture<'_>>,
        elements: u32,
        max_elements: u32,
        pos: Vec2d,
        max_allowed_length: u32,
        inner_color: Color,
        outer_color: Color,
    ) {
        const SPACE_TO_WORLD_FACTOR: u32 = 5;
        const HEIGHT: u32 = 20;

        let max_length = max_allowed_length - max_allowed_length / SPACE_TO_WORLD_FACTOR;
        let length = ((elements as f32 / max_elements as f32) * max_length as f32).round() as u32;
        print!(
            "bar v: {} | max: {} | l: {}\n",
            elements, max_elements, length
        );
        // Draw filling

        // If we have zero elements, we draw no filling
        if elements != 0 {
            draw::draw_rect(canvas, &pos, length, HEIGHT, inner_color, true).unwrap();
        }

        // Draw outer border
        let mut points: Vec<Vec2d> = Vec::new();
        points.push(pos);
        points.push(Vec2d {
            x: pos.x + max_length as f32,
            y: pos.y,
        });
        points.push(Vec2d {
            x: pos.x + max_length as f32,
            y: pos.y + HEIGHT as f32,
        });
        points.push(Vec2d {
            x: pos.x,
            y: pos.y + HEIGHT as f32,
        });
        draw::neon_draw_lines(
            canvas,
            &points,
            outer_color,
            true,
            textures.get("neon").unwrap(),
        )
        .unwrap();
    }
}
