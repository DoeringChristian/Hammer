// Copyright (c) 2016 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

// Welcome to the triangle example!
//
// This is the only example that is entirely detailed. All the other examples avoid code
// duplication by using helper functions.
//
// This example assumes that you are already more or less familiar with graphics programming
// and that you want to learn Vulkan. This means that for example it won't go into details about
// what a vertex or a shader is.


use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents},
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, QueueCreateInfo,
    },
    image::{view::{ImageView, ImageViewCreationError}, ImageAccess, ImageUsage, SwapchainImage, ImageViewAbstract},
    impl_vertex,
    instance::{Instance, InstanceCreateInfo},
    pipeline::{
        graphics::{
            input_assembly::InputAssemblyState,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    swapchain::{
        acquire_next_image, AcquireError, Swapchain, SwapchainCreateInfo, SwapchainCreationError, Surface, SwapchainAcquireFuture,
    },
    sync::{self, FlushError, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

mod vk;
mod hammer;

extern crate derive_more;
use derive_more::*;


fn main() {
    // The first step of any Vulkan program is to create an instance.
    //
    // When we create an instance, we have to pass a list of extensions that we want to enable.
    //
    // All the window-drawing functionalities are part of non-core extensions that we need
    // to enable manually. To do so, we ask the `vulkano_win` crate for the list of extensions
    // required to draw to a window.
    let required_extensions = vulkano_win::required_extensions();

    // Now creating the instance.
    let instance = Instance::new(InstanceCreateInfo {
        enabled_extensions: required_extensions,
        ..Default::default()
    })
    .unwrap();
    let instance = hammer::Instance::new(InstanceCreateInfo{
        enabled_extensions: required_extensions,
        ..Default::default()
    });

    // The objective of this example is to draw a triangle on a window. To do so, we first need to
    // create the window.
    //
    // This is done by creating a `WindowBuilder` from the `winit` crate, then calling the
    // `build_vk_surface` method provided by the `VkSurfaceBuild` trait from `vulkano_win`. If you
    // ever get an error about `build_vk_surface` being undefined in one of your projects, this
    // probably means that you forgot to import this trait.
    //
    // This returns a `vulkano::swapchain::Surface` object that contains both a cross-platform winit
    // window and a cross-platform Vulkan surface that represents the surface of the window.
    let event_loop = EventLoop::new();
    let mut surface = hammer::Surface::new(
        WindowBuilder::new().build(&event_loop).unwrap(),
        instance.clone(),
    );

    let desc = hammer::AdapterDescriptor{
        supports_surface: Some(&surface),
        ..hammer::AdapterDescriptor::graphics()
    };

    let adapter = instance.request_adapter(&desc);

    let (device, queue) = adapter.request_device(vulkano::device::Features::default());

    surface.create_swapchain(device.clone(), &adapter);

    // We now create a buffer that will store the shape of our triangle.
    // We use #[repr(C)] here to force rustc to not do anything funky with our data, although for this
    // particular example, it doesn't actually change the in-memory representation.
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
    struct Vertex {
        position: [f32; 2],
    }
    impl_vertex!(Vertex, position);

    let vertices = [
        Vertex {
            position: [-0.5, -0.25],
        },
        Vertex {
            position: [0.0, 0.5],
        },
        Vertex {
            position: [0.25, -0.1],
        },
    ];
    let vertex_buffer =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, vertices)
        .unwrap();

    // The next step is to create the shaders.
    //
    // The raw shader creation API provided by the vulkano library is unsafe, for various reasons.
    //
    // An overview of what the `shader!` macro generates can be found in the
    // `vulkano-shaders` crate docs. You can view them at https://docs.rs/vulkano-shaders/
    //
    // TODO: explain this in details
    mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            src: "
                                #version 450
                                layout(location = 0) in vec2 position;
                                void main() {
                                        gl_Position = vec4(position, 0.0, 1.0);
                                }
                        "
        }
    }

    mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            src: "
                #version 450
                layout(location = 0) out vec4 f_color;
            void main() {
                f_color = vec4(1.0, 0.0, 0.0, 1.0);
            }
                        "
        }
    }

    let vs = vs::load(device.clone()).unwrap();
    let fs = fs::load(device.clone()).unwrap();

    // At this point, OpenGL initialization would be finished. However in Vulkan it is not. OpenGL
    // implicitly does a lot of computation whenever you draw. In Vulkan, you have to do all this
    // manually.


    // The next step is to create a *render pass*, which is an object that describes where the
    // output of the graphics pipeline will go. It describes the layout of the images
    // where the colors, depth and/or stencil information will be written.
    let render_pass = vulkano::single_pass_renderpass!(
        device.clone(),
        attachments: {
            // `color` is a custom name we give to the first and only attachment.
            color: {
                // `load: Clear` means that we ask the GPU to clear the content of this
                // attachment at the start of the drawing.
                load: Clear,
                // `store: Store` means that we ask the GPU to store the output of the draw
                // in the actual image. We could also ask it to discard the result.
                store: Store,
                // `format: <ty>` indicates the type of the format of the image. This has to
                // be one of the types of the `vulkano::format` module (or alternatively one
                // of your structs that implements the `FormatDesc` trait). Here we use the
                // same format as the swapchain.
                format: surface.image_format().unwrap(),
                // TODO:
                samples: 1,
                }
        },
        pass: {
            // We use the attachment named `color` as the one and only color attachment.
            color: [color],
            // No depth-stencil attachment is indicated with empty brackets.
            depth_stencil: {}
        }
    )
        .unwrap();

            // Before we draw we have to create what is called a pipeline. This is similar to an OpenGL
            // program, but much more specific.
            let pipeline = GraphicsPipeline::start()
                // We need to indicate the layout of the vertices.
                .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
                // A Vulkan shader can in theory contain multiple entry points, so we have to specify
                // which one.
                .vertex_shader(vs.entry_point("main").unwrap(), ())
                // The content of the vertex buffer describes a list of triangles.
                .input_assembly_state(InputAssemblyState::new())
                // Use a resizable viewport set to draw over the entire window
                .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
                // See `vertex_shader`.
                .fragment_shader(fs.entry_point("main").unwrap(), ())
                // We have to indicate which subpass of which render pass this pipeline is going to be used
                // in. The pipeline will only be usable from this particular subpass.
                .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
                // Now that our builder is filled, we call `build()` to obtain an actual pipeline.
                .build(device.clone())
                .unwrap();

            // Dynamic viewports allow us to recreate just the viewport when the window is resized
            // Otherwise we would have to recreate the whole pipeline.
            let mut viewport = Viewport {
                origin: [0.0, 0.0],
                dimensions: [0.0, 0.0],
                depth_range: 0.0..1.0,
            };

            // The render pass we created above only describes the layout of our framebuffers. Before we
            // can draw we also need to create the actual framebuffers.
            //
            // Since we need to draw to multiple images, we are going to create a different framebuffer for
            // each image.
            //let mut framebuffers = window_size_dependent_setup(&images, render_pass.clone(), &mut viewport);

            // Initialization is finally finished!

            // In some situations, the swapchain will become invalid by itself. This includes for example
            // when the window is resized (as the images of the swapchain will no longer match the
            // window's) or, on Android, when the application went to the background and goes back to the
            // foreground.
            //
            // In this situation, acquiring a swapchain image or presenting it will return an error.
            // Rendering to an image of that swapchain will not produce any error, but may or may not work.
            // To continue rendering, we need to recreate the swapchain by creating a new swapchain.
            // Here, we remember that we need to do this for the next loop iteration.
            let mut recreate_swapchain = false;

            // In the loop below we are going to submit commands to the GPU. Submitting a command produces
            // an object that implements the `GpuFuture` trait, which holds the resources for as long as
            // they are in use by the GPU.
            //
            // Destroying the `GpuFuture` blocks until the GPU is finished executing it. In order to avoid
            // that, we store the submission of the previous frame here.
            let mut previous_frame_end = Some(sync::now(device.clone()).boxed());

            event_loop.run(move |event, _, control_flow| {
                match event {
                    Event::WindowEvent {
                        event: WindowEvent::CloseRequested,
                        ..
                    } => {
                        *control_flow = ControlFlow::Exit;
                    }
                    Event::WindowEvent {
                        event: WindowEvent::Resized(_),
                        ..
                    } => {
                        recreate_swapchain = true;
                    }
                    Event::RedrawEventsCleared => {
                        // It is important to call this function from time to time, otherwise resources will keep
                        // accumulating and you will eventually reach an out of memory error.
                        // Calling this function polls various fences in order to determine what the GPU has
                        // already processed, and frees the resources that are no longer needed.
                        previous_frame_end.as_mut().unwrap().cleanup_finished();

                        // Whenever the window resizes we need to recreate everything dependent on the window size.
                        // In this example that includes the swapchain, the framebuffers and the dynamic state viewport.
                        if recreate_swapchain {
                            surface.recreate_swapchain();
                            recreate_swapchain = false;
                        }

                        //framebuffers = window_size_dependent_setup(&images, render_pass.clone(), &mut viewport);

                        // Before we can draw on the output, we have to *acquire* an image from the swapchain. If
                        // no image is available (which happens if you submit draw commands too quickly), then the
                        // function will block.
                        // This operation returns the index of the image that we are allowed to draw upon.
                        //
                        // This function can block if no image is available. The parameter is an optional timeout
                        // after which the function call will return an error.
                        /*
                        let (image_num, suboptimal, acquire_future) =
                            match acquire_next_image(swapchain.clone(), None) {
                                Ok(r) => r,
                                Err(AcquireError::OutOfDate) => {
                                    recreate_swapchain = true;
                                    return;
                                }
                                Err(e) => panic!("Failed to acquire next image: {:?}", e),
                            };
                        */
                        let target_image = surface.get_current_image();
                        let framebuffer = target_image.framebuffer_setup(render_pass.clone(), &mut viewport);

                        // acquire_next_image can be successful, but suboptimal. This means that the swapchain image
                        // will still work, but it may not display correctly. With some drivers this can be when
                        // the window resizes, but it may not cause the swapchain to become out of date.
                        if target_image.suboptimal {
                            recreate_swapchain = true;
                        }

                        // Specify the color to clear the framebuffer with i.e. blue
                        let clear_values = vec![[0.0, 0.0, 1.0, 1.0].into()];

                        // In order to draw, we have to build a *command buffer*. The command buffer object holds
                        // the list of commands that are going to be executed.
                        //
                        // Building a command buffer is an expensive operation (usually a few hundred
                        // microseconds), but it is known to be a hot path in the driver and is expected to be
                        // optimized.
                        //
                        // Note that we have to pass a queue family when we create the command buffer. The command
                        // buffer will only be executable on that given queue family.
                        let mut builder = AutoCommandBufferBuilder::primary(
                            device.clone(),
                            queue.family(),
                            CommandBufferUsage::OneTimeSubmit,
                        )
                            .unwrap();

                        builder
                            // Before we can draw, we have to *enter a render pass*. There are two methods to do
                            // this: `draw_inline` and `draw_secondary`. The latter is a bit more advanced and is
                            // not covered here.
                            //
                            // The third parameter builds the list of values to clear the attachments with. The API
                            // is similar to the list of attachments when building the framebuffers, except that
                            // only the attachments that use `load: Clear` appear in the list.
                            .begin_render_pass(
                                framebuffer.clone(),
                                SubpassContents::Inline,
                                clear_values,
                            )
                            .unwrap()
                            // We are now inside the first subpass of the render pass. We add a draw command.
                            //
                            // The last two parameters contain the list of resources to pass to the shaders.
                            // Since we used an `EmptyPipeline` object, the objects have to be `()`.
                            .set_viewport(0, [viewport.clone()])
                            .bind_pipeline_graphics(pipeline.clone())
                            .bind_vertex_buffers(0, vertex_buffer.clone())
                            .draw(vertex_buffer.len() as u32, 1, 0, 0)
                            .unwrap()
                            // We leave the render pass by calling `draw_end`. Note that if we had multiple
                            // subpasses we could have called `next_inline` (or `next_secondary`) to jump to the
                            // next subpass.
                            .end_render_pass()
                            .unwrap();

                        // Finish building the command buffer by calling `build`.
                        let command_buffer = builder.build().unwrap();

                        let future = previous_frame_end
                            .take()
                            .unwrap()
                            .join(target_image.acquire_future)
                            .then_execute(queue.clone(), command_buffer)
                            .unwrap()
                            // The color output is now expected to contain our triangle. But in order to show it on
                            // the screen, we have to *present* the image by calling `present`.
                            //
                            // This function does not actually present the image immediately. Instead it submits a
                            // present command at the end of the queue. This means that it will only be presented once
                            // the GPU has finished executing the command buffer that draws the triangle.
                            .then_swapchain_present(queue.clone(), surface.swapchain.as_ref().unwrap().swapchain.clone(), target_image.image_num)
                            .then_signal_fence_and_flush();

                        match future {
                            Ok(future) => {
                                previous_frame_end = Some(future.boxed());
                            }
                            Err(FlushError::OutOfDate) => {
                                recreate_swapchain = true;
                                previous_frame_end = Some(sync::now(device.clone()).boxed());
                            }
                            Err(e) => {
                                println!("Failed to flush future: {:?}", e);
                                previous_frame_end = Some(sync::now(device.clone()).boxed());
                            }
                            }
                    }
                    _ => (),
                }
            });
}

/// This method is called once during initialization, then again whenever the window is resized
fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> Vec<Arc<Framebuffer>> {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            let mut attachments: Vec<Arc<dyn ImageViewAbstract>> = Vec::new();
            attachments.push(view);
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments,
                    ..Default::default()
                },
            )
                .unwrap()
        })
    .collect::<Vec<_>>()
}
