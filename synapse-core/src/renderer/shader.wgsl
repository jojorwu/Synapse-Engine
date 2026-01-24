
struct CameraUniform {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
}
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct LightUniform {
    position: vec3<f32>,
    _padding: u32,
    color: vec3<f32>,
    _padding2: u32,
    light_view_proj: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> light: LightUniform;

@group(1) @binding(1)
var shadow_map: texture_depth_2d;
@group(1) @binding(2)
var shadow_sampler: sampler_comparison;

@group(2) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(2) @binding(1)
var s_diffuse: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
    @location(3) shadow_pos: vec4<f32>,
    @location(4) uv: vec2<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) model_matrix_0: vec4<f32>,
    @location(4) model_matrix_1: vec4<f32>,
    @location(5) model_matrix_2: vec4<f32>,
    @location(6) model_matrix_3: vec4<f32>,
    @location(7) color: vec4<f32>,
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
    out.shadow_pos = light.light_view_proj * world_position;
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Perform perspective divide and transform to UV space
    let shadow_uv = in.shadow_pos.xy / in.shadow_pos.w * 0.5 + 0.5;

    // Sample the shadow map
    var shadow = 1.0; // Default to not in shadow
    if (shadow_uv.x > 0.0 && shadow_uv.x < 1.0 && shadow_uv.y > 0.0 && shadow_uv.y < 1.0) {
        // use textureSampleCompare instead of textureSample
        // the last argument is the depth to compare against
        shadow = textureSampleCompare(shadow_map, shadow_sampler, shadow_uv, in.shadow_pos.z - 0.005);
    }


    let normal = normalize(in.normal);
    let light_dir = normalize(light.position - in.world_position);

    // Ambient
    let ambient_strength = 0.1;
    let ambient_color = light.color * ambient_strength;

    // Diffuse - only apply if not in shadow
    let diff = max(dot(normal, light_dir), 0.0);
    let diffuse_color = light.color * diff * shadow;

    // Specular (Blinn-Phong) - only apply if not in shadow
    let specular_strength = 0.5;
    let view_dir = normalize(camera.position - in.world_position);
    let halfway_dir = normalize(light_dir + view_dir);
    let spec = pow(max(dot(normal, halfway_dir), 0.0), 32.0);
    let specular_color = light.color * specular_strength * spec * shadow;

    let texture_color = textureSample(t_diffuse, s_diffuse, in.uv);
    let result = (ambient_color + diffuse_color + specular_color) * in.color.rgb * texture_color.rgb;
    return vec4<f32>(result, in.color.a);
}
