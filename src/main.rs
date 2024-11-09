mod wgpu_stuff;
use wgpu_stuff::framework;
use wgpu_stuff::sim;

mod bodies;

const NUM_PARTICLES: u32 = 500;
const PARTICLES_PER_GROUP: u32 = 64;

pub fn main() {
    crate::framework::run::<sim::Sim>("nbody");
}
