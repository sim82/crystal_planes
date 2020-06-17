use amethyst::error::Error;
use amethyst::{
    core::ecs::{DispatcherBuilder, Join, Read, ReadStorage, SystemData, World},
    prelude::*,
    renderer::{
        bundle::{RenderOrder, RenderPlan, RenderPlugin, Target},
        pipeline::{PipelineDescBuilder, PipelinesBuilder},
        pod::ViewArgs,
        rendy::{
            command::{QueueId, RenderPassEncoder},
            factory::Factory,
            graph::{
                render::{PrepareResult, RenderGroup, RenderGroupDesc},
                GraphContext, NodeBuffer, NodeImage,
            },
            hal::{self, device::Device, pso},
            mesh::{AsVertex, Color, Mesh, Position},
            shader::{Shader, SpirvShader},
        },
        submodules::{gather::CameraGatherer, DynamicUniform, DynamicVertexBuffer},
        types::Backend,
        util, ChangeDetection,
    },
};

use genmesh::{
    generators::{IndexedPolygon, SharedVertex},
    Triangulate,
};

use crate::{quad::QuadInstance, vertex::*};

use derivative::Derivative;

pub type Triangle = crate::custom_pass::Triangle;

use amethyst::renderer::rendy::shader::{PathBufShaderInfo, ShaderKind, SourceLanguage};
use std::path::PathBuf;
lazy_static::lazy_static! {
    static ref VERTEX: SpirvShader = PathBufShaderInfo::new(
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src/assets/shaders/src/vertex/quad.vert")),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
       "main",
    ).precompile().unwrap();
    static ref FRAGMENT: SpirvShader = PathBufShaderInfo::new(
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src/assets/shaders/src/fragment/quad.frag")),
        ShaderKind::Fragment,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();
}
/// '''

/// Draw triangles.
#[derive(Clone, Debug, PartialEq, Derivative)]
#[derivative(Default(bound = ""))]
pub struct DrawQuadDesc;

impl DrawQuadDesc {
    /// Create instance of `DrawQuadDesc` render group
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug)]
struct InstanceBuffers<B: Backend> {
    instance: DynamicVertexBuffer<B, Color>,
    instance_const: DynamicVertexBuffer<B, QuadInstanceArgsConst>,
    instance_count: usize,
}

impl<B: Backend> RenderGroupDesc<B, World> for DrawQuadDesc {
    fn build(
        self,
        _ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        _queue: QueueId,
        _world: &World,
        framebuffer_width: u32,
        framebuffer_height: u32,
        subpass: hal::pass::Subpass<'_, B>,
        _buffers: Vec<NodeBuffer>,
        _images: Vec<NodeImage>,
    ) -> Result<Box<dyn RenderGroup<B, World>>, failure::Error> {
        let env = DynamicUniform::new(factory, pso::ShaderStageFlags::VERTEX)?;
        let instance = DynamicVertexBuffer::new();
        let instance_const = DynamicVertexBuffer::new();

        let (pipeline, pipeline_layout) = build_custom_pipeline(
            factory,
            subpass,
            framebuffer_width,
            framebuffer_height,
            vec![env.raw_layout()],
        )?;

        let instance_buffers = InstanceBuffers::<B> {
            instance,
            instance_const,
            instance_count: 0,
        };

        Ok(Box::new(DrawQuad::<B> {
            pipeline,
            pipeline_layout,
            env,
            quad_mesh: None,
            change: Default::default(),
            instance_buffers,
            color_generation: Vec::new(),
        }))
    }
}

/// Draws triangles to the screen.
#[derive(Debug)]
pub struct DrawQuad<B: Backend> {
    pipeline: B::GraphicsPipeline,
    pipeline_layout: B::PipelineLayout,
    env: DynamicUniform<B, ViewArgs>,
    quad_mesh: Option<Mesh<B>>,
    change: ChangeDetection,
    instance_buffers: InstanceBuffers<B>,
    color_generation: Vec<usize>,
}

impl<B: Backend> RenderGroup<B, World> for DrawQuad<B> {
    fn prepare(
        &mut self,
        factory: &Factory<B>,
        queue: QueueId,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        world: &World,
    ) -> PrepareResult {
        let projview = CameraGatherer::gather(world).projview;
        self.env.write(factory, index, projview);
        // println!("projview: {:?}", projview);
        let mut changed = false;
        if self.quad_mesh.is_none() {
            self.quad_mesh = Some(gen_quad_mesh(queue, &factory));
            changed = true;
        }

        let instance_buffers = &mut self.instance_buffers;
        let color_generation = <Read<Option<crate::quad::ColorGeneration>>>::fetch(world);

        if instance_buffers.instance_count == 0 {
            let quad_instances = <ReadStorage<'_, QuadInstance>>::fetch(world);
            let mut qi = quad_instances.join().collect::<Vec<_>>();

            qi.sort_unstable_by(|a, b| a.index.cmp(&b.index));
            instance_buffers.instance_count = qi.len();

            let instance_data_const_iter = qi.iter().map(|instance| instance.get_args_const());

            instance_buffers.instance_const.write(
                factory,
                index,
                instance_buffers.instance_count as u64,
                Some(instance_data_const_iter.collect::<Box<[QuadInstanceArgsConst]>>()),
            );
            let instance_data_iter = qi.iter().map(|instance| instance.get_args());
            instance_buffers.instance.write(
                factory,
                0,
                instance_buffers.instance_count as u64,
                Some(instance_data_iter.collect::<Box<[Color]>>()),
            );
            // println!("instance: {:?}", self.instance);
            changed = true;
        } else if let Some(ref color_generation) = *color_generation {
            if self.color_generation.len() <= index {
                self.color_generation.resize(index + 1, 0);
            }
            if color_generation.0 != self.color_generation[index] {
                // println!("write color: {}", index);
                let quad_instances = <ReadStorage<'_, QuadInstance>>::fetch(world);
                let mut qi = quad_instances.join().collect::<Vec<_>>();

                qi.sort_unstable_by(|a, b| a.index.cmp(&b.index));
                let instance_data_iter = qi.iter().map(|instance| instance.get_args());
                instance_buffers.instance.write(
                    factory,
                    index,
                    instance_buffers.instance_count as u64,
                    Some(instance_data_iter.collect::<Box<[Color]>>()),
                );
                // println!("instance: {:?}", self.instance);
                changed = true;
                self.color_generation[index] = color_generation.0;
            }
        }

        self.change.prepare_result(index, changed)
    }

    fn draw_inline(
        &mut self,
        mut encoder: RenderPassEncoder<'_, B>,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        _world: &World,
    ) {
        // Don't worry about drawing if there are no vertices. Like before the state adds them to the screen.
        if self.quad_mesh.is_none() {
            return;
        }
        // println!("draw: {}", index);
        // Bind the pipeline to the the encoder
        encoder.bind_graphics_pipeline(&self.pipeline);

        // Bind the Dynamic buffer with the scale to the encoder
        self.env.bind(index, &self.pipeline_layout, 0, &mut encoder);

        let quad_mesh = &self.quad_mesh.as_ref().unwrap();
        quad_mesh
            .bind(0, &[Position::vertex()], &mut encoder)
            .unwrap();

        let instance_buffers = &mut self.instance_buffers;
        instance_buffers.instance.bind(index, 1, 0, &mut encoder);
        instance_buffers.instance_const.bind(0, 2, 0, &mut encoder);

        // Draw the vertices
        unsafe {
            encoder.draw_indexed(
                0..quad_mesh.len() as u32,
                0,
                0..instance_buffers.instance_count as u32,
            );
        }
    }

    fn dispose(self: Box<Self>, factory: &mut Factory<B>, _world: &World) {
        unsafe {
            factory.device().destroy_graphics_pipeline(self.pipeline);
            factory
                .device()
                .destroy_pipeline_layout(self.pipeline_layout);
        }
    }
}

fn build_custom_pipeline<B: Backend>(
    factory: &Factory<B>,
    subpass: hal::pass::Subpass<'_, B>,
    framebuffer_width: u32,
    framebuffer_height: u32,
    layouts: Vec<&B::DescriptorSetLayout>,
) -> Result<(B::GraphicsPipeline, B::PipelineLayout), failure::Error> {
    let pipeline_layout = unsafe {
        factory
            .device()
            .create_pipeline_layout(layouts, None as Option<(_, _)>)
    }?;

    // Load the shaders
    let shader_vertex = unsafe { VERTEX.module(factory).unwrap() };
    let shader_fragment = unsafe { FRAGMENT.module(factory).unwrap() };
    println!(
        "desc: {:?}",
        [
            (Position::vertex(), pso::VertexInputRate::Vertex),
            (
                QuadInstanceArgsConst::vertex(),
                pso::VertexInputRate::Instance(1),
            ),
            (Color::vertex(), pso::VertexInputRate::Instance(1)),
        ]
    );
    // Build the pipeline
    let pipes = PipelinesBuilder::new()
        .with_pipeline(
            PipelineDescBuilder::new()
                .with_vertex_desc(&[
                    (Position::vertex(), pso::VertexInputRate::Vertex),
                    (Color::vertex(), pso::VertexInputRate::Instance(1)),
                    (
                        QuadInstanceArgsConst::vertex(),
                        pso::VertexInputRate::Instance(1),
                    ),
                ])
                .with_input_assembler(pso::InputAssemblerDesc::new(hal::Primitive::TriangleList))
                // Add the shaders
                .with_shaders(util::simple_shader_set(
                    &shader_vertex,
                    Some(&shader_fragment),
                ))
                .with_layout(&pipeline_layout)
                .with_subpass(subpass)
                .with_framebuffer_size(framebuffer_width, framebuffer_height)
                // We are using alpha blending
                .with_depth_test(pso::DepthTest {
                    fun: pso::Comparison::Less,
                    write: true,
                })
                .with_blend_targets(vec![pso::ColorBlendDesc {
                    mask: pso::ColorMask::ALL,
                    blend: None,
                }]),
        )
        .build(factory, None);

    // Destoy the shaders once loaded
    unsafe {
        factory.destroy_shader_module(shader_vertex);
        factory.destroy_shader_module(shader_fragment);
    }

    // Handle the Errors
    match pipes {
        Err(e) => {
            unsafe {
                factory.device().destroy_pipeline_layout(pipeline_layout);
            }
            Err(e)
        }
        Ok(mut pipes) => Ok((pipes.remove(0), pipeline_layout)),
    }
}

/// A [RenderPlugin] for our custom plugin
#[derive(Default, Debug)]
pub struct RenderQuad {}

impl<B: Backend> RenderPlugin<B> for RenderQuad {
    fn on_build<'a, 'b>(
        &mut self,
        world: &mut World,
        _builder: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        // Add the required components to the world ECS
        world.register::<Triangle>();
        // world.insert(QuadUniformArgs { scale: 1.0 });
        Ok(())
    }

    fn on_plan(
        &mut self,
        plan: &mut RenderPlan<B>,
        _factory: &mut Factory<B>,
        _world: &World,
    ) -> Result<(), Error> {
        plan.extend_target(Target::Main, |ctx| {
            // Add our Description
            ctx.add(RenderOrder::Transparent, DrawQuadDesc::new().builder())?;
            Ok(())
        });
        Ok(())
    }
}

pub fn gen_quad_mesh<B: Backend>(queue: QueueId, factory: &Factory<B>) -> Mesh<B> {
    let plane = genmesh::generators::Plane::new();
    let indices: Vec<_> = genmesh::Vertices::vertices(plane.indexed_polygon_iter().triangulate())
        .map(|i| i as u32)
        .collect();

    println!("indices: {}", indices.len());
    let vertices: Vec<_> = plane
        .shared_vertex_iter()
        .map(|v| Position(v.pos.into()))
        .collect();
    println!("vertices: {}", vertices.len());
    // for v in &vertices {
    //     println!("vert: {:?}", v);
    // }
    println!("indices: {:?}", indices);
    println!("vertices: {:?}", vertices);
    let mesh = Mesh::<B>::builder()
        .with_indices(indices)
        .with_vertices(vertices)
        .build(queue, factory)
        .unwrap();

    println!("mesh: {:?}", mesh);
    mesh
}
