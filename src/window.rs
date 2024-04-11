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

    pub fn poll(
        &mut self,
    ) -> (
        impl Iterator<Item = InputEvent>,
        impl Iterator<Item = WindowEvent>,
    ) {
        let mut input_events = Vec::new();
        let mut window_events = Vec::new();
        use winit::platform::pump_events::EventLoopExtPumpEvents;
        #[rustfmt::skip]
        let status = self
            .event_loop
            .pump_events(Some(std::time::Duration::ZERO), |event, _| {
                match event {
                    winit::event::Event::WindowEvent { window_id, event } => {
                        match event {
                            // Keyboard input event.
                            winit::event::WindowEvent::KeyboardInput { device_id, event, is_synthetic } => {
                                use winit::platform::scancode::PhysicalKeyExtScancode;
                                let keycode = match &event.logical_key {
                                    winit::keyboard::Key::Character(c) => c.as_str().chars().next().unwrap(),
                                    _ => return,
                                };

                                let press_state = match event.state {
                                    winit::event::ElementState::Pressed => PressState::Down,
                                    winit::event::ElementState::Released => PressState::Up,
                                };

                                input_events.push(InputEvent::KeyboardInput {
                                    keycode,
                                    press_state,
                                });
                            }

                            // Window close event.
                            winit::event::WindowEvent::CloseRequested => {
                                window_events.push(WindowEvent::WindowClose);
                            }

                            // Mouse move event.
                            winit::event::WindowEvent::CursorMoved { device_id, position } => {
                                input_events.push(InputEvent::MouseMove {
                                    x: position.x as f32,
                                    y: position.y as f32,
                                });
                            }

                            // Mouse click event.
                            winit::event::WindowEvent::MouseInput { device_id, state, button } => {
                                let mouse_button = match button {
                                    winit::event::MouseButton::Left => MouseButton::Left,
                                    winit::event::MouseButton::Right => MouseButton::Right,
                                    winit::event::MouseButton::Middle => MouseButton::Middle,
                                    winit::event::MouseButton::Other(i) => MouseButton::Button(i as u8),
                                    _ => return,
                                };

                                let press_state = match state {
                                    winit::event::ElementState::Pressed => PressState::Down,
                                    winit::event::ElementState::Released => PressState::Up,
                                };

                                input_events.push(InputEvent::MouseClick {
                                    mouse_button,
                                    press_state,
                                });
                            }

                            _ => return,
                        }
                    }
                    
                    _ => return,
                }
            });

        (input_events.into_iter(), window_events.into_iter())
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
}

pub enum WindowEvent {
    WindowClose,
}
