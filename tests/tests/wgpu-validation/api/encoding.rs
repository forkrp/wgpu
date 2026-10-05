//! Tests of [`wgpu::CommandEncoder`] and related.

#[test]
fn abandoned_pass_does_not_end_or_report_late_errors() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let texture = device.create_texture(&wgpu::TextureDescriptor {
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
    for render in [false, true] {
        for finish_first in [false, true] {
            let mut encoder = device.create_command_encoder(&Default::default());
            let render_pass = render.then(|| {
                encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    })
                    .forget_lifetime()
            });
            let compute_pass = (!render).then(|| {
                encoder
                    .begin_compute_pass(&Default::default())
                    .forget_lifetime()
            });
            if finish_first {
                let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
                let _commands = encoder.finish();
                assert!(pollster::block_on(scope.pop()).is_some());
                let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
                if let Some(pass) = render_pass {
                    pass.abandon();
                }
                if let Some(pass) = compute_pass {
                    pass.abandon();
                }
                assert!(pollster::block_on(scope.pop()).is_none());
            } else {
                let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
                if let Some(pass) = render_pass {
                    pass.abandon();
                }
                if let Some(pass) = compute_pass {
                    pass.abandon();
                }
                assert!(pollster::block_on(scope.pop()).is_none());
                let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
                let _commands = encoder.finish();
                assert!(pollster::block_on(scope.pop()).is_some());
            }
        }
    }
}

#[test]
fn destroyed_resources_preserve_construction_validation() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 64,
        usage: wgpu::BufferUsages::UNIFORM,
        mapped_at_creation: false,
    });
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
        ],
    });
    buffer.destroy();
    texture.destroy();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let view = texture.create_view(&Default::default());
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
        ],
    });
    assert!(pollster::block_on(scope.pop()).is_none());
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    texture.create_view(&wgpu::TextureViewDescriptor {
        base_mip_level: 1,
        ..Default::default()
    });
    assert!(pollster::block_on(scope.pop()).is_some());
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 256,
                    size: None,
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
        ],
    });
    assert!(pollster::block_on(scope.pop()).is_some());
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_bind_group(0, &group, &[]);
    }
    queue.submit([encoder.finish()]);
    assert!(pollster::block_on(scope.pop()).is_some());
}

#[test]
fn empty_render_buffer_slices_preserve_ranges() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl("@vertex fn vs(@location(0) x: f32) -> @builtin(position) vec4f { return vec4f(x,0,0,1); } @fragment fn fs() -> @location(0) vec4f { return vec4f(1); }".into()),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: 4,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 0,
                    shader_location: 0,
                }],
            })],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
        }),
        multiview_mask: None,
        cache: None,
    });
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 16,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::INDEX,
        mapped_at_creation: false,
    });
    let texture = device.create_texture(&wgpu::TextureDescriptor {
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
    for bundle in [false, true] {
        for indexed in [false, true] {
            for count in [0, 1] {
                let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
                if bundle {
                    let mut encoder =
                        device.create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
                            color_formats: &[Some(wgpu::TextureFormat::Rgba8Unorm)],
                            sample_count: 1,
                            ..Default::default()
                        });
                    encoder.set_pipeline(&pipeline);
                    if indexed {
                        encoder.set_vertex_buffer(0, buffer.slice(..));
                        encoder.set_index_buffer(buffer.slice(4..4), wgpu::IndexFormat::Uint32);
                        encoder.draw_indexed(0..count, 0, 0..1);
                    } else {
                        encoder.set_vertex_buffer(0, buffer.slice(4..4));
                        encoder.draw(0..count, 0..1);
                    }
                    let _bundle = encoder.finish(&Default::default());
                } else {
                    let mut encoder = device.create_command_encoder(&Default::default());
                    {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                depth_slice: None,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            ..Default::default()
                        });
                        pass.set_pipeline(&pipeline);
                        if indexed {
                            pass.set_vertex_buffer(0, buffer.slice(..));
                            pass.set_index_buffer(buffer.slice(4..4), wgpu::IndexFormat::Uint32);
                            pass.draw_indexed(0..count, 0, 0..1);
                        } else {
                            pass.set_vertex_buffer(0, buffer.slice(4..4));
                            pass.draw(0..count, 0..1);
                        }
                    }
                    let commands = encoder.finish();
                    if count == 0 {
                        queue.submit([commands]);
                    }
                }
                let error = pollster::block_on(scope.pop());
                assert_eq!(
                    error.is_some(),
                    count != 0,
                    "bundle={bundle}, indexed={indexed}, count={count}, error={error:?}"
                );
            }
        }
    }
}

#[test]
fn as_hal() {
    // Sanity-test that the raw encoding API isn't completely broken.

    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    unsafe {
        encoder.as_hal_mut::<wgpu_hal::api::Noop, _, ()>(|_| ());
    }
    encoder.finish();
}

#[test]
#[should_panic = "Mixing the wgpu encoding API with the raw encoding API is not permitted"]
fn mix_apis_wgpu_then_hal() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());

    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.clear_buffer(&buffer, 0, None);
    unsafe {
        encoder.as_hal_mut::<wgpu_hal::api::Noop, _, ()>(|_| ());
    }
}

#[test]
#[should_panic = "Mixing the wgpu encoding API with the raw encoding API is not permitted"]
fn mix_apis_hal_then_wgpu() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());

    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    unsafe {
        encoder.as_hal_mut::<wgpu_hal::api::Noop, _, ()>(|_| ());
    }
    encoder.clear_buffer(&buffer, 0, None);
}

/// Test that the command encoder’s label is remembered and used in errors.
#[test]
#[should_panic = "In a CommandEncoder, label = 'my encoder'"]
fn encoding_error_contains_label_of_encoder() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("my buffer"),
        size: 1024,
        usage: wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("my encoder"),
    });
    // This is erroneous because it is copying to the same buffer.
    encoder.copy_buffer_to_buffer(&buffer, 0, &buffer, 0, 10);
    queue.submit([encoder.finish()]);
}
