use crate::gl;
use glfw::*;

pub struct Window {
    window: PWindow,
    events: GlfwReceiver<(f64, WindowEvent)>,
}

impl Window {
    pub fn new(glfw: &mut Glfw, w: u32, h: u32) -> Self {
        glfw.window_hint(WindowHint::Resizable(false));
        let (mut window, events) = glfw
            .create_window(w, h, "Rarr", WindowMode::Windowed)
            .unwrap();
        window.set_key_polling(true);
        window.set_mouse_button_polling(true);
        window.set_cursor_pos_polling(true);
        window.make_current();

        Self { window, events }
    }

    pub fn gl_load(&mut self) {
        gl::load_with(|s| self.window.get_proc_address(s) as *const _);
    }

    pub fn swap(&mut self) {
        self.window.swap_buffers();
    }

    pub fn poll(&mut self) -> Vec<InputEvent> {
        let (w_w, w_h) = self.window.get_size();
        let (w_w, w_h) = (w_w as f64, w_h as f64);
        glfw::flush_messages(&self.events)
            .map(|(_, e)| match e {
                glfw::WindowEvent::CursorPos(x, y) => InputEvent::MouseMove {
                    x: (x / w_w) as f32,
                    y: (y / w_h) as f32,
                },
                glfw::WindowEvent::MouseButton(button, action, _) => {
                    let mouse_button = match button {
                        glfw::MouseButtonLeft => MouseButton::Left,
                        glfw::MouseButtonMiddle => MouseButton::Middle,
                        glfw::MouseButtonRight => MouseButton::Right,
                        glfw::MouseButton::Button4 => MouseButton::Button(4),
                        glfw::MouseButton::Button5 => MouseButton::Button(5),
                        glfw::MouseButton::Button6 => MouseButton::Button(6),
                        glfw::MouseButton::Button7 => MouseButton::Button(7),
                        glfw::MouseButton::Button8 => MouseButton::Button(8),
                    };
                    let press_state = match action {
                        Action::Release => PressState::Up,
                        Action::Press => PressState::Down,
                        _ => unreachable!(),
                    };
                    InputEvent::MouseClick {
                        mouse_button,
                        press_state,
                    }
                }
                glfw::WindowEvent::Key(key, _, action, _) => {
                    let keycode = key as u8 as char;
                    let press_state = match action {
                        Action::Release => PressState::Up,
                        Action::Press => PressState::Down,
                        Action::Repeat => PressState::DownRepeat,
                    };
                    InputEvent::KeyboardInput {
                        keycode,
                        press_state,
                    }
                }
                _ => InputEvent::WindowClose,
            })
            .collect()
    }
}

#[derive(Debug, Copy, Clone)]
pub enum PressState {
    Up,
    Down,
    DownRepeat,
}

#[derive(Debug, Copy, Clone)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Button(u8),
}

#[derive(Debug, Copy, Clone)]
pub enum InputEvent {
    KeyboardInput {
        keycode: char,
        press_state: PressState,
    },
    MouseMove {
        x: f32,
        y: f32,
    },
    MouseClick {
        mouse_button: MouseButton,
        press_state: PressState,
    },
    WindowClose,
}
