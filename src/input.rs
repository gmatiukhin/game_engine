use winit::event::{ElementState, KeyboardInput, WindowEvent};

pub use winit::event::VirtualKeyCode as KeyCode;
pub use winit::event::MouseButton;

#[derive(Default)]
pub struct InputHandler {
    keyboard_input: Option<KeyboardInput>,
    mouse_button_input: Option<MouseInput>,
}

struct MouseInput {
    state: ElementState,
    button: MouseButton,
}

impl InputHandler {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub fn accept_input(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { input, .. } => self.keyboard_input = Some(*input),
            WindowEvent::MouseWheel { delta: _delta, .. } => {}
            WindowEvent::MouseInput { state, button, .. } => self.mouse_button_input = Some(MouseInput {
                state: *state,
                button: *button,
            }),
            WindowEvent::CursorMoved { position: _position, .. } => {}
            _ => {}
        }
    }

    // o-----------------------------------o
    // |            KEYBOARD               |
    // o-----------------------------------o
    fn on_key_change_state(&self, key_code: KeyCode, key_state: ElementState, mut function: impl FnMut()) {
        if let Some(keyboard_input) = self.keyboard_input {
            if let Some(key) = keyboard_input.virtual_keycode {
                if key == key_code && keyboard_input.state == key_state {
                    function();
                }
            }
        }
    }

    pub fn on_key_pressed(&self, key_code: KeyCode, function: impl FnMut()) {
        self.on_key_change_state(key_code, ElementState::Pressed, function);
    }

    pub fn on_key_released(&self, key_code: KeyCode, function: impl FnMut()) {
        self.on_key_change_state(key_code, ElementState::Released, function);
    }

    pub fn key_input(&self) -> Option<(KeyCode, ElementState)> {
        if let Some(key_input) = self.keyboard_input {
            if let Some(v_code) = key_input.virtual_keycode {
                return Some((v_code, key_input.state));
            }
        }
        None
    }

    // o-----------------------------------o
    // |              MOUSE                |
    // o-----------------------------------O

    pub fn mouse_wheel(&self) -> MouseWheelDirection {
        todo!()
    }

    pub fn mouse_delta(&self) {
        todo!()
    }

    pub fn mouse_position(&self) {
        todo!()
    }

    fn on_mouse_button_change_state(&self, mouse_button: MouseButton, button_state: ElementState, mut function: impl FnMut()) {
        if let Some(mouse_button_input) = &self.mouse_button_input {
            if mouse_button_input.button == mouse_button && mouse_button_input.state == button_state {
                function();
            }
        }
    }

    pub fn on_mouse_button_pressed(&self, mouse_button: MouseButton, function: impl FnMut()) {
        self.on_mouse_button_change_state(mouse_button, ElementState::Pressed, function);
    }

    pub fn on_mouse_button_released(&self, mouse_button: MouseButton, function: impl FnMut()) {
        self.on_mouse_button_change_state(mouse_button, ElementState::Released, function);
    }

    pub fn mouse_button_key_input(&self) {
        todo!()
    }
}

pub enum MouseWheelDirection {
    Up,
    Down,
    None,
}