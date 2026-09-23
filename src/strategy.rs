//! Strategy output: per-timestep trading intent produced by Python in one call.

pub mod signal;
pub mod signal_frame;

pub use signal::Signal;
pub use signal_frame::{SignalFrame, SignalRow};
