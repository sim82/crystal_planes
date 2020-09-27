use bevy::{
    prelude::*,
    render::{
        mesh::{shape, VertexAttributeValues},
        pipeline::{DynamicBinding, PipelineDescriptor, PipelineSpecialization, RenderPipeline},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph},
        renderer::RenderResources,
        shader::{ShaderStage, ShaderStages},
    },
};

pub struct RadBuffer {
    pub r: Vec<f32>,
    pub g: Vec<f32>,
    pub b: Vec<f32>,
}

impl RadBuffer {
    pub fn new(size: usize) -> RadBuffer {
        RadBuffer {
            r: vec![0.0; size],
            g: vec![0.0; size],
            b: vec![0.0; size],
        }
    }

    pub fn new_with(size: usize, r: f32, g: f32, b: f32) -> Self {
        RadBuffer {
            r: vec![r; size],
            g: vec![g; size],
            b: vec![b; size],
        }
    }
}

pub struct FrontBuf(pub RadBuffer);
pub struct BackBuf(pub RadBuffer);

#[derive(Bundle)]
pub struct PlaneBundle {
    pub plane: Plane,
}
pub struct Plane {
    pub buf_index: usize,
}
