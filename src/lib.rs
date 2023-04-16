pub mod common;
pub mod improved;
pub mod simple;

pub use common::decibels_to_gain;
pub use improved::ImpulseResponseImproved;
pub use simple::ImpulseResponseSimple;

pub trait ImpulseResponseGenerator {
    fn generate(&self, sample_rate: u32) -> Vec<f32>;
}
