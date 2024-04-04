pub struct Server {}

impl Server {
    pub fn new(port: u16) -> (Self, u16) {
        (Self {}, 0)
    }

    pub fn update_once(&mut self) {}
}
