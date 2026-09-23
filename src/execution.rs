//! Turning signals into fills: sizing -> order -> simulated fill.

pub mod fill;
pub mod order;
pub mod simulator;
pub mod sizing;

pub use fill::Fill;
pub use order::Order;
pub use simulator::FillSimulator;
