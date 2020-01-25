use super::Canon;

#[derive(Debug)]
pub struct NetworkedHistory<T> {
    front_frame: usize,
    canon: Vec<Canon>,
    data: Vec<T>,
}

impl<T: Default + Clone> NetworkedHistory<T> {
    pub fn new() -> Self {
        Self {
            front_frame: 0,
            canon: vec![],
            data: vec![],
        }
    }

    fn adjust_frame(&self, frame: usize) -> Option<usize> {
        frame.checked_sub(self.front_frame)
    }

    pub fn add_input(&mut self, frame: usize, data: T) {
        let relative_frame = self.adjust_frame(frame).unwrap();
        if relative_frame == self.data.len() {
            self.canon.push(Canon::Canon);
            self.data.push(data);
        } else if relative_frame > self.data.len() {
            self.canon.resize(relative_frame + 1, Canon::Empty);
            self.data.resize(relative_frame + 1, Default::default());

            self.canon[relative_frame] = Canon::Canon;
            self.data[relative_frame] = data;
        } else {
            if self.canon[relative_frame] != Canon::Canon {
                self.canon[relative_frame] = Canon::Canon;
                self.data[relative_frame] = data;
            }
        }
    }
    pub fn has_input(&self, frame: usize) -> bool {
        self.adjust_frame(frame)
            .and_then(|frame| self.canon.get(frame))
            .map(|canon| *canon == Canon::Canon)
            .unwrap_or(false)
    }

    pub fn get_inputs(&self, frame: usize, amt: usize) -> &[T] {
        let frame = self.adjust_frame(frame).unwrap();
        let end_idx = self.data.len().min(frame + 1);
        let start_idx = end_idx.checked_sub(amt).unwrap_or(0);

        &self.data[start_idx..end_idx]
    }

    pub fn clean(&mut self, frame: usize) {
        let front_elements = self.adjust_frame(frame);

        if let Some(front_elements) = front_elements {
            if front_elements > 0 {
                self.data.drain(0..front_elements);
                self.canon.drain(0..front_elements);
                self.front_frame = frame;
            }
        }
    }
}
