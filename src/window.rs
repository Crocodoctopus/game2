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
        glfw::flush_messages(&self.events)
            .map(|(_, e)| match e {
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
