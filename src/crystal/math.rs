use bevy::math::prelude::*;
use core::ops::*;
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Default, Serialize)]
pub struct Vec3i(pub i32, pub i32, pub i32);
pub type Point3i = Vec3i;

impl Vec3i {
    pub fn new(x: i32, y: i32, z: i32) -> Vec3i {
        Vec3i(x, y, z)
    }

    pub fn into_vec3(self) -> Vec3 {
        Vec3::new(self.0 as f32, self.1 as f32, self.2 as f32)
    }
    pub fn from_vec3(v: &Vec3) -> Self {
        Vec3i(v.x() as i32, v.y() as i32, v.z() as i32)
    }

    pub fn zero() -> Vec3i {
        Vec3i(0, 0, 0)
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

impl Mul<i32> for Vec3i {
    type Output = Self;

    #[inline]
    fn mul(self, other: i32) -> Self {
        Self(self.0 * other, self.1 * other, self.2 * other)
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
