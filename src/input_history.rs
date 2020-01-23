#[derive(Debug)]
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
    fn adjust_index(&self, idx: usize) -> i32 {
        idx as i32 + self.front_frame
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

    pub fn get_inputs(&self, frame: i32, amt: usize) -> (i32, &[T]) {
        let frame = self.adjust_frame(frame);
        let end_idx = self.data.len().min(frame + 1);
        let start_idx = end_idx.checked_sub(amt).unwrap_or(0);

        (self.adjust_index(start_idx), &self.data[start_idx..end_idx])
    }

    pub fn clean(&mut self, frame: i32) {
        let front_elements = self.adjust_frame(frame).checked_sub(20);

        if let Some(front_elements) = front_elements {
            if front_elements > 0 {
                self.data.drain(0..front_elements);
                self.front_frame = frame - 20;
            }
        }
    }
}
