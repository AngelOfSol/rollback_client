mod local_history;
mod networked_history;

pub use local_history::LocalHistory;
pub use networked_history::NetworkedHistory;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Canon {
    Canon,
    Empty,
}
