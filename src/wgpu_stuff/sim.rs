use std::mem::size_of;
use wgpu::util::DeviceExt;

use crate::NUM_PARTICLES;
use crate::PARTICLES_PER_GROUP;

use crate::bodies;

pub struct Sim {
    particle_bind_groups: Vec<wgpu::BindGroup>,
    particle_buffers: Vec<wgpu::Buffer>,
    vertices_buffer: wgpu::Buffer,
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,
    work_group_count: u32,
    frame_num: usize,
}

impl crate::framework::Sim for Sim {
    fn required_limits() -> wgpu::Limits {
        wgpu::Limits::downlevel_defaults()
    }

    fn required_downlevel_capabilities() -> wgpu::DownlevelCapabilities {
        wgpu::DownlevelCapabilities {
            flags: wgpu::DownlevelFlags::COMPUTE_SHADERS,
            ..Default::default()
        }
    }

    /// constructs initial instance of Example struct
    fn init(
        config: &wgpu::SurfaceConfiguration,
        _adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        _queue: &wgpu::Queue
    ) -> Self {
        let compute_shader = device.create_shader_module(
            wgpu::include_wgsl!("shader/compute.wgsl")
        );
        let draw_shader = device.create_shader_module(wgpu::include_wgsl!("shader/draw.wgsl"));

        // buffer for simulation parameters uniform

        let sim_param_data = [
            0.04f32, // deltaT
        ].to_vec();
        let sim_param_buffer = device.create_buffer_init(
            &(wgpu::util::BufferInitDescriptor {
                label: Some("Simulation Parameter Buffer"),
                contents: bytemuck::cast_slice(&sim_param_data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            })
        );

        // create compute bind layout group and compute pipeline layout

        let compute_bind_group_layout = device.create_bind_group_layout(
            &(wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (sim_param_data.len() * size_of::<f32>()) as _
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new((NUM_PARTICLES * 8 * 4) as _),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new((NUM_PARTICLES * 8 * 4) as _),
                        },
                        count: None,
                    },
                ],
                label: None,
            })
        );
        let compute_pipeline_layout = device.create_pipeline_layout(
            &(wgpu::PipelineLayoutDescriptor {
                label: Some("compute"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            })
        );

        // create render pipeline with empty bind group layout

        let render_pipeline_layout = device.create_pipeline_layout(
            &(wgpu::PipelineLayoutDescriptor {
                label: Some("render"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            })
        );

        let render_pipeline = device.create_render_pipeline(
            &(wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &draw_shader,
                    entry_point: Some("main_vs"),
                    compilation_options: Default::default(),
                    buffers: &[
                        wgpu::VertexBufferLayout {
                            array_stride: 4 * 4,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                        },
                        wgpu::VertexBufferLayout {
                            array_stride: 2 * 4,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &wgpu::vertex_attr_array![2 => Float32x2],
                        },
                    ],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &draw_shader,
                    entry_point: Some("main_fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(config.view_formats[0].into())],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        );

        // create compute pipeline

        let compute_pipeline = device.create_compute_pipeline(
            &(wgpu::ComputePipelineDescriptor {
                label: Some("Compute pipeline"),
                layout: Some(&compute_pipeline_layout),
                module: &compute_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            })
        );

        // buffer for the three 2d triangle vertices of each instance

        let vertex_buffer_data = [-0.01f32, -0.02, 0.01, -0.02, 0.0, 0.02];
        let vertices_buffer = device.create_buffer_init(
            &(wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::bytes_of(&vertex_buffer_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            })
        );

        // buffer for all particles data of type [(posx,posy,velx,vely,accx,accy,mass),...]

        let mut initial_particle_data = vec![0.0f32; (8 * NUM_PARTICLES) as usize];

        let bodies = bodies::utils::uniform_disc(NUM_PARTICLES as usize);

        for (i, body) in bodies.iter().enumerate() {
            let particle_instance_chunk = &mut initial_particle_data[i * 8..(i + 1) * 8];
            particle_instance_chunk[0] = body.pos.x;
            particle_instance_chunk[1] = body.pos.y;
            particle_instance_chunk[2] = body.vel.x;
            particle_instance_chunk[3] = body.vel.y;
            particle_instance_chunk[4] = body.acc.x;
            particle_instance_chunk[5] = body.acc.y;
            particle_instance_chunk[6] = body.mass;
            particle_instance_chunk[7] = body.radius;
        }

        // creates two buffers of particle data each of size NUM_PARTICLES
        // the two buffers alternate as dst and src for each frame

        let mut particle_buffers = Vec::<wgpu::Buffer>::new();
        let mut particle_bind_groups = Vec::<wgpu::BindGroup>::new();
        for i in 0..2 {
            particle_buffers.push(
                device.create_buffer_init(
                    &(wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("Particle Buffer {i}")),
                        contents: bytemuck::cast_slice(&initial_particle_data),
                        usage: wgpu::BufferUsages::VERTEX |
                        wgpu::BufferUsages::STORAGE |
                        wgpu::BufferUsages::COPY_DST,
                    })
                )
            );
        }

        // create two bind groups, one for each buffer as the src
        // where the alternate buffer is used as the dst

        for i in 0..2 {
            particle_bind_groups.push(
                device.create_bind_group(
                    &(wgpu::BindGroupDescriptor {
                        layout: &compute_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: sim_param_buffer.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: particle_buffers[i].as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: particle_buffers[(i + 1) % 2].as_entire_binding(), // bind to opposite buffer
                            },
                        ],
                        label: None,
                    })
                )
            );
        }

        // calculates number of work groups from PARTICLES_PER_GROUP constant
        let work_group_count = (
            (NUM_PARTICLES as f32) / (PARTICLES_PER_GROUP as f32)
        ).ceil() as u32;

        // returns Example struct and No encoder commands

        Sim {
            particle_bind_groups,
            particle_buffers,
            vertices_buffer,
            compute_pipeline,
            render_pipeline,
            work_group_count,
            frame_num: 0,
        }
    }

    /// update is called for any WindowEvent not handled by the framework
    fn update(&mut self, _event: winit::event::WindowEvent) {
        //empty
    }

    /// resize is called on WindowEvent::Resized events
    fn resize(
        &mut self,
        _sc_desc: &wgpu::SurfaceConfiguration,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue
    ) {
        //empty
    }

    /// render is called each frame, dispatching compute groups proportional
    ///   a TriangleList draw call for all NUM_PARTICLES at 3 vertices each
    fn render(&mut self, view: &wgpu::TextureView, device: &wgpu::Device, queue: &wgpu::Queue) {
        // create render pass descriptor and its color attachments
        let color_attachments = [
            Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Not clearing here in order to test wgpu's zero texture initialization on a surface texture.
                    // Users should avoid loading uninitialized memory since this can cause additional overhead.
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
        ];
        let render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &color_attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        // get command encoder
        let mut command_encoder = device.create_command_encoder(
            &(wgpu::CommandEncoderDescriptor { label: None })
        );

        command_encoder.push_debug_group("compute boid movement");
        {
            // compute pass
            let mut cpass = command_encoder.begin_compute_pass(
                &(wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: None,
                })
            );
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.particle_bind_groups[self.frame_num % 2], &[]);
            cpass.dispatch_workgroups(self.work_group_count, 1, 1);
        }
        command_encoder.pop_debug_group();

        command_encoder.push_debug_group("render boids");
        {
            // render pass
            let mut rpass = command_encoder.begin_render_pass(&render_pass_descriptor);
            rpass.set_pipeline(&self.render_pipeline);
            // render dst particles
            rpass.set_vertex_buffer(0, self.particle_buffers[(self.frame_num + 1) % 2].slice(..));
            // the three instance-local vertices
            rpass.set_vertex_buffer(1, self.vertices_buffer.slice(..));
            rpass.draw(0..3, 0..NUM_PARTICLES);
        }
        command_encoder.pop_debug_group();

        // update frame count
        self.frame_num += 1;

        // done
        queue.submit(Some(command_encoder.finish()));
    }

    fn simulate(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        // get command encoder
        let mut command_encoder = device.create_command_encoder(
            &(wgpu::CommandEncoderDescriptor { label: None })
        );

        command_encoder.push_debug_group("compute boid movement");
        {
            // compute pass
            let mut cpass = command_encoder.begin_compute_pass(
                &(wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: None,
                })
            );
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.particle_bind_groups[self.frame_num % 2], &[]);
            cpass.dispatch_workgroups(self.work_group_count, 1, 1);
        }
        command_encoder.pop_debug_group();

        // update frame count
        self.frame_num += 1;

        // done
        queue.submit(Some(command_encoder.finish()));
    }
}