use log::{info, warn};
use winit::{
    event::Event,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};
use winit::event::WindowEvent;
use winit::event_loop::ControlFlow;

pub extern crate cgmath;

pub mod input;
use input::InputHandler;

#[cfg(test)]
mod tests {
    use crate::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn default_functions() {
        init();

        let game_loop = GameLoop::new();
        let game = Game {
            title: "default_functions".to_string(),
            game_loop,
        };

        game.run();
    }

    #[test]
    fn custom_functions() {
        init();

        let mut game_loop = GameLoop::new();

        game_loop.on_start_call(|_| info!("Custom start function!"));
        game_loop.on_update_call(|_, _, _| info!("Custom update function!"));

        let game = Game {
            title: "custom_functions".to_string(),
            game_loop,
        };
        game.run();
    }
}

/// Example
/// ```
/// use game_engine::*;
/// let game_loop = GameLoop::new();
/// let game = Game {
///     title: "default_functions".to_string(),
///     game_loop,
/// };
/// game.run();
/// ```
pub struct Game {
    pub title: String,
    pub game_loop: GameLoop,
    // Todo: add game settings
}

impl Game {
    pub fn run(self) {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new().with_title(self.title).build(&event_loop).unwrap();
        self.game_loop.run(window, event_loop);
    }
}

pub struct GameFlow {
    flow: ControlFlow,
}

impl GameFlow {
    pub fn new() -> Self {
        Self {
            flow: Default::default()
        }
    }
    pub fn exit(&mut self) {
        self.flow = ControlFlow::Exit;
    }
}

/// Example
/// ```
/// use game_engine::*;
/// let mut game_loop = GameLoop::new();
/// game_loop.on_start_call(|_| println!("Custom start function!"));
/// game_loop.on_update_call(|_, _, _| println!("Custom update function!"));
/// let game = Game {
///     title: "custom_functions".to_string(),
///     game_loop,
/// };
/// game.run();
/// ```
pub struct GameLoop {
    start_fn: Box<dyn FnMut(&mut Renderer)>,
    update_fn: Box<dyn FnMut(&mut Renderer, &mut InputHandler, std::time::Duration)>,
    pub control_flow: GameFlow,
}

impl GameLoop {
    pub fn new() -> Self {
        GameLoop {
            start_fn: Box::new(|_| warn!("Empty start function!")),
            update_fn: Box::new(|_, _, _| warn!("Empty update function!")),
            control_flow: GameFlow::new(),
        }
    }

    pub fn on_start_call<S>(&mut self, start_function: S)
    where S: 'static + FnMut(&mut Renderer)
    {
        self.start_fn = Box::new(start_function);
    }

    pub fn on_update_call<U>(&mut self, update_function: U)
    where U: 'static + FnMut(&mut Renderer, &mut InputHandler, std::time::Duration)
    {
        self.update_fn = Box::new(update_function);
    }

    fn run(mut self, window: Window, event_loop: EventLoop<()>) {
        info!("Game loop is running");

        let mut renderer = Renderer {};
        let mut input = InputHandler::new();

        (self.start_fn)(&mut renderer);

        let mut last_time = std::time::Instant::now();
        event_loop.run(move |event, _, control_flow| {
            // Capture result from the start function
            *control_flow = self.control_flow.flow;
            match event {
                Event::WindowEvent { window_id, event } if window_id == window.id() => {
                    input.accept_input(&event);
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(_physical_size) => {
                            // Todo: resize logic
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size: _, ..} => {
                            // Todo: resize logic
                        }
                        _ => {}
                    }
                }
                Event::RedrawRequested(_) => {
                    let now = std::time::Instant::now();
                    let dt = now - last_time;
                    last_time = now;
                    (self.update_fn)(&mut renderer, &mut input, dt);
                    *control_flow = self.control_flow.flow;
                    // Todo: render
                }
                // RedrawRequested will only trigger once, unless we manually request it
                Event::MainEventsCleared => window.request_redraw(),
                _ => {}
            }
        })
    }
}

// WGPU
pub struct Renderer {}