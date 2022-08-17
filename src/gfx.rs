use log::info;
use winit::window::Window;

pub struct Renderer {}

impl Renderer {
    pub(crate) fn new(window: &Window) -> Self {
        info!("Creating Renderer");
    }
}