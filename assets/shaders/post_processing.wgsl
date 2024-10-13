#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var depth_texture: texture_depth_multisampled_2d;
@group(0) @binding(3) var normal_texture: texture_multisampled_2d<f32>;

struct PostProcessSettings {
    pixel_size: f32,
    normal_edge_strength: f32,
    depth_edge_strength: f32,
    resolution: vec2<f32>,
}
@group(0) @binding(4) var<uniform> settings: PostProcessSettings;

fn get_depth(uv: vec2<f32>) -> f32 {
    let dims = textureDimensions(depth_texture);
    let coords = vec2<i32>(uv * vec2<f32>(dims));
    return textureLoad(depth_texture, coords, 0);
}

fn get_normal(uv: vec2<f32>) -> vec3<f32> {
    let dims = textureDimensions(normal_texture);
    let coords = vec2<i32>(uv * vec2<f32>(dims));
    let normal = textureLoad(normal_texture, coords, 0);
    return vec3<f32>(normal.x, normal.y, normal.z) * 2.0 - vec3<f32>(1.0, 1.0, 1.0);
}

fn depth_edge_indicator(uv: vec2<f32>, depth: f32) -> f32 {
    let offset = 1.0 / settings.resolution;
    var diff = 0.0;
    diff += max(get_depth(uv + vec2<f32>(offset.x, 0.0)) - depth, 0.0);
    diff += max(get_depth(uv + vec2<f32>(-offset.x, 0.0)) - depth, 0.0);
    diff += max(get_depth(uv + vec2<f32>(0.0, offset.y)) - depth, 0.0);
    diff += max(get_depth(uv + vec2<f32>(0.0, -offset.y)) - depth, 0.0);
    return floor(smoothstep(0.01, 0.02, diff) * 2.0) / 2.0;
}

fn neighbor_normal_edge_indicator(uv: vec2<f32>, offset: vec2<f32>, depth: f32, normal: vec3<f32>) -> f32 {
    let neighbor_uv = uv + offset;
    let neighbor_depth = get_depth(neighbor_uv);
    let neighbor_normal = get_normal(neighbor_uv);

    let normal_edge_bias = vec3<f32>(1.0, 1.0, 1.0);
    let normal_diff = dot(normal - neighbor_normal, normal_edge_bias);
    let normal_indicator = clamp(smoothstep(-0.01, 0.01, normal_diff), 0.0, 1.0);

    let depth_indicator = clamp(sign(neighbor_depth - depth) * 0.25 + 0.0025, 0.0, 1.0);

    return (1.0 - dot(normal, neighbor_normal)) * depth_indicator * normal_indicator;
}

fn normal_edge_indicator(uv: vec2<f32>, depth: f32, normal: vec3<f32>) -> f32 {
    let offset = 1.0 / settings.resolution;
    var indicator = 0.0;

    indicator += neighbor_normal_edge_indicator(uv, vec2<f32>(0.0, -offset.y), depth, normal);
    indicator += neighbor_normal_edge_indicator(uv, vec2<f32>(0.0, offset.y), depth, normal);
    indicator += neighbor_normal_edge_indicator(uv, vec2<f32>(-offset.x, 0.0), depth, normal);
    indicator += neighbor_normal_edge_indicator(uv, vec2<f32>(offset.x, 0.0), depth, normal);

    return step(0.1, indicator);
}

fn quantize_and_dither(color: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let color_depth = max(32.0, 2.0);  // Reduced color depth for more extreme quantization
    let x = fract(uv.x * f32(textureDimensions(screen_texture).x));
    let y = fract(uv.y * f32(textureDimensions(screen_texture).y));
    let dither_value = (fract(x * 0.375 + y * 0.75 + 0.8) * 2.0 - 1.0) / color_depth;

    // Apply color shifts
    let shifted_color = vec3<f32>(
        fract(color.r + 0.33),
        fract(color.g + 0.66),
        fract(color.b + 0.99)
    );

    // Increase contrast
    let contrasted_color = pow(shifted_color, vec3<f32>(2.5));

    // Apply quantization and dithering
    let quantized_color = floor((contrasted_color + vec3(dither_value)) * color_depth) / (color_depth - 1.0);

    // Add some hue rotation
    let hue_shift = 0.1;  // Adjust this value to change the amount of hue shift
    return vec3<f32>(
        fract(quantized_color.r + hue_shift),
        fract(quantized_color.g + hue_shift),
        fract(quantized_color.b + hue_shift)
    );
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pixel_size = max(settings.pixel_size, 1.0);
    let pixel_uv = floor(in.uv * settings.resolution / pixel_size) * pixel_size / settings.resolution;

    let depth = get_depth(pixel_uv);
    let normal = get_normal(pixel_uv);

    let color = textureSample(screen_texture, texture_sampler, pixel_uv);


    let dei = depth_edge_indicator(pixel_uv, depth);
    let nei = normal_edge_indicator(pixel_uv, depth, normal);

    var edge_strength: f32;
    if (dei > 0.0) {
        edge_strength = 1.0 - settings.depth_edge_strength * dei;
    } else {
        edge_strength = 1.0 + settings.normal_edge_strength * nei;
    }

    var final_color = vec3<f32>(color.x, color.y, color.z) * edge_strength;
    // final_color = quantize_and_dither(final_color, in.uv);

    return vec4<f32>(final_color.x, final_color.y, final_color.z, 1.0);
}
