pub mod sorting;

pub trait Algorithm {
    fn step(&mut self);
    fn get_state(&self) -> &AlgorithmState;
}

#[derive(serde::Serialize)]
pub enum AlgorithmState {
    Initial,
    Stepping(usize),
    Finished,
}
