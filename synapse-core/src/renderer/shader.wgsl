
struct CameraUniform {
    view_proj: mat4x4<f32>,
    light_view_proj: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct LightUniform {
    position: vec3<f32>,
    color: vec3<f32>,
}
@group(1) @binding(0)
var<uniform> light: LightUniform;
@group(1) @binding(1)
var shadow_map: texture_depth_2d;
@group(1) @binding(2)
var shadow_sampler: sampler_comparison;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) model_matrix_0: vec4<f32>,
    @location(3) model_matrix_1: vec4<f32>,
    @location(4) model_matrix_2: vec4<f32>,
    @location(5) model_matrix_3: vec4<f32>,
    @location(6) color: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    let model_matrix = mat4x4<f32>(
        model_matrix_0,
        model_matrix_1,
        model_matrix_2,
        model_matrix_3,
    );
    let world_position = model_matrix * vec4<f32>(position, 1.0);
    out.clip_position = camera.view_proj * world_position;
    out.world_position = world_position.xyz;
    out.normal = (model_matrix * vec4<f32>(normal, 0.0)).xyz;
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Transform fragment position to light space
    let shadow_pos = camera.light_view_proj * vec4<f32>(in.world_position, 1.0);
    let shadow_uv = shadow_pos.xy / shadow_pos.w * 0.5 + 0.5;

    // Sample the shadow map
    var shadow = 0.0;
    if (shadow_uv.x > 0.0 && shadow_uv.x < 1.0 && shadow_uv.y > 0.0 && shadow_uv.y < 1.0) {
        shadow = textureSampleCompare(shadow_map, shadow_sampler, shadow_uv, shadow_pos.z - 0.005);
    }

    let normal = normalize(in.normal);
    let light_dir = normalize(light.position - in.world_position);

    // Ambient
    let ambient_strength = 0.1;
    let ambient_color = light.color * ambient_strength;

    // Diffuse
    let diff = max(dot(normal, light_dir), 0.0);
    let diffuse_color = light.color * diff * shadow;

    // Specular (Blinn-Phong)
    let specular_strength = 0.5;
    let view_dir = normalize(camera.view_proj[3].xyz - in.world_position);
    let halfway_dir = normalize(light_dir + view_dir);
    let spec = pow(max(dot(normal, halfway_dir), 0.0), 32.0);
    let specular_color = light.color * specular_strength * spec * shadow;

    let result = (ambient_color + diffuse_color + specular_color) * in.color.rgb;
    return vec4<f32>(result, in.color.a);
}
