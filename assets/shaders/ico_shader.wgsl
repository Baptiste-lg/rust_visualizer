#import bevy_sprite::mesh2d_vertex_output::VertexOutput

// --- 1. CONFIGURATION & UNIFORMS ---

struct IcoMaterial {
    color: vec4<f32>,            // Tint global
    resolution_mouse: vec4<f32>, // xy = physical resolution, zw = mouse
    time_params: vec4<f32>,      // x = time, y = speed, z = CAMERA ZOOM
    audio_params: vec4<f32>,     // x = Bass, y = Mid, z = Treble, w = Flux
};

@group(2) @binding(0)
var<uniform> material: IcoMaterial;

const PI: f32 = 3.14159265359;
const MAX_TRACE_DISTANCE: f32 = 40.0;
const INTERSECTION_PRECISION: f32 = 0.001;
const NUM_OF_TRACE_STEPS: i32 = 100;

// --- 2. DATA STRUCTURES ---

struct IcoBasis {
    nc: vec3<f32>,
    fold1: vec3<f32>,
    fold2: vec3<f32>,
    fold3: vec3<f32>,
    icoF1a: vec3<f32>,
    icoA1: vec3<f32>,
    icoB1: vec3<f32>,
    icoC1: vec3<f32>,
}

// --- 3. MATH HELPERS ---

fn saturate(x: f32) -> f32 { return clamp(x, 0.0, 1.0); }
fn vmax(v: vec3<f32>) -> f32 { return max(max(v.x, v.y), v.z); }

fn fPlane(p: vec3<f32>, n: vec3<f32>, dist: f32) -> f32 {
    return dot(p, n) + dist;
}

fn fPlaneComputed(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, center: vec3<f32>, dist: f32) -> f32 {
    var n = normalize(cross(c - b, a - b));
    var d = -dot(a, n);
    if (dot(n, center) + d > 0.0) {
        n = -n;
        d = -d;
    }
    return dot(p, n) + d + dist;
}

fn fOpIntersectionRound(a: f32, b: f32, r: f32) -> f32 {
    let m = max(a, b);
    if ((-a < r) && (-b < r)) {
        return max(m, -(r - sqrt((r + a) * (r + a) + (r + b) * (r + b))));
    } else {
        return m;
    }
}

fn fOpUnionRound(a: f32, b: f32, r: f32) -> f32 {
    let m = min(a, b);
    if ((a < r) && (b < r)) {
        return min(m, r - sqrt((r - a) * (r - a) + (r - b) * (r - b)));
    } else {
        return m;
    }
}

fn fCone(p_in: vec3<f32>, radius: f32, height: f32) -> f32 {
    let q = vec2<f32>(length(p_in.xz), p_in.y);
    let tip = q - vec2<f32>(0.0, height);
    let mantleDir = normalize(vec2<f32>(height, radius));
    let mantle = dot(tip, mantleDir);
    let d_init = max(mantle, -q.y);
    let projected = dot(tip, vec2<f32>(mantleDir.y, -mantleDir.x));

    var d = d_init;
    if ((q.y > height) && (projected < 0.0)) {
        d = max(d, length(tip));
    }
    if ((q.x > radius) && (projected > length(vec2<f32>(height, radius)))) {
        d = max(d, length(q - vec2<f32>(radius, 0.0)));
    }
    return d;
}

fn fConeDirected(p_in: vec3<f32>, radius: f32, height: f32, direction: vec3<f32>) -> f32 {
    let p = reflect(p_in, normalize(mix(vec3<f32>(0.0, 1.0, 0.0), -direction, 0.5)));
    return fCone(p, radius, height);
}

fn rotationMatrix(axis_in: vec3<f32>, angle: f32) -> mat3x3<f32> {
    let axis = normalize(axis_in);
    let s = sin(angle);
    let c = cos(angle);
    let oc = 1.0 - c;

    return mat3x3<f32>(
        oc * axis.x * axis.x + c,           oc * axis.x * axis.y - axis.z * s,  oc * axis.z * axis.x + axis.y * s,
        oc * axis.x * axis.y + axis.z * s,  oc * axis.y * axis.y + c,           oc * axis.y * axis.z - axis.x * s,
        oc * axis.z * axis.x - axis.y * s,  oc * axis.y * axis.z + axis.x * s,  oc * axis.z * axis.z + c
    );
}

fn pR(p: vec2<f32>, a: f32) -> vec2<f32> {
    return cos(a) * p + sin(a) * vec2<f32>(p.y, -p.x);
}

fn pReflect(p: ptr<function, vec3<f32>>, planeNormal: vec3<f32>, offset: f32) -> f32 {
    let t = dot(*p, planeNormal) + offset;
    if (t < 0.0) {
        *p = *p - (2.0 * t) * planeNormal;
    }
    return sign(t);
}

fn bToC(A: vec3<f32>, B: vec3<f32>, C: vec3<f32>, barycentric: vec3<f32>) -> vec3<f32> {
    return barycentric.x * A + barycentric.y * B + barycentric.z * C;
}

// --- 4. ICOSAHEDRON GEOMETRY SETUP ---

fn initIcosahedron() -> IcoBasis {
    let Type = 5.0;
    let cospin = cos(PI / Type);
    let scospin = sqrt(0.75 - cospin * cospin);

    let nc = vec3<f32>(-0.5, -cospin, scospin);
    let pab = vec3<f32>(0.0, 0.0, 1.0);
    let pbc_raw = vec3<f32>(scospin, 0.0, 0.5);
    let pca_raw = vec3<f32>(0.0, scospin, cospin);

    let pbc = normalize(pbc_raw);
    let pca = normalize(pca_raw);

    let A = pbc;
    let C = reflect(A, normalize(cross(pab, pca)));
    let B = reflect(C, normalize(cross(pbc, pca)));

    let p1_1 = bToC(A, B, C, vec3<f32>(0.5, 0.0, 0.5));
    let p2_1 = bToC(A, B, C, vec3<f32>(0.5, 0.5, 0.0));
    let fold1 = normalize(cross(p1_1, p2_1));

    let A2 = reflect(A, fold1);
    let B2 = p1_1;
    let C2 = p2_1;

    let icoF1a = pca;
    let icoA1 = A2;
    let icoB1 = normalize(B2);
    let icoC1 = normalize(C2);

    let p1_2 = bToC(A2, B2, C2, vec3<f32>(0.5, 0.0, 0.5));
    let p2_2 = bToC(A2, B2, C2, vec3<f32>(0.5, 0.5, 0.0));
    let fold2 = normalize(cross(p1_2, p2_2));

    let p1_3 = bToC(A2, B2, C2, vec3<f32>(0.0, 0.5, 0.5));
    let fold3 = normalize(cross(p2_2, p1_3));

    return IcoBasis(nc, fold1, fold2, fold3, icoF1a, icoA1, icoB1, icoC1);
}

fn pModIcosahedron(p: ptr<function, vec3<f32>>, subdivisions: i32, basis: IcoBasis) -> f32 {
    *p = abs(*p);
    pReflect(p, basis.nc, 0.0);

    (*p).x = abs((*p).x);
    (*p).y = abs((*p).y);

    pReflect(p, basis.nc, 0.0);

    (*p).x = abs((*p).x);
    (*p).y = abs((*p).y);

    pReflect(p, basis.nc, 0.0);

    var i: f32 = 0.0;
    if (subdivisions > 0) {
        i += pReflect(p, basis.fold1, 0.0) / 2.0 + 0.5;
        if (subdivisions > 1) {
             pReflect(p, basis.fold2, 0.0);
             pReflect(p, basis.fold3, 0.0);
        }
    }
    return i;
}

fn pRoll(p: ptr<function, vec3<f32>>, t: f32, basis: IcoBasis) {
    // Modify rotation speed slightly with music flux
    let speed_mod = 1.0 + material.audio_params.w * 0.5;

    var yx = vec2<f32>((*p).y, (*p).x);
    yx = pR(yx, PI / 3.0);
    (*p).y = yx.x;
    (*p).x = yx.y;

    var yz = vec2<f32>((*p).y, (*p).z);
    yz = pR(yz, PI / -5.0);
    (*p).y = yz.x;
    (*p).z = yz.y;

    let m = rotationMatrix(normalize(basis.icoF1a), t * speed_mod * ((PI * 2.0) / 3.0));
    *p = m * (*p);
}

// --- 5. MODELING (SDF) ---

fn fHolePart(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, d: vec3<f32>, round_r: f32, thick: f32) -> f32 {
    let center = (a + b + c + d) / 4.0;
    let f0 = fPlaneComputed(p, a, b, c, center, thick);
    let f1 = fPlaneComputed(p, a, c, d, center, thick);
    return fOpIntersectionRound(f0, f1, round_r);
}

fn fHole(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>) -> f32 {
    let w = 1.0;
    let h = 1.0;

    // React to Mid frequencies: Change the hole roundness
    let round_r = 0.08 + (material.audio_params.y * 0.05);
    let thick = 0.02;

    let AB = mix(a, b, 0.5);
    let AAB = mix(a, b, w);
    let ABB = mix(a, b, 1.0 - w);
    let n = normalize(cross(a, b));
    let cn = dot(c, n) * n;
    let AF = c - cn * (1.0 - h);
    let AF2 = reflect(AF, n);

    let part1 = fHolePart(p, vec3<f32>(0.0), AF2, AAB, AF, round_r, thick);
    let part2 = fHolePart(p, vec3<f32>(0.0), AF2, ABB, AF, round_r, thick);
    return fOpIntersectionRound(part1, part2, round_r);
}

fn holes(p: vec3<f32>, i: f32, basis: IcoBasis) -> f32 {
    var d = 1000.0;
    if (i > 0.0) {
        return min(d, fHole(p, basis.icoC1, basis.icoB1, basis.icoF1a));
    }
    d = min(d, fHole(p, basis.icoC1, basis.icoB1, basis.icoF1a));
    d = min(d, fHole(p, basis.icoA1, basis.icoB1, basis.icoF1a));
    return d;
}

fn spikes(p: vec3<f32>, basis: IcoBasis) -> f32 {
    // React to Treble: Increase spike length
    let spike_boost = material.audio_params.z * 1.5;

    var d = 1000.0;
    d = min(d, fConeDirected(p, 0.05, 1.3 + spike_boost, basis.icoF1a));
    d = min(d, fConeDirected(p, 0.05, 1.7 + spike_boost, basis.icoA1));
    d = min(d, fConeDirected(p, 0.05, 1.8 + spike_boost, basis.icoB1));
    return d;
}

fn shell(p: vec3<f32>, i: f32, basis: IcoBasis) -> f32 {
    let thick = 0.03;
    let round_r = 0.015;

    // React to Bass: Pulse the main shell size
    let pulse = material.audio_params.x * 0.2;

    var d = length(p) - (1.0 + pulse);
    d = fOpUnionRound(d, spikes(p, basis), 0.12);
    d = max(d, -(length(p) - ((1.0 + pulse) - thick)));
    var h = holes(p, i, basis);
    h = max(h, (length(p) - 1.1 - pulse));
    d = fOpIntersectionRound(d, -h, round_r);
    return d;
}

fn model(p_in: vec3<f32>, basis: IcoBasis, t: f32) -> f32 {
    var p = p_in;
    pRoll(&p, t, basis);

    var d = 1000.0;
    let i = pModIcosahedron(&p, 1, basis);
    d = min(d, shell(p, i, basis));
    return d;
}

fn map(p: vec3<f32>, basis: IcoBasis, t: f32) -> f32 {
    return model(p, basis, t);
}

// --- 6. RAYMARCHING & RENDERING ---

fn calcNormal(pos: vec3<f32>, basis: IcoBasis, t: f32) -> vec3<f32> {
    let eps = vec3<f32>(0.001, 0.0, 0.0);
    let nor = vec3<f32>(
        map(pos + eps.xyy, basis, t) - map(pos - eps.xyy, basis, t),
        map(pos + eps.yxy, basis, t) - map(pos - eps.yxy, basis, t),
        map(pos + eps.yyx, basis, t) - map(pos - eps.yyx, basis, t)
    );
    return normalize(nor);
}

fn softshadow(ro: vec3<f32>, rd: vec3<f32>, mint: f32, tmax: f32, basis: IcoBasis, t_val: f32) -> f32 {
    var res = 1.0;
    var t = mint;
    for (var i = 0; i < 16; i++) {
        let h = map(ro + rd * t, basis, t_val);
        res = min(res, 8.0 * h / t);
        t += clamp(h, 0.02, 0.10);
        if (h < 0.001 || t > tmax) { break; }
    }
    return clamp(res, 0.0, 1.0);
}

fn calcAO(pos: vec3<f32>, nor: vec3<f32>, basis: IcoBasis, t: f32) -> f32 {
    var occ = 0.0;
    var sca = 1.0;
    for (var i = 0; i < 5; i++) {
        let hr = 0.01 + 0.12 * f32(i) / 4.0;
        let aopos = nor * hr + pos;
        let dd = map(aopos, basis, t);
        occ += -(dd - hr) * sca;
        sca *= 0.95;
    }
    return clamp(1.0 - 3.0 * occ, 0.0, 1.0);
}

fn doLighting(col_in: vec3<f32>, pos: vec3<f32>, nor: vec3<f32>, refl: vec3<f32>, rd: vec3<f32>, basis: IcoBasis, t: f32) -> vec3<f32> {
    var col = col_in;
    let occ = calcAO(pos, nor, basis, t);
    let lig = normalize(vec3<f32>(-0.6, 0.7, 0.5));
    let amb = clamp(0.5 + 0.5 * nor.y, 0.0, 1.0);
    var dif = clamp(dot(nor, lig), 0.0, 1.0);
    let bac = clamp(dot(nor, normalize(vec3<f32>(-lig.x, 0.0, -lig.z))), 0.0, 1.0) * clamp(1.0 - pos.y, 0.0, 1.0);
    let fre = pow(clamp(1.0 + dot(nor, rd), 0.0, 1.0), 2.0);

    dif *= softshadow(pos, lig, 0.02, 2.5, basis, t);

    var lin = vec3<f32>(0.0);
    lin += 1.20 * dif * vec3<f32>(0.95, 0.80, 0.60) * material.color.rgb;
    lin += 0.80 * amb * vec3<f32>(0.50, 0.70, 0.80) * occ;
    lin += 0.30 * bac * vec3<f32>(0.25, 0.25, 0.25) * occ;

    // React to Flux: Add extra brightness to the fresnel rim light
    let flux_flash = material.audio_params.w * 0.5;
    lin += (0.20 + flux_flash) * fre * vec3<f32>(1.00, 1.00, 1.00) * occ;

    col = col * lin;

    return col;
}

fn calcIntersection(ro: vec3<f32>, rd: vec3<f32>, basis: IcoBasis, t: f32) -> vec2<f32> {
    var h = INTERSECTION_PRECISION * 2.0;
    var dist = 0.0;
    var res = -1.0;
    var id = -1.0;

    for (var i = 0; i < NUM_OF_TRACE_STEPS; i++) {
        if (h < INTERSECTION_PRECISION || dist > MAX_TRACE_DISTANCE) { break; }
        h = map(ro + rd * dist, basis, t);
        dist += h;
        id = 1.0;
    }

    if (dist < MAX_TRACE_DISTANCE) { res = dist; }
    if (dist > MAX_TRACE_DISTANCE) { id = -1.0; }

    return vec2<f32>(res, id);
}

fn render_scene(res: vec2<f32>, ro: vec3<f32>, rd: vec3<f32>, basis: IcoBasis, t: f32) -> vec3<f32> {
    var color = vec3<f32>(0.04, 0.045, 0.05);

    if (res.y > -0.5) {
        let pos = ro + rd * res.x;
        let nor = calcNormal(pos, basis, t);
        let refl = reflect(rd, nor);

        color = vec3<f32>(0.5);
        color = doLighting(color, pos, nor, refl, rd, basis, t);
    }
    return color;
}

fn calcLookAtMatrix(ro: vec3<f32>, ta: vec3<f32>, roll: f32) -> mat3x3<f32> {
    let ww = normalize(ta - ro);
    let uu = normalize(cross(ww, vec3<f32>(sin(roll), cos(roll), 0.0)));
    let vv = normalize(cross(uu, ww));
    return mat3x3<f32>(uu, vv, ww);
}

// --- 7. MAIN FRAGMENT ---

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let basis = initIcosahedron();

    var t_val = material.time_params.x * material.time_params.y;
    t_val = (t_val / 4.0) - floor(t_val / 4.0);

    let resolution = material.resolution_mouse.xy;
    let camera_zoom_scale = material.time_params.z;

    if (resolution.x == 0.0) { return vec4<f32>(0.0); }

    // Center coordinates: (0,0) is center of screen
    let p = (2.0 * mesh.position.xy - resolution.xy) / resolution.y;
    let p_corrected = vec2<f32>(p.x, -p.y);

    let orient = normalize(vec3<f32>(0.1, 1.0, 0.0));

    // Distance of camera (ro). Scaled by camera zoom.
    var zoom = 4.0 * camera_zoom_scale;

    let ro = zoom * orient;
    let ta = vec3<f32>(0.0);

    let camMat = calcLookAtMatrix(ro, ta, 0.0);
    let rd = normalize(camMat * vec3<f32>(p_corrected.xy, 2.0));

    let res = calcIntersection(ro, rd, basis, t_val);
    var color = render_scene(res, ro, rd, basis, t_val);

    // Gamma correction
    color = pow(color, vec3<f32>(1.0 / 2.2));

    return vec4<f32>(color, 1.0);
}