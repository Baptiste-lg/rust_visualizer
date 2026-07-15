struct TunnelMaterial {
    color: vec4<f32>,
    time: f32,
    speed: f32,
    ring_count: f32,
    twist: f32,
    resolution: vec2<f32>,
    bass: f32,
    mid: f32,
    treble: f32,
    flux: f32,
    zoom: f32,
    _pad: f32,
};

@group(2) @binding(0)
var<uniform> material: TunnelMaterial;

const PI: f32 = 3.14159265359;

@fragment
fn fragment(
    @builtin(position) frag_coord: vec4<f32>
) -> @location(0) vec4<f32> {
    var p = (frag_coord.xy / material.resolution) * 2.0 - 1.0;
    p.x *= material.resolution.x / material.resolution.y;
    p *= material.zoom;

    let dist = length(p);
    let angle = atan2(p.y, p.x);

    if (dist < 0.001) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // Tunnel depth from inverse distance
    let depth = 1.0 / dist;
    let t = material.time * material.speed;

    // Twist the tunnel
    let twisted_angle = angle + depth * material.twist + t * 0.2;

    // Ring pattern along depth
    let ring_val = sin((depth - t) * material.ring_count * PI) * 0.5 + 0.5;

    // Angular segments
    let segments = 8.0;
    let seg_val = sin(twisted_angle * segments) * 0.5 + 0.5;

    // Combine patterns
    let pattern = ring_val * 0.7 + seg_val * 0.3;

    // Audio reactivity
    let bass_pulse = 1.0 + material.bass * 0.5;
    let mid_glow = material.mid * 0.3;
    let treble_flicker = 1.0 + sin(depth * 50.0 + t * 10.0) * material.treble * 0.2;

    // Fog/fade with distance
    let fog = exp(-dist * 0.8) * bass_pulse;

    // Color mixing
    let base = material.color.rgb;
    let highlight = vec3<f32>(1.0, 1.0, 1.0);

    var col = mix(base * 0.2, base, pattern) * fog * treble_flicker;
    col += highlight * ring_val * mid_glow * fog;

    // Vignette: bright center rim
    let rim = smoothstep(0.0, 0.15, dist) * smoothstep(2.0, 0.5, dist);
    col *= rim;

    // Gamma correction
    col = pow(max(col, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));

    return vec4<f32>(col, 1.0);
}
