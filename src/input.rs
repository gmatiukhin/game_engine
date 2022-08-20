use log::info;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, KeyboardInput, MouseScrollDelta, WindowEvent};

pub use winit::event::{MouseButton, VirtualKeyCode};

#[derive(Debug, Copy, Clone)]
pub enum ScrollDirection {
    Up,
    Down,
    None,
}

/// Processes input from the key presses, mouse button presses, cursor movement and mouse scroll wheel.
/// If several inputs are being processed at the same time some information may be lost.
pub struct InputHandler {
    active_keys: Vec<VirtualKeyCode>,
    active_mouse_buttons: Vec<MouseButton>,
    current_cursor_position: cgmath::Point2<f32>,
    previous_cursor_position: cgmath::Point2<f32>,
    cursor_delta: cgmath::Vector2<f32>,
    scroll_direction: ScrollDirection,
    scroll_delta: f32,
}

impl Default for InputHandler {
    fn default() -> Self {
        Self {
            active_keys: vec![],
            active_mouse_buttons: vec![],
            current_cursor_position: cgmath::Point2::new(0.0, 0.0),
            previous_cursor_position: cgmath::Point2::new(0.0, 0.0),
            cursor_delta: cgmath::Vector2::new(0.0, 0.0),
            scroll_direction: ScrollDirection::None,
            scroll_delta: 0.0,
        }
    }
}

impl InputHandler {
    pub(crate) fn new() -> Self {
        info!("Creating input handler");
        Default::default()
    }

    pub fn accept_input(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { input, .. } => self.accept_keyboard_input(input),
            WindowEvent::MouseWheel { delta, .. } => self.accept_scroll_wheel_input(delta),
            WindowEvent::MouseInput { state, button, .. } => {
                self.accept_mouse_button_input(state, button)
            }
            WindowEvent::CursorMoved { position, .. } => self.accept_cursor_input(position),
            _ => {}
        }
    }

    // o-----------------------------------o
    // |            KEYBOARD               |
    // o-----------------------------------o

    fn accept_keyboard_input(&mut self, keyboard_input: &KeyboardInput) {
        match keyboard_input {
            KeyboardInput {
                state,
                virtual_keycode: Some(key_code),
                ..
            } => match state {
                ElementState::Pressed => {
                    if !self.active_keys.contains(key_code) {
                        self.active_keys.push(*key_code);
                    }
                }
                ElementState::Released => {
                    if let Some(index) = self.active_keys.iter().position(|el| el == key_code) {
                        self.active_keys.remove(index);
                    }
                }
            },
            _ => {}
        }
    }

    pub fn is_key_down(&self, key_code: &VirtualKeyCode) -> bool {
        self.active_keys.contains(key_code)
    }

    pub fn is_key_up(&self, key_code: &VirtualKeyCode) -> bool {
        !self.is_key_down(key_code)
    }

    // o-----------------------------------o
    // |          MOUSE BUTTONS            |
    // o-----------------------------------o

    fn accept_mouse_button_input(&mut self, state: &ElementState, button: &MouseButton) {
        match state {
            ElementState::Pressed => {
                if !self.active_mouse_buttons.contains(button) {
                    self.active_mouse_buttons.push(*button);
                }
            }
            ElementState::Released => {
                if let Some(index) = self.active_mouse_buttons.iter().position(|el| el == button) {
                    self.active_mouse_buttons.remove(index);
                }
            }
        }
    }

    pub fn is_mouse_button_down(&self, button: &MouseButton) -> bool {
        self.active_mouse_buttons.contains(button)
    }

    pub fn is_mouse_button_up(&self, button: &MouseButton) -> bool {
        !self.is_mouse_button_down(button)
    }

    // o-----------------------------------o
    // |             CURSOR                |
    // o-----------------------------------o

    fn accept_cursor_input(&mut self, position: &PhysicalPosition<f64>) {
        self.previous_cursor_position = self.current_cursor_position;
        self.current_cursor_position = cgmath::Point2::new(position.x as f32, position.y as f32);
        self.cursor_delta = self.current_cursor_position - self.previous_cursor_position;
    }

    pub fn cursor_position(&self) -> cgmath::Point2<f32> {
        self.current_cursor_position
    }

    pub fn cursor_delta(&self) -> cgmath::Vector2<f32> {
        self.cursor_delta
    }

    pub(crate) fn reset_cursor_delta(&mut self) {
        self.cursor_delta = cgmath::Vector2::new(0.0, 0.0);
    }

    // o-----------------------------------o
    // |          SCROLL WHEEL             |
    // o-----------------------------------o

    fn accept_scroll_wheel_input(&mut self, delta: &MouseScrollDelta) {
        let scroll_delta = match delta {
            MouseScrollDelta::LineDelta(_, scroll) => *scroll,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => *y as f32,
        };
        self.scroll_delta = scroll_delta;

        if scroll_delta == 0.0 {
            self.scroll_direction = ScrollDirection::None;
        } else if scroll_delta > 0.0 {
            self.scroll_direction = ScrollDirection::Up;
        } else {
            self.scroll_direction = ScrollDirection::Down;
        }
    }

    pub fn scroll_direction(&self) -> ScrollDirection {
        self.scroll_direction
    }

    pub fn scroll_delta(&self) -> f32 {
        self.scroll_delta
    }

    /// Resets scroll wheel state to prevent infinite scrolling
    pub(crate) fn reset_scroll(&mut self) {
        self.scroll_direction = ScrollDirection::None;
        self.scroll_delta = 0.0;
    }
}
