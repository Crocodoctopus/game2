pub struct Window {
    pub window: winit::window::Window,
}

impl Window {
    pub fn new(window: winit::window::Window) -> Self {
        Self { window }
    }

    pub fn swap(&self) {
        self.window.request_redraw();
    }
}

pub struct EventLoop {
    pub event_loop: winit::event_loop::EventLoop<()>,
}

impl EventLoop {
    pub fn new(event_loop: winit::event_loop::EventLoop<()>) -> Self {
        Self { event_loop }
    }

    pub fn poll(&mut self) -> impl Iterator<Item = InputEvent> {
        let mut vec = Vec::new();
        use winit::platform::pump_events::EventLoopExtPumpEvents;
        let status = self
            .event_loop
            .pump_events(Some(std::time::Duration::ZERO), |event, _| {
                /*use winit::event::*;
                use winit::keyboard::*;
                match event {
                    Event::WindowEvent {
                        event: WindowEvent::CloseRequested,
                        ..
                    } => { vec.push(InputEvent::WindowClose) },
                    Event::WindowEvent {
                        event: WindowEvent::KeyboardInput { device_id, event, is_synthetic },
                        ..
                    } => {
                        // Get keycode.
                        use winit::platform::scancode::PhysicalKeyExtScancode;
                        let scancode = match event.physical_key {
                            PhysicalKey::Code(keycode) => match keycode.to_scancode() {
                                Some(scancode) => scancode,
                                None => return,
                            },
                            _ => return,
                        }



                    }
                    _ => { }
                }*/
            });

        /*
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
            .collect()*/

        vec.into_iter()
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
