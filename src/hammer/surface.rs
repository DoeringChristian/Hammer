
use std::sync::Arc;
use derive_more::*;

// Getting rust analyzer problems when not defining the module here again.
mod vulkano{
    pub use vulkano::*;
    pub use vulkano::instance::*;
    pub use vulkano::device::*;
    pub use vulkano::image::*;
    pub use vulkano::image::view::*;
    pub use vulkano::device::physical::*;
    pub use vulkano::swapchain::*;
    pub use vulkano::render_pass::*;
    pub use vulkano::pipeline::graphics::viewport::*;
}
use super::*;

#[derive(Deref, DerefMut)]
pub struct Swapchain<W>{
    pub device: Arc<vulkano::Device>,
    #[deref]
    #[deref_mut]
    pub swapchain: Arc<vulkano::Swapchain<W>>,
    pub images: Vec<Arc<vulkano::SwapchainImage<W>>>,
}

#[derive(Deref, DerefMut)]
pub struct Surface<W>{
    #[deref]
    #[deref_mut]
    pub surface: Arc<vulkano::Surface<W>>,
    pub swapchain: Option<Swapchain<W>>,
}

pub trait WithInnerIsize{
    fn inner_size(&self) -> [u32; 2];
}

impl WithInnerIsize for winit::window::Window{
    fn inner_size(&self) -> [u32; 2] {
        self.inner_size().into()
    }
}

impl Surface<winit::window::Window>{
    pub fn new(window: winit::window::Window, instance: Arc<vulkano::Instance>) -> Surface<winit::window::Window>{
        let surface = vulkano_win::create_surface_from_winit(window, instance).unwrap();
        Surface{
            surface,
            swapchain: None,
        }
    }
}

impl<W: WithInnerIsize> Surface<W>{
    pub fn create_swapchain(
        &mut self, 
        device: Arc<vulkano::Device>, 
        pdevice: &vulkano::PhysicalDevice
    ) -> bool{
        let (swapchain, images) = {
            let surface_capabilities = pdevice
                .surface_capabilities(&self.surface, Default::default())
                .unwrap();

            let image_format = Some(
                pdevice
                .surface_formats(&self.surface, Default::default())
                .unwrap()[0]
                .0,
            );

            vulkano::Swapchain::new(
                device.clone(),
                self.surface.clone(),
                vulkano::SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count,

                    image_format,
                    image_extent: self.surface.window().inner_size().into(),

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
        self.swapchain = Some(
            Swapchain{
                device,
                swapchain,
                images,
            }
        );
        true
    }
    pub fn recreate_swapchain(&mut self) -> bool{
        match self.swapchain{
            Some(ref mut swapchain) => {
                let (new_swapchain, new_images) = 
                    match swapchain.recreate(vulkano::SwapchainCreateInfo{
                        image_extent: self.surface.window().inner_size().into(),
                        ..swapchain.create_info()
                    }){
                        Ok(r) => r,
                        Err(vulkano::SwapchainCreationError::ImageExtentNotSupported{..}) => return false,
                        Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                    };
                swapchain.swapchain = new_swapchain;
                swapchain.images = new_images;
                true
            },
            _ => false,
        }
    }
    pub fn get_current_image(&self) -> SurfaceImage<W>{

        let (image_num, suboptimal, acquire_future) = 
            match vulkano::acquire_next_image(self.swapchain.as_ref().unwrap().swapchain.clone(), None){
                Ok(r) => r,
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };

        SurfaceImage{
            image: self.swapchain.as_ref().unwrap().images[image_num].clone(),
            suboptimal,
            acquire_future,
            image_num,
        }
    }
    pub fn image_format(&self) -> Option<vulkano::format::Format>{
        Some(self.swapchain.as_ref()?.image_format())
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
