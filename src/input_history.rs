pub struct InputHistory<T> {
    front_frame: i32,
    data: Vec<T>,
}

impl<T> InputHistory<T> {
    pub fn new() -> Self {
        Self {
            front_frame: 0,
            data: Vec::new(),
        }
    }

    pub fn add_local_input(frame: i32, data: T) {
        let relative_frame = frame - front_frame;
        if relative_frame >= self.data.len() {
            self.data.push(data);
        } else {
            panic!("should not be ")
        }
    }

    pub fn add_network_input(frame: i32, data: T) {
        let relative_frame = frame - front_frame;
        if relative_frame == self.data.len() {
            self.data.push(data);
        } else if relative_frame > self.data.len() {
            // do nothing because we can't ues the data anyway
        } else {
            panic!("should not be ")
        }
    }
}
