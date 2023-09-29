mod vertex;
use vertex::Vertex;

pub struct SurfaceMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl SurfaceMesh {
    pub fn cylinder<const N: usize>(height: f32, radius: f32) -> Self {
        let top_offset = N + 1;
        let mut vertices: Vec<Vertex> = vec![Default::default(); (N + 1) * 2];
        let mut indices: Vec<u32> = vec![0; N * 6 * 2];

        for i in 0..N {
            let ratio = i as f32 / N as f32;
            let angle = ratio * std::f32::consts::PI * 2.0;
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;

            // Create outer ring vertices for bottom and top plane
            vertices[i] = Vertex { pos: [x, 0.0, z] };
            vertices[i + top_offset] = Vertex {
                pos: [x, height, z],
            };
        }
        // Create center vertices for bottom and top plane
        vertices[N] = Vertex {
            pos: [0.0, 0.0, 0.0],
        };
        vertices[N + top_offset] = Vertex {
            pos: [0.0, height, 0.0],
        };

        // Generate indices
        let bottom_center_index = N as u32;
        let top_center_index = top_offset as u32 + N as u32;
        let wall_size = N * 6;
        for i in 0..N {
            let bottom_index = i as u32;
            let next_bottom_index = (i as u32 + 1) % N as u32;

            let top_index = top_offset as u32 + i as u32;
            let next_top_index = top_offset as u32 + (i as u32 + 1) % N as u32;

            //Bottom triangle
            indices[i * 3] = bottom_index;
            indices[i * 3 + 1] = next_bottom_index;
            indices[i * 3 + 2] = bottom_center_index;

            // Top Triangle
            indices[N * 3 + i * 3] = next_top_index;
            indices[N * 3 + i * 3 + 1] = top_index;
            indices[N * 3 + i * 3 + 2] = top_center_index;

            // Wall
            // First Triangle
            indices[wall_size + i * 6] = bottom_index;
            indices[wall_size + i * 6 + 1] = top_index;
            indices[wall_size + i * 6 + 2] = next_bottom_index;
            // Second Triangle
            indices[wall_size + i * 6 + 3] = next_bottom_index;
            indices[wall_size + i * 6 + 4] = top_index;
            indices[wall_size + i * 6 + 5] = next_top_index;
        }

        Self { vertices, indices }
    }

    pub fn cone<const N: usize>(height: f32, radius: f32) -> Self {
        let top_offset = N + 1;
        let mut vertices: Vec<Vertex> = vec![Default::default(); N + 2];
        let mut indices: Vec<u32> = vec![0; N * 6];

        for i in 0..N {
            let ratio = i as f32 / N as f32;
            let angle = ratio * std::f32::consts::PI * 2.0;
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;
            // Create outer ring vertices for bottom plane
            vertices[i] = Vertex { pos: [x, 0.0, z] };
        }
        // Create center vertices for bottom plane
        vertices[N] = Vertex {
            pos: [0.0, 0.0, 0.0],
        };
        // Tip vertex
        vertices[N + 1] = Vertex {
            pos: [0.0, height, 0.0],
        };

        // Generate indices
        let bottom_center_index = N as u32;
        let top_center_index = (N + 1) as u32;
        let ws = N * 3;
        for i in 0..N {
            let bottom_index = i as u32;
            let next_bottom_index = (i as u32 + 1) % N as u32;

            //Bottom triangle
            indices[i * 3] = bottom_index;
            indices[i * 3 + 1] = next_bottom_index;
            indices[i * 3 + 2] = bottom_center_index;

            // Wall
            indices[ws + i * 3] = bottom_index;
            indices[ws + i * 3 + 1] = top_center_index;
            indices[ws + i * 3 + 2] = next_bottom_index;
        }

        Self { vertices, indices }
    }
}
