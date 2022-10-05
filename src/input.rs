use log::info;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyboardInput, MouseScrollDelta, WindowEvent},
};

pub use winit::event::{MouseButton, VirtualKeyCode};

/// Describes current direction of the scroll wheel
#[derive(Debug, Copy, Clone)]
pub enum ScrollDirection {
    Up,
    Down,
    None,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Pressable {
    KeyboardKey(VirtualKeyCode),
    MouseButton(MouseButton),
}

/// State of things that could be pressed (keyboard key, mouse buttons)
#[derive(Debug, Copy, Clone)]
struct PressableState {
    button: Pressable,
    current_state: ElementState,
    previous_state: ElementState,
}

impl PressableState {
    /// Creates a new instance of the struct using keyboard key as an input
    fn new_keyboard_key(key: &VirtualKeyCode) -> Self {
        let button = Pressable::KeyboardKey(*key);
        Self {
            button,
            current_state: ElementState::Released,
            previous_state: ElementState::Released,
        }
    }

    /// Creates a new instance of the struct using mouse button as an input
    fn new_mouse_button(button: &MouseButton) -> Self {
        let button = Pressable::MouseButton(*button);
        Self {
            button,
            current_state: ElementState::Released,
            previous_state: ElementState::Released,
        }
    }

    /// Sets current state of the instance and updates the previous
    fn set_state(&mut self, new_state: &ElementState) {
        self.previous_state = self.current_state;
        self.current_state = *new_state;
    }

    /// Updates current state
    /// Changes current state ether from `Pressed` to `Down`
    /// or from `Released` to `Up`
    /// This allows to split the state of the button into 4 phases
    /// - Pressed (only during one frame)
    /// - Held
    /// - Released (only during one frame)
    /// - Up
    fn update_state(&mut self) {
        use ElementState::*;
        if self.previous_state == Released && self.current_state == Pressed {
            self.previous_state = Pressed;
        } else if self.previous_state == Pressed && self.current_state == Released {
            self.previous_state = Released;
        }
    }

    fn is_down(&self) -> bool {
        self.current_state == ElementState::Pressed && self.previous_state == ElementState::Released
    }

    fn is_held(&self) -> bool {
        self.current_state == ElementState::Pressed && self.previous_state == ElementState::Pressed
    }

    fn is_released(&self) -> bool {
        self.current_state == ElementState::Released && self.previous_state == ElementState::Pressed
    }

    fn is_up(&self) -> bool {
        self.current_state == ElementState::Released
            && self.previous_state == ElementState::Released
    }
}

impl PartialEq for PressableState {
    fn eq(&self, other: &Self) -> bool {
        self.button == other.button
    }
}

/// Processes input from the key presses, mouse button presses, cursor movement and mouse scroll wheel.
/// If several inputs are being processed at the same time some information may be lost.
pub struct InputHandler {
    active_keys: Vec<PressableState>,
    current_cursor_position: cgmath::Point2<f32>,
    previous_cursor_position: cgmath::Point2<f32>,
    cursor_delta: cgmath::Vector2<f32>,
    scroll_direction: ScrollDirection,
    scroll_delta: f32,
}

impl InputHandler {
    /// Creates a new instance
    pub(crate) fn new() -> Self {
        info!("Creating input handler");
        Self {
            active_keys: vec![],
            current_cursor_position: cgmath::Point2::new(0.0, 0.0),
            previous_cursor_position: cgmath::Point2::new(0.0, 0.0),
            cursor_delta: cgmath::Vector2::new(0.0, 0.0),
            scroll_direction: ScrollDirection::None,
            scroll_delta: 0.0,
        }
    }

    /// Accepts input event from the system
    pub(crate) fn accept_input(&mut self, event: &WindowEvent) {
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

    /// Handles all that can be pressed
    fn handle_pressable(&mut self, button: &mut PressableState, state: &ElementState) {
        match state {
            ElementState::Pressed => {
                if !self.active_keys.contains(button) {
                    button.set_state(state);
                    self.active_keys.push(*button);
                } else {
                    if let Some(index) = self.active_keys.iter().position(|el| el == button) {
                        self.active_keys[index].set_state(state);
                    }
                }
            }
            ElementState::Released => {
                if let Some(index) = self.active_keys.iter().position(|el| el == button) {
                    self.active_keys[index].set_state(state);
                }
            }
        }
    }

    /// Updates and resets values
    pub(crate) fn update_input_state(&mut self) {
        self.reset_scroll();
        self.reset_cursor_delta();
        self.update_key_state();
    }
}

// o-----------------------------------o
// |            KEYBOARD               |
// o-----------------------------------o
/// Methods related to processing of the keyboard's input
impl InputHandler {
    /// Handles processing and storage of keyboard's input
    fn accept_keyboard_input(&mut self, keyboard_input: &KeyboardInput) {
        match keyboard_input {
            KeyboardInput {
                state,
                virtual_keycode: Some(key_code),
                ..
            } => {
                let mut button = PressableState::new_keyboard_key(key_code);
                self.handle_pressable(&mut button, state);
            }
            _ => {}
        }
    }

    /// Returns true on the first frame when the keyboard key is pressed
    pub fn is_key_down(&self, key_code: &VirtualKeyCode) -> bool {
        let key = PressableState::new_keyboard_key(key_code);
        if let Some(index) = self.active_keys.iter().position(|el| el == &key) {
            return self.active_keys[index].is_down();
        }

        false
    }

    /// Returns true while the keyboard key is held down
    pub fn is_key_held(&self, key_code: &VirtualKeyCode) -> bool {
        let key = PressableState::new_keyboard_key(key_code);
        if let Some(index) = self.active_keys.iter().position(|el| el == &key) {
            return self.active_keys[index].is_held();
        }

        false
    }

    /// Returns true on the first frame when the keyboard key is released
    pub fn is_key_released(&self, key_code: &VirtualKeyCode) -> bool {
        let key = PressableState::new_keyboard_key(key_code);
        if let Some(index) = self.active_keys.iter().position(|el| el == &key) {
            return self.active_keys[index].is_released();
        }

        false
    }

    /// Returns true while the keyboard key is not pressed
    pub fn is_key_up(&self, key_code: &VirtualKeyCode) -> bool {
        let key = PressableState::new_keyboard_key(key_code);
        !self.active_keys.contains(&key)
    }

    /// Updates the state of all active keys and removes those which are no longer active
    fn update_key_state(&mut self) {
        self.active_keys = self
            .active_keys
            .iter_mut()
            .map(|key| {
                key.update_state();
                *key
            })
            .filter(|key| !key.is_up())
            .collect();
    }
}

// o-----------------------------------o
// |          MOUSE BUTTONS            |
// o-----------------------------------o
/// Methods related to processing of the mouse buttons' input
impl InputHandler {
    /// Handles processing and storage of mouse buttons' input
    fn accept_mouse_button_input(&mut self, state: &ElementState, button: &MouseButton) {
        let mut button = PressableState::new_mouse_button(button);
        self.handle_pressable(&mut button, state);
    }

    /// Returns true on the first frame when the mouse button is pressed
    pub fn is_mouse_button_down(&self, key_code: &MouseButton) -> bool {
        let key = PressableState::new_mouse_button(key_code);
        if let Some(index) = self.active_keys.iter().position(|el| el == &key) {
            return self.active_keys[index].is_down();
        }

        false
    }

    /// Returns true while the mouse button is held down
    pub fn is_mouse_button_held(&self, key_code: &MouseButton) -> bool {
        let key = PressableState::new_mouse_button(key_code);
        if let Some(index) = self.active_keys.iter().position(|el| el == &key) {
            return self.active_keys[index].is_held();
        }

        false
    }

    /// Returns true on the first frame when the mouse button is released
    pub fn is_mouse_button_released(&self, key_code: &MouseButton) -> bool {
        let key = PressableState::new_mouse_button(key_code);
        if let Some(index) = self.active_keys.iter().position(|el| el == &key) {
            return self.active_keys[index].is_released();
        }

        false
    }

    /// Returns true while the mouse button is not pressed
    pub fn is_mouse_button_up(&self, key_code: &MouseButton) -> bool {
        let key = PressableState::new_mouse_button(key_code);
        !self.active_keys.contains(&key)
    }
}

// o-----------------------------------o
// |             CURSOR                |
// o-----------------------------------o
/// Methods related to processing of the cursor's input
impl InputHandler {
    /// Handles processing and storage of cursor's input
    fn accept_cursor_input(&mut self, position: &PhysicalPosition<f64>) {
        self.previous_cursor_position = self.current_cursor_position;
        self.current_cursor_position = cgmath::Point2::new(position.x as f32, position.y as f32);
        self.cursor_delta = self.current_cursor_position - self.previous_cursor_position;
    }

    /// Returns current cursor position on the screen
    pub fn cursor_position(&self) -> cgmath::Point2<f32> {
        self.current_cursor_position
    }

    /// Returns the difference between cursor's position during the current frame
    /// and during the previous frame
    pub fn cursor_delta(&self) -> cgmath::Vector2<f32> {
        self.cursor_delta
    }

    /// Resets delta to zero between frames
    fn reset_cursor_delta(&mut self) {
        self.cursor_delta = cgmath::Vector2::new(0.0, 0.0);
    }
}

// o-----------------------------------o
// |          SCROLL WHEEL             |
// o-----------------------------------o
/// Methods related to processing of the scroll wheel's input
impl InputHandler {
    /// Handles processing and storage of scroll wheel's input
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

    /// Returns scroll direction as a value of an enum
    pub fn scroll_direction(&self) -> ScrollDirection {
        self.scroll_direction
    }

    /// Returns change in cursors scroll direction
    pub fn scroll_delta(&self) -> f32 {
        self.scroll_delta
    }

    /// Resets scroll wheel state to prevent infinite scrolling
    fn reset_scroll(&mut self) {
        self.scroll_direction = ScrollDirection::None;
        self.scroll_delta = 0.0;
    }
}
