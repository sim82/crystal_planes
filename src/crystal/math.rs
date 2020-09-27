use bevy::math::prelude::*;
use core::ops::*;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Default)]
pub struct Vec3i(pub i32, pub i32, pub i32);
pub type Point3i = Vec3i;

impl Vec3i {
    pub fn new(x: i32, y: i32, z: i32) -> Vec3i {
        Vec3i(x, y, z)
    }

    pub fn into_vec3(self) -> Vec3 {
        Vec3::new(self.0 as f32, self.1 as f32, self.2 as f32)
    }

    pub fn x(&self) -> i32 {
        self.0
    }

    pub fn y(&self) -> i32 {
        self.1
    }

    pub fn z(&self) -> i32 {
        self.2
    }
}

impl Add for Vec3i {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0, self.1 + other.1, self.2 + other.2)
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Default)]
pub struct Vec2i(pub i32, pub i32);
pub type Point2i = Vec2i;

impl Vec2i {
    pub fn new(x: i32, y: i32) -> Vec2i {
        Vec2i(x, y)
    }

    pub fn x(&self) -> i32 {
        self.0
    }

    pub fn y(&self) -> i32 {
        self.1
    }
}

pub struct DisplayWrap<T>(T);

impl<T> From<T> for DisplayWrap<T> {
    fn from(t: T) -> Self {
        DisplayWrap(t)
    }
}

impl std::fmt::Display for DisplayWrap<Point3i> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let DisplayWrap::<Point3i>(v) = self;
        write!(f, "[{} {} {}]", v.0, v.1, v.2)
    }
}

impl std::fmt::Display for DisplayWrap<[i32; 4]> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let DisplayWrap::<[i32; 4]>([i1, i2, i3, i4]) = self;

        write!(f, "[{} {} {} {}]", i1, i2, i3, i4)
    }
}

pub mod prelude {
    pub use super::{DisplayWrap, Point2i, Point3i, Vec2i, Vec3i};
}
