use derive_more::*;
use std::sync::Arc;

// Getting rust analyzer problems when not defining the module here again.
mod vulkano {
    pub use vulkano::device::physical::*;
    pub use vulkano::device::*;
    pub use vulkano::image::view::*;
    pub use vulkano::image::*;
    pub use vulkano::instance::*;
    pub use vulkano::pipeline::graphics::viewport::*;
    pub use vulkano::render_pass::*;
    pub use vulkano::swapchain::*;
    pub use vulkano::*;
}

#[derive(Deref, DerefMut)]
pub struct Instance {
    instance: Arc<vulkano::Instance>,
}

impl Instance {
    pub fn new(info: vulkano::InstanceCreateInfo) -> Self{
        Self{
            instance: vulkano::Instance::new(info).unwrap(),
        }
    }
    pub fn request_adapter<'a, 'ad, W>(&'a self, desc: &AdapterDescriptor<'ad, W>) -> Adapter<'a> {
        let (physical_device, queue_family) = vulkano::PhysicalDevice::enumerate(&self.instance)
            .filter(|&p| {
                p.supported_extensions()
                    .is_superset_of(&desc.device_extensions)
            })
            .filter_map(|p| {
                p.queue_families()
                    .find(|&q| desc.compatible(&q))
                    .map(|q| (p, q))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                vulkano::PhysicalDeviceType::DiscreteGpu => 0,
                vulkano::PhysicalDeviceType::IntegratedGpu => 1,
                vulkano::PhysicalDeviceType::VirtualGpu => 2,
                vulkano::PhysicalDeviceType::Cpu => 3,
                vulkano::PhysicalDeviceType::Other => 4,
            })
            .unwrap();
        println!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        Adapter {
            physical_device,
            queue_family,
            device_extensions: desc.device_extensions,
        }
    }
}


pub struct AdapterDescriptor<'ad, W> {
    pub device_extensions: vulkano::DeviceExtensions,
    pub supports_graphics: bool,
    pub supports_compute: bool,
    pub supports_surface: Option<&'ad vulkano::Surface<W>>,
}

impl<'ad, W> AdapterDescriptor<'ad, W> {
    fn compatible(&self, queue_family: &vulkano::QueueFamily) -> bool {
        if self.supports_graphics && !queue_family.supports_graphics() {
            return false;
        }
        if self.supports_compute && !queue_family.supports_compute() {
            return false;
        }
        if let Some(surface) = self.supports_surface {
            if !queue_family.supports_surface(&surface).unwrap_or(false) {
                return false;
            }
        }

        return true;
    }
    pub fn graphics() -> Self{
        AdapterDescriptor{
            device_extensions: vulkano::DeviceExtensions{
                khr_swapchain: true,
                ..vulkano::DeviceExtensions::none()
            },
            supports_graphics: true,
            supports_surface: None,
            supports_compute: false,
        }
    }
}

pub struct Adapter<'a> {
    pub physical_device: vulkano::PhysicalDevice<'a>,
    pub queue_family: vulkano::QueueFamily<'a>,
    device_extensions: vulkano::DeviceExtensions,
}

impl<'a> Adapter<'a> {
    pub fn request_device(
        &self,
        features: vulkano::Features,
    ) -> (Arc<vulkano::Device>, Arc<vulkano::Queue>) {
        let (device, mut queues) = vulkano::Device::new(
            // Which physical device to connect to.
            self.physical_device,
            vulkano::DeviceCreateInfo {
                // A list of optional features and extensions that our program needs to work correctly.
                // Some parts of the Vulkan specs are optional and must be enabled manually at device
                // creation. In this example the only thing we are going to need is the `khr_swapchain`
                // extension that allows us to draw to a window.
                enabled_extensions: self
                    .physical_device
                    // Some devices require certain extensions to be enabled if they are present
                    // (e.g. `khr_portability_subset`). We add them to the device extensions that we're
                    // going to enable.
                    .required_extensions()
                    .union(&self.device_extensions),

                // The list of queues that we are going to use. Here we only use one queue, from the
                // previously chosen queue family.
                queue_create_infos: vec![vulkano::QueueCreateInfo::family(self.queue_family)],

                enabled_features: features,

                ..Default::default()
            },
        )
        .unwrap();
        let queue = queues.next().unwrap();

        (device, queue)
    }
}



pub trait GetPhysicalDevice{
    fn get_physical_device(&self) -> &vulkano::PhysicalDevice;
}

impl<'a> GetPhysicalDevice for &Adapter<'a>{
    fn get_physical_device(&self) -> &vulkano::PhysicalDevice {
        &self.physical_device
    }
}

impl<'p> GetPhysicalDevice for &vulkano::PhysicalDevice<'p>{
    fn get_physical_device(&self) -> &vulkano::PhysicalDevice {
        self
    }
}
