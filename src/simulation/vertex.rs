use std::vec;

use crate::vecmath::Vec2d;

pub struct Vertex {
    main_position: Vec2d,
    position: Vec2d,
    direction: Vec2d,
}

impl Vertex {
    pub fn new(pos: Vec2d) -> Self {
        Self {
            position: pos,
            main_position: pos,
            direction: Vec2d::new(0.0, 0.0),
        }
    }

    pub fn mov(&mut self) {
        if self.direction.isnan() {
            panic!("dir is nan");
        }

        if self.direction.is_inf() {
            panic!("dir is inf");
        }

        if self.direction.is_not_zero() {
            self.position = self.position + self.direction;
            //self.direction = self.direction * 0.5;
        }

        if self.direction.isnan() {
            panic!("dir is nan");
        }

        if self.direction.is_inf() {
            panic!("dir is inf");
        }
    }

    pub fn add_to_dir(&mut self, dir: Vec2d) {
        if dir.is_inf() {
            panic!("dir is inf");
        }

        if dir.isnan() {
            panic!("dir is nan");
        }

        if self.direction.isnan() {
            panic!("dir is nan");
        }

        if dir.is_not_zero() {
            self.direction = self.direction + dir;
        }
    }

    pub fn set_dir_back(&mut self, multiplier: f32) {
        // Any vertex should slowly gravitate back to the main position
        // if no other forces move it:

        if self.direction.isnan() {
            panic!("dir is nan");
        }

        if self.direction.is_inf() {
            panic!("dir is inf");
        }

        // direction to main pos:
        let dir = self.main_position - self.position;
        if dir.isnan() {
            panic!("dir is nan");
        }

        if dir.is_inf() {
            panic!("dir is inf");
        }

        if dir.is_not_zero() {
            self.add_to_dir(dir * 0.15 * multiplier);

            // decrease velocity -> the grid should lost 99.5% of
            // its velocity per second, if no further force is added.
            let _d = self.direction * 0.995 * multiplier;
            self.direction = self.direction - _d;
        } else {
            // we are close to main pos, snap to it
            self.direction = Vec2d::new(0.0, 0.0);
            self.position = self.main_position;
        }

        if self.direction.isnan() {
            panic!("dir is nan");
        }

        if self.direction.is_inf() {
            panic!("dir is inf");
        }
    }

    pub fn position(&self) -> Vec2d {
        self.position
    }

    pub fn direction(&self) -> Vec2d {
        self.direction
    }
}
