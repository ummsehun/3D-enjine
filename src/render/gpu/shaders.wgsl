// Vertex shader for 3D mesh rendering
// Transforms vertices from object space to clip space and passes
// normal and UV coordinates to the fragment shader.

struct Uniforms {
    mvp_matrix: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
    lighting_params: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = uniforms.mvp_matrix * vec4<f32>(input.position, 1.0);
    output.world_normal = input.normal;
    output.world_position = input.position;
    output.uv = input.uv;
    return output;
}

// Fragment shader for 3D mesh rendering
// Implements Phong lighting with basic shading.

struct FragmentInput {
    @location(0) world_normal: vec3<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@fragment
fn fs_main(input: FragmentInput) -> @location(0) vec4<f32> {
    let ambient_strength = uniforms.lighting_params.x;
    let diffuse_strength = uniforms.lighting_params.y;
    let specular_strength = uniforms.lighting_params.z;
    let specular_power = uniforms.lighting_params.w;

    let normal = normalize(input.world_normal);
    let light_dir = normalize(uniforms.light_dir.xyz);

    let ambient = ambient_strength * vec3<f32>(1.0, 1.0, 1.0);

    let diff = max(dot(normal, light_dir), 0.0);
    let diffuse = diffuse_strength * diff * vec3<f32>(1.0, 1.0, 1.0);

    let view_dir = normalize(uniforms.camera_pos.xyz - input.world_position);
    let reflect_dir = reflect(-light_dir, normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), specular_power);
    let specular = specular_strength * spec * vec3<f32>(1.0, 1.0, 1.0);

    let color = ambient + diffuse + specular;
    return vec4<f32>(color, 1.0);
}