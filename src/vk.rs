
use std::sync::Arc;
use derive_more::*;


#[derive(Deref, DerefMut)]
pub struct Device{
    #[deref]
    #[deref_mut]
    device: Arc<vulkano::device::Device>,
    queues: Vec<Arc<vulkano::device::Queue>>,
}

impl Device{
}
