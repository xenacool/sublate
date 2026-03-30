pub mod transitions;
pub mod render;

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SAM {
    /// Mapping of state names to their definitions.
    pub states: HashMap<String, AnimationState>,
    /// Global transitions between states.
    pub transitions: Vec<Transition>,
    /// The entry point state.
    pub entry_state: String,
    /// The Lottie composition as a JSON value (for serialization).
    pub lottie_json: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AnimationState {
    pub name: String,
    pub loop_behavior: LoopBehavior,
    /// Optional: Start and end frames within the composition
    pub frame_range: Option<(f64, f64)>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum LoopBehavior {
    None,
    Loop,
    PingPong,
    Hold,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub trigger: Trigger,
    /// Duration of the transition (interpolation between states if applicable)
    pub duration_frames: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Trigger {
    /// Transition occurs automatically when the current state finishes.
    OnFinish,
    /// Transition occurs when a specific named input is received.
    OnInput(String),
}

impl Default for SAM {
    fn default() -> Self {
        Self {
            states: HashMap::new(),
            transitions: Vec::new(),
            entry_state: String::new(),
            lottie_json: serde_json::Value::Null,
        }
    }
}

pub struct VisualStateMachine {
    pub current_step: usize,
}

impl VisualStateMachine {
    pub fn new() -> Self {
        Self {
            current_step: 0,
        }
    }

    pub fn transition_to(&mut self, _next_state: crate::algorithm::AlgorithmState) {
        // Logic for transitioning between Lottie states
    }
}
