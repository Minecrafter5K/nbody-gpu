struct Particle {
    pos: vec2<f32>,
    vel: vec2<f32>,
    acc: vec2<f32>,
    mass: f32,
    radius: f32,
};

struct SimParams {
    deltaT: f32,
};

@group(0) @binding(0) var<uniform> params : SimParams;
@group(0) @binding(1) var<storage, read> particlesSrc : array<Particle>;
@group(0) @binding(2) var<storage, read_write> particlesDst : array<Particle>;

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let total = arrayLength(&particlesSrc);
    let index = global_invocation_id.x;
    if index >= total {
        return;
    }

    var pos: vec2<f32> = particlesSrc[index].pos;
    var vel: vec2<f32> = particlesSrc[index].vel;
    var acc: vec2<f32> = particlesSrc[index].acc;
    var mass: f32 = particlesSrc[index].mass;
    var radius: f32 = particlesSrc[index].radius;

    // Write back
    particlesDst[index] = Particle(pos, vel, vec2<f32>(0.0, 0.0), mass, radius);
}
