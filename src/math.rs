use amethyst::core::math;

pub type Vec2i = math::Vector2<i32>;

pub type Vec3i = math::Vector3<i32>;
pub type Vec3 = math::Vector3<f32>;
pub type Vec4 = math::Vector4<f32>;

pub type Point2i = math::Point2<i32>;
pub type Point3i = math::Point3<i32>;
pub type Point3 = math::Point3<f32>;
pub type Color = Vec3;

pub mod prelude {
    pub use super::{Color, Point2i, Point3, Point3i, Vec2i, Vec3, Vec3i, Vec4};
}
