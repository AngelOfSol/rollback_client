pub struct InputHistory<T> {
    front_frame: i32,
    data: Vec<T>,
}

impl<T: Clone> InputHistory<T> {
    pub fn new(x: T) -> Self {
        Self {
            front_frame: 0,
            data: vec![x; 20],
        }
    }

    fn adjust_frame(&self, frame: i32) -> usize {
        (frame - self.front_frame) as usize
    }

    pub fn add_local_input(&mut self, frame: i32, data: T) {
        let relative_frame = self.adjust_frame(frame);
        if relative_frame >= self.data.len() {
            self.data.push(data);
        } else {
            //panic!("should not be ")
        }
    }

    pub fn add_network_input(&mut self, frame: i32, data: T) {
        let relative_frame = self.adjust_frame(frame);
        if relative_frame == self.data.len() {
            self.data.push(data);
        } else if relative_frame > self.data.len() {
            // do nothing because we can't ues the data anyway
        } else {
            //panic!("should not be ")
        }
    }
    pub fn has_input(&self, frame: i32) -> bool {
        self.get_input(frame).is_some()
    }
    pub fn get_input(&self, frame: i32) -> Option<&T> {
        let relative_frame = self.adjust_frame(frame);
        self.data.get(relative_frame)
    }

    pub fn latest_input(&self) -> i32 {
        self.front_frame + self.data.len() as i32 - 1
    }
}
