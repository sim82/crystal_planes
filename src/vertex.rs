#[allow(unused_imports)]
use amethyst::prelude::*;

use amethyst::renderer::rendy::{
    hal::format::Format,
    mesh::{AsAttribute, AsVertex, VertexFormat},
};

// custom attributes
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QuadDir(pub u32);
impl<T> From<T> for QuadDir
where
    T: Into<u32>,
{
    fn from(from: T) -> Self {
        QuadDir(from.into())
    }
}
impl AsAttribute for QuadDir {
    const NAME: &'static str = "dir";
    const FORMAT: Format = Format::R32Uint;
}

/// Type for position attribute of vertex.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Translate(pub [f32; 3]);
impl<T> From<T> for Translate
where
    T: Into<[f32; 3]>,
{
    fn from(from: T) -> Self {
        Translate(from.into())
    }
}
impl AsAttribute for Translate {
    const NAME: &'static str = "translate";
    const FORMAT: Format = Format::Rgb32Sfloat;
}

// #[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
// #[repr(C, align(4))]
// pub struct QuadArgs {
//     position: Position,
// }

// impl AsVertex for QuadArgs {
//     fn vertex() -> VertexFormat {
//         VertexFormat::new((
//             // position: vec3
//             Position::vertex(),
//         ))
//     }
// }

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[repr(C, align(16))]
pub struct QuadInstanceArgsConst {
    pub translate: Translate,
    pub dir: QuadDir,
    // pub color: Color,
}

impl AsVertex for QuadInstanceArgsConst {
    fn vertex() -> VertexFormat {
        VertexFormat::new((
            // color: vec3
            Translate::vertex(),
            // pad: u32
            QuadDir::vertex(),
            // Color::vertex(),
        ))
    }
}
