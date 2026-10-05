use wgpu_test::{
    fail, fail_if, gpu_test, GpuTestConfiguration, GpuTestInitializer, TestParameters,
};

pub fn all_tests(vec: &mut Vec<GpuTestInitializer>) {
    vec.extend([
        COMPUTE_PIPELINE_DEFAULT_LAYOUT_BAD_MODULE,
        COMPUTE_PIPELINE_DEFAULT_LAYOUT_BAD_BGL_INDEX,
        RENDER_PIPELINE_DEFAULT_LAYOUT_BAD_MODULE,
        RENDER_PIPELINE_DEFAULT_LAYOUT_BAD_BGL_INDEX,
        NO_TARGETLESS_RENDER,
        COMPUTE_PIPELINE_WORKGROUP_LIMITS,
        COUNTED_DRAW_OFFSETS,
        RENDER_BUNDLE_INDIRECT_OFFSETS,
        RENDER_PASS_DRAW_LIMIT,
    ]);
}

#[gpu_test]
static COUNTED_DRAW_OFFSETS: GpuTestConfiguration = GpuTestConfiguration::new()
    .parameters(
        TestParameters::default()
            .limits(wgpu::Limits::downlevel_defaults())
            .enable_noop(),
    )
    .run_sync(|ctx| {
        let pipeline = draw_test_pipeline(&ctx.device);
        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let indices = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 4,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: false,
        });
        wgpu_test::valid(&ctx.device, || {
            let mut encoder = ctx.device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
                pass.set_pipeline(&pipeline);
                pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_counted(0, 1, 0, u32::MAX);
                pass.draw_counted(1, 1, u32::MAX, 0);
                pass.draw_indexed_counted(0, 1, 0, 0, u32::MAX);
                let mut bundle =
                    ctx.device
                        .create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
                            color_formats: &[Some(wgpu::TextureFormat::Rgba8Unorm)],
                            sample_count: 1,
                            ..Default::default()
                        });
                bundle.set_pipeline(&pipeline);
                bundle.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
                bundle.draw_counted(0, 1, 0, u32::MAX);
                bundle.draw_counted(1, 1, u32::MAX, 0);
                bundle.draw_indexed_counted(0, 1, 0, 0, u32::MAX);
                let bundle = bundle.finish(&Default::default());
                pass.execute_bundles([&bundle]);
            }
            ctx.queue.submit([encoder.finish()]);
        });
    });

fn draw_test_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl("@vertex fn vs()->@builtin(position) vec4f{return vec4f(0,0,0,1);} @fragment fn fs()->@location(0) vec4f{return vec4f(1,0,0,1);}".into()),
        });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: None,
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}

#[gpu_test]
static RENDER_BUNDLE_INDIRECT_OFFSETS: GpuTestConfiguration = GpuTestConfiguration::new()
    .parameters(
        TestParameters::default()
            .limits(wgpu::Limits::downlevel_defaults())
            .enable_noop(),
    )
    .run_sync(|ctx| {
        let pipeline = draw_test_pipeline(&ctx.device);
        let indices = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 4,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: false,
        });
        for indexed in [false, true] {
            let stride = if indexed { 20 } else { 16 };
            for size in [0, 4, 12, 16, 20, 24] {
                let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size,
                    usage: wgpu::BufferUsages::INDIRECT,
                    mapped_at_creation: false,
                });
                for offset in [0, 2, 4, u64::MAX - 3] {
                    let invalid =
                        offset % 4 != 0 || size < stride || offset > size.saturating_sub(stride);
                    fail_if(
                        &ctx.device,
                        invalid,
                        || {
                            let mut bundle = ctx.device.create_render_bundle_encoder(
                                &wgpu::RenderBundleEncoderDescriptor {
                                    color_formats: &[Some(wgpu::TextureFormat::Rgba8Unorm)],
                                    sample_count: 1,
                                    ..Default::default()
                                },
                            );
                            bundle.set_pipeline(&pipeline);
                            if indexed {
                                bundle
                                    .set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
                                bundle.draw_indexed_indirect(&buffer, offset);
                            } else {
                                bundle.draw_indirect(&buffer, offset);
                            }
                            bundle.finish(&Default::default())
                        },
                        None,
                    );
                }
            }
        }
    });

#[gpu_test]
static RENDER_PASS_DRAW_LIMIT: GpuTestConfiguration = GpuTestConfiguration::new()
    .parameters(
        TestParameters::default()
            .limits(wgpu::Limits::downlevel_defaults())
            .enable_noop(),
    )
    .run_sync(|ctx| {
        let pipeline = draw_test_pipeline(&ctx.device);
        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        for limit in [0, 1, 4] {
            for count in [0, limit, limit + 1] {
                for bundle_count in [0, 1, 2] {
                    fail_if(
                        &ctx.device,
                        count > limit,
                        || {
                            let bundles: Vec<_> = (0..bundle_count)
                                .map(|_| {
                                    let mut bundle = ctx.device.create_render_bundle_encoder(
                                        &wgpu::RenderBundleEncoderDescriptor {
                                            color_formats: &[Some(wgpu::TextureFormat::Rgba8Unorm)],
                                            sample_count: 1,
                                            ..Default::default()
                                        },
                                    );
                                    bundle.set_pipeline(&pipeline);
                                    for _ in 0..count {
                                        bundle.draw_counted(0, 1, 0, 0);
                                    }
                                    bundle.finish(&Default::default())
                                })
                                .collect();
                            let mut encoder =
                                ctx.device.create_command_encoder(&Default::default());
                            {
                                let mut pass =
                                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        color_attachments: &[Some(
                                            wgpu::RenderPassColorAttachment {
                                                view: &view,
                                                depth_slice: None,
                                                resolve_target: None,
                                                ops: wgpu::Operations {
                                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                                                    store: wgpu::StoreOp::Store,
                                                },
                                            },
                                        )],
                                        max_draw_count: Some(limit * bundle_count.max(1)),
                                        ..Default::default()
                                    });
                                if bundle_count == 0 {
                                    pass.set_pipeline(&pipeline);
                                    for _ in 0..count {
                                        pass.draw_counted(0, 1, 0, 0);
                                    }
                                } else {
                                    pass.execute_bundles(&bundles);
                                }
                            }
                            encoder.finish()
                        },
                        None,
                    );
                }
            }
        }
    });

#[gpu_test]
static COMPUTE_PIPELINE_WORKGROUP_LIMITS: GpuTestConfiguration = GpuTestConfiguration::new()
    .parameters(TestParameters::default().limits(wgpu::Limits::downlevel_defaults()).enable_noop())
    .run_sync(|ctx| {
        let limits = ctx.device.limits();
        let create = |code: String, entry: &str, constants: &[(&str, f64)], invalid: bool| {
            let module = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None, source: wgpu::ShaderSource::Wgsl(code.into()),
            });
            fail_if(&ctx.device, invalid, || ctx.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None, layout: None, module: &module, entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions { constants, ..Default::default() }, cache: None,
            }), None);
        };
        let count = limits.max_compute_workgroup_storage_size / 16;
        for delta in [0, 1] {
            let code = format!("var<workgroup> data:array<vec4f,{}>; fn helper(){{_ = data;}} @compute @workgroup_size(1) fn main(){{helper();}} @compute @workgroup_size(1) fn empty(){{}}", count + delta);
            create(code.clone(), "empty", &[], false);
            create(code, "main", &[], delta != 0);
            create("override count:u32; var<workgroup> data:array<vec4f,count>; @compute @workgroup_size(1) fn main(){_ = data[0];}".into(), "main", &[("count", f64::from(count + delta))], delta != 0);
            let scalars = (limits.max_compute_workgroup_storage_size - 16) / 4 + delta;
            create(format!("var<workgroup> data:array<f32,{scalars}>; var<workgroup> tail:f32; @compute @workgroup_size(1) fn main(){{_ = data; _ = tail;}}"), "main", &[], delta != 0);
        }
        let code = "override x:u32=1; override y:u32=1; override z:u32=1; @compute @workgroup_size(x,y,z) fn main(){}";
        for (name, limit) in [("x",limits.max_compute_workgroup_size_x),("y",limits.max_compute_workgroup_size_y),("z",limits.max_compute_workgroup_size_z)] {
            create(code.into(), "main", &[(name,f64::from(limit))], false);
            create(code.into(), "main", &[(name,f64::from(limit + 1))], true);
        }
        create(code.into(), "main", &[("x",16.),("y",17.)], true);
    });

const INVALID_SHADER_DESC: wgpu::ShaderModuleDescriptor = wgpu::ShaderModuleDescriptor {
    label: Some("invalid shader"),
    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed("not valid wgsl")),
};

const TRIVIAL_COMPUTE_SHADER_DESC: wgpu::ShaderModuleDescriptor = wgpu::ShaderModuleDescriptor {
    label: Some("trivial compute shader"),
    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
        "@compute @workgroup_size(1) fn main() {}",
    )),
};

const TRIVIAL_VERTEX_SHADER_DESC: wgpu::ShaderModuleDescriptor = wgpu::ShaderModuleDescriptor {
    label: Some("trivial vertex shader"),
    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
        "@vertex fn main() -> @builtin(position) vec4<f32> { return vec4<f32>(0); }",
    )),
};

const TRIVIAL_FRAGMENT_SHADER_DESC: wgpu::ShaderModuleDescriptor = wgpu::ShaderModuleDescriptor {
    label: Some("trivial fragment shader"),
    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
        "@fragment fn main() -> @location(0) vec4<f32> { return vec4<f32>(0); }",
    )),
};

// Create an invalid shader and a compute pipeline that uses it
// with a default bindgroup layout, and then ask for that layout.
// Validation should fail, but wgpu should not panic.
#[gpu_test]
static COMPUTE_PIPELINE_DEFAULT_LAYOUT_BAD_MODULE: GpuTestConfiguration =
    GpuTestConfiguration::new()
        .parameters(TestParameters::default().enable_noop())
        .run_sync(|ctx| {
            fail(
                &ctx.device,
                || {
                    let module = ctx.device.create_shader_module(INVALID_SHADER_DESC);

                    let pipeline =
                        ctx.device
                            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                                label: Some("compute pipeline"),
                                layout: None,
                                module: &module,
                                entry_point: Some("doesn't exist"),
                                compilation_options: Default::default(),
                                cache: None,
                            });

                    // https://github.com/gfx-rs/wgpu/issues/4167 this used to panic
                    pipeline.get_bind_group_layout(0);
                },
                Some("Shader 'invalid shader' parsing error"),
            );
        });

#[gpu_test]
static COMPUTE_PIPELINE_DEFAULT_LAYOUT_BAD_BGL_INDEX: GpuTestConfiguration =
    GpuTestConfiguration::new()
        .parameters(
            TestParameters::default()
                .test_features_limits()
                .enable_noop(),
        )
        .run_sync(|ctx| {
            fail(
                &ctx.device,
                || {
                    let module = ctx.device.create_shader_module(TRIVIAL_COMPUTE_SHADER_DESC);

                    let pipeline =
                        ctx.device
                            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                                label: Some("compute pipeline"),
                                layout: None,
                                module: &module,
                                entry_point: Some("main"),
                                compilation_options: Default::default(),
                                cache: None,
                            });

                    pipeline.get_bind_group_layout(u32::MAX);
                },
                Some("Bind group layout index 4294967295 is greater than the device's configured `max_bind_groups` limit"),
            );
        });

#[gpu_test]
static RENDER_PIPELINE_DEFAULT_LAYOUT_BAD_MODULE: GpuTestConfiguration =
    GpuTestConfiguration::new()
        .parameters(TestParameters::default().enable_noop())
        .run_sync(|ctx| {
            fail(
                &ctx.device,
                || {
                    let module = ctx.device.create_shader_module(INVALID_SHADER_DESC);

                    let pipeline =
                        ctx.device
                            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                                label: Some("render pipeline"),
                                layout: None,
                                vertex: wgpu::VertexState {
                                    module: &module,
                                    entry_point: Some("doesn't exist"),
                                    compilation_options: Default::default(),
                                    buffers: &[],
                                },
                                primitive: Default::default(),
                                depth_stencil: None,
                                multisample: Default::default(),
                                fragment: None,
                                multiview_mask: None,
                                cache: None,
                            });

                    pipeline.get_bind_group_layout(0);
                },
                Some("Shader 'invalid shader' parsing error"),
            );
        });

#[gpu_test]
static RENDER_PIPELINE_DEFAULT_LAYOUT_BAD_BGL_INDEX: GpuTestConfiguration =
    GpuTestConfiguration::new()
        .parameters(
            TestParameters::default()
                .test_features_limits()
                .enable_noop(),
        )
        .run_sync(|ctx| {
            fail(
                &ctx.device,
                || {
                    let vs_module = ctx.device.create_shader_module(TRIVIAL_VERTEX_SHADER_DESC);
                    let fs_module = ctx
                        .device
                        .create_shader_module(TRIVIAL_FRAGMENT_SHADER_DESC);

                    let pipeline =
                        ctx.device
                            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                                label: Some("render pipeline"),
                                layout: None,
                                vertex: wgpu::VertexState {
                                    module: &vs_module,
                                    entry_point: Some("main"),
                                    compilation_options: Default::default(),
                                    buffers: &[],
                                },
                                primitive: Default::default(),
                                depth_stencil: None,
                                multisample: Default::default(),
                                fragment: Some(wgpu::FragmentState {
                                    module: &fs_module,
                                    entry_point: Some("main"),
                                    compilation_options: Default::default(),
                                    targets: &[Some(wgpu::ColorTargetState {
                                        format: wgpu::TextureFormat::Rgba8Unorm,
                                        blend: None,
                                        write_mask: wgpu::ColorWrites::ALL,
                                    })],
                                }),
                                multiview_mask: None,
                                cache: None,
                            });

                    pipeline.get_bind_group_layout(u32::MAX);
                },
                Some("Bind group layout index 4294967295 is greater than the device's configured `max_bind_groups` limit"),
            );
        });

#[gpu_test]
static NO_TARGETLESS_RENDER: GpuTestConfiguration = GpuTestConfiguration::new()
    .parameters(TestParameters::default().enable_noop())
    .run_sync(|ctx| {
        fail(
            &ctx.device,
            || {
                // Testing multisampling is important, because some backends don't behave well if one
                // tries to compile code in an unsupported multisample count. Failing to validate here
                // has historically resulted in requesting the back end to compile code.
                for power_of_two in [1, 2, 4, 8, 16, 32, 64] {
                    let _ = ctx
                        .device
                        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                            label: None,
                            layout: None,
                            vertex: wgpu::VertexState {
                                module: &ctx
                                    .device
                                    .create_shader_module(TRIVIAL_VERTEX_SHADER_DESC),
                                entry_point: Some("main"),
                                compilation_options: Default::default(),
                                buffers: &[],
                            },
                            primitive: Default::default(),
                            depth_stencil: None,
                            multisample: wgpu::MultisampleState {
                                count: power_of_two,
                                ..Default::default()
                            },
                            fragment: None,
                            multiview_mask: None,
                            cache: None,
                        });
                }
            },
            Some(concat!(
                "At least one color attachment or depth-stencil attachment was expected, ",
                "but no render target for the pipeline was specified."
            )),
        )
    });
