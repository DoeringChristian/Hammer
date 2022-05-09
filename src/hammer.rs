
use std::sync::Arc;
use derive_more::*;

mod vulkano{
    pub use vulkano::*;
    pub use vulkano::device::*;
    pub use vulkano::image::*;
    pub use vulkano::image::view::*;
    pub use vulkano::device::physical::*;
    pub use vulkano::swapchain::*;
    pub use vulkano::render_pass::*;
    pub use vulkano::pipeline::graphics::viewport::*;
}

pub struct TargetSurface<W>{
    pub device: Arc<vulkano::Device>,
    pub surface: Arc<vulkano::Surface<W>>,
    pub swapchain: Arc<vulkano::Swapchain<W>>,
    pub images: Vec<Arc<vulkano::SwapchainImage<W>>>,
}

pub trait WithInnerIsize{
    fn inner_size(&self) -> [u32; 2];
}

impl WithInnerIsize for winit::window::Window{
    fn inner_size(&self) -> [u32; 2] {
        self.inner_size().into()
    }
}

impl<W: WithInnerIsize> TargetSurface<W>{
    pub fn new(
        device: Arc<vulkano::Device>, 
        pdevice: &vulkano::PhysicalDevice, 
        surface: Arc<vulkano::Surface<W>>
    ) -> Self{
        let (swapchain, images) = {
            let surface_capabilities = pdevice
                .surface_capabilities(&surface, Default::default())
                .unwrap();

            let image_format = Some(
                pdevice
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0,
            );

            vulkano::Swapchain::new(
                device.clone(),
                surface.clone(),
                vulkano::SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count,

                    image_format,
                    image_extent: surface.window().inner_size().into(),

                    image_usage: vulkano::ImageUsage::color_attachment(),

                    composite_alpha: surface_capabilities
                        .supported_composite_alpha
                        .iter()
                        .next()
                        .unwrap(),

                        ..Default::default()
                },
                )
                    .unwrap()
        };

        Self{
            images,
            swapchain,
            device,
            surface,
        }
    }
    pub fn recreate(&mut self) -> bool{
        let (new_swapchain, new_images) = 
            match self.swapchain.recreate(vulkano::SwapchainCreateInfo{
                image_extent: self.surface.window().inner_size().into(),
                ..self.swapchain.create_info()
            }){
                Ok(r) => r,
                Err(vulkano::SwapchainCreationError::ImageExtentNotSupported{..}) => return false,
                Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
            };

        self.swapchain = new_swapchain;
        self.images = new_images;

        true
    }
    pub fn get_current_image(&self) -> SurfaceImage<W>{

        let (image_num, suboptimal, acquire_future) = 
            match vulkano::acquire_next_image(self.swapchain.clone(), None){
                Ok(r) => r,
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };

        SurfaceImage{
            image: self.images[image_num].clone(),
            suboptimal,
            acquire_future,
            image_num,
        }
    }
}

#[derive(Deref, DerefMut)]
pub struct SurfaceImage<W>{
    #[deref]
    #[deref_mut]
    pub image: Arc<vulkano::SwapchainImage<W>>,
    pub suboptimal: bool,
    pub acquire_future: vulkano::SwapchainAcquireFuture<W>,
    pub image_num: usize, 
}

impl<W: 'static + Send + Sync> SurfaceImage<W>{
    pub fn create_view_default(&self) -> Result<Arc<vulkano::ImageView<vulkano::SwapchainImage<W>>>, vulkano::ImageViewCreationError>{
        vulkano::ImageView::new_default(self.image.clone())
    }
    pub fn framebuffer_setup(&self, render_pass: Arc<vulkano::RenderPass>, viewport: &mut vulkano::Viewport) -> Arc<vulkano::Framebuffer>{
        let dimensions = vulkano::ImageAccess::dimensions(&self.image).width_height();
        viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

        let view = self.create_view_default().unwrap();

        let mut attachments: Vec<Arc<dyn vulkano::ImageViewAbstract>> = Vec::new();
        attachments.push(view);
        vulkano::Framebuffer::new(
            render_pass,
            vulkano::FramebufferCreateInfo{
                attachments,
                ..Default::default()
            },
        ).unwrap()
    }
}
