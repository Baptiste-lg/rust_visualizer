struct KaleidoscopeMaterial {
    color: vec4<f32>,
    time: f32,
    speed: f32,
    segments: f32,
    pattern_zoom: f32,
    resolution: vec2<f32>,
    bass: f32,
    mid: f32,
    treble: f32,
    flux: f32,
    zoom: f32,
    _pad: f32,
};

@group(2) @binding(0)
var<uniform> material: KaleidoscopeMaterial;

const PI: f32 = 3.14159265359;
const TAU: f32 = 6.28318530718;

// Attempt a pseudo-random hash for pattern generation
fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn pattern(p: vec2<f32>, t: f32) -> f32 {
    let p1 = sin(p.x * 3.0 + t) * cos(p.y * 2.0 - t * 0.7);
    let p2 = sin(length(p) * 5.0 - t * 1.3) * 0.5;
    let p3 = cos(p.x * p.y * 2.0 + t * 0.5);
    return (p1 + p2 + p3) / 3.0;
}

fn voronoi(p: vec2<f32>) -> f32 {
    let ip = floor(p);
    let fp = fract(p);
    var min_dist: f32 = 1.0;

    for (var y: i32 = -1; y <= 1; y++) {
        for (var x: i32 = -1; x <= 1; x++) {
            let neighbor = vec2<f32>(f32(x), f32(y));
            let cell_id = ip + neighbor;
            let cell_pos = neighbor + vec2<f32>(hash21(cell_id), hash21(cell_id + 100.0)) - fp;
            let d = length(cell_pos);
            min_dist = min(min_dist, d);
        }
    }
    return min_dist;
}

@fragment
fn fragment(
    @builtin(position) frag_coord: vec4<f32>
) -> @location(0) vec4<f32> {
    var p = (frag_coord.xy / material.resolution) * 2.0 - 1.0;
    p.x *= material.resolution.x / material.resolution.y;
    p *= material.zoom;

    let t = material.time * material.speed;

    // Convert to polar
    let dist = length(p);
    var angle = atan2(p.y, p.x);

    // Kaleidoscope mirror: fold angle into segment
    if (material.segments >= 1.0) {
        let seg_angle = TAU / material.segments;
        angle = abs(((angle % seg_angle) + seg_angle) % seg_angle - seg_angle * 0.5);
    }

    // Audio-reactive radius distortion
    let bass_warp = 1.0 + material.bass * 0.3;
    let warped_dist = dist * bass_warp;

    // Convert back to cartesian in folded space
    let folded = vec2<f32>(cos(angle), sin(angle)) * warped_dist * material.pattern_zoom;

    // Layer 1: flowing organic pattern
    let layer1 = pattern(folded + t * 0.1, t);

    // Layer 2: voronoi cells that react to mid frequencies
    let vor_scale = 3.0 + material.mid * 2.0;
    let layer2 = voronoi(folded * vor_scale + vec2<f32>(t * 0.15, t * 0.1));

    // Layer 3: radial waves from treble
    let layer3 = sin(warped_dist * 10.0 - t * 2.0 + material.treble * 5.0) * 0.5 + 0.5;

    // Combine layers
    let combined = layer1 * 0.4 + (1.0 - layer2) * 0.35 + layer3 * 0.25;
    let intensity = clamp(combined, 0.0, 1.0);

    // Color mapping with audio reactivity
    let base = material.color.rgb;
    let flux_shift = material.flux * 0.5;

    var col = vec3<f32>(
        base.r * intensity + flux_shift * 0.3,
        base.g * intensity * (1.0 + material.treble * 0.2),
        base.b * intensity + material.bass * 0.15,
    );

    // Bright center glow
    let center_glow = exp(-dist * 2.0) * material.bass * 0.3;
    col += vec3<f32>(center_glow);

    // Edge darkening
    let vignette = 1.0 - smoothstep(0.5, 2.0, dist);
    col *= vignette;

    // Gamma correction
    col = pow(max(col, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));

    return vec4<f32>(col, 1.0);
}
