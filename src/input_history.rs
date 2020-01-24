#[derive(Debug)]
pub struct InputHistory<T> {
    front_frame: usize,
    data: Vec<T>,
}

impl<T> InputHistory<T> {
    pub fn new() -> Self {
        Self {
            front_frame: 0,
            data: vec![],
        }
    }

    fn adjust_frame(&self, frame: usize) -> usize {
        frame - self.front_frame
    }
    fn adjust_index(&self, idx: usize) -> usize {
        idx + self.front_frame
    }

    pub fn add_local_input(&mut self, frame: usize, data: T) -> usize {
        let relative_frame = self.adjust_frame(frame);
        if relative_frame == self.data.len() {
            self.data.push(data);
            frame
        } else if relative_frame > self.data.len() {
            self.data.push(data);
            self.front_frame + self.data.len() - 1
        } else {
            frame
        }
    }

    pub fn add_network_input(&mut self, frame: usize, data: T) {
        let relative_frame = self.adjust_frame(frame);
        if relative_frame == self.data.len() {
            self.data.push(data);
        } else if relative_frame > self.data.len() {
            dbg!("threw data out");
            dbg!(relative_frame);
            dbg!(self.data.len());
        } else {
        }
    }
    pub fn has_input(&self, frame: usize) -> bool {
        self.get_input(frame).is_some()
    }
    pub fn get_input(&self, frame: usize) -> Option<&T> {
        let relative_frame = self.adjust_frame(frame);
        self.data.get(relative_frame)
    }

    pub fn last_input(&self) -> usize {
        self.front_frame + self.data.len().checked_sub(1).unwrap_or(0)
    }

    pub fn get_inputs(&self, frame: usize, amt: usize) -> &[T] {
        let frame = self.adjust_frame(frame);
        let end_idx = self.data.len().min(frame + 1);
        let start_idx = end_idx.checked_sub(amt).unwrap_or(0);

        &self.data[start_idx..end_idx]
    }

    pub fn clean(&mut self, frame: usize) {
        let front_elements = self.adjust_frame(frame).checked_sub(20);

        if let Some(front_elements) = front_elements {
            if front_elements > 0 {
                self.data.drain(0..front_elements);
                self.front_frame = frame - 20;
            }
        }
    }
}
