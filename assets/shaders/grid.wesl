// World-anchored background grid, adapted from bevy-falling-sand-editor `cursor_guide.wgsl`.
import bevy_sprite_render::mesh2d::vertex_output::VertexOutput;

struct GridMaterial {
    line_color: vec4<f32>,
    /// World units between gridlines on each axis.
    spacing: vec2<f32>,
    /// Line thickness in screen pixels (stays constant under zoom).
    line_width: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: GridMaterial;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // Position in grid cells; integer values lie on gridlines.
    let cell = mesh.world_position.xy / material.spacing;

    // Distance to the nearest gridline, converted from cells to screen pixels.
    let dist_cells = abs(fract(cell + 0.5) - 0.5);
    let dist_px = dist_cells / fwidth(cell);

    // One pixel of antialiasing on each side of the line.
    let half_line = material.line_width * 0.5;
    let line = 1.0 - smoothstep(vec2(half_line - 0.5), vec2(half_line + 0.5), dist_px);
    let grid_line = max(line.x, line.y);

    return vec4<f32>(material.line_color.rgb, grid_line * material.line_color.a);
}
