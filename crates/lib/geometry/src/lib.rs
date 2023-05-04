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
        let mut indices: Vec<u32> = vec![0; (N + 1) * 6 + 6];

        for i in 0..N {
            let ratio = i as f32 / N as f32;
            let angle = ratio * std::f32::consts::PI * 2.0;
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;
            vertices[i] = Vertex { pos: [x, 0.0, z] };
            vertices[i + top_offset] = Vertex {
                pos: [x, height, z],
            };
        }
        vertices[N] = Vertex {
            pos: [0.0, 0.0, 0.0],
        };
        vertices[N + top_offset] = Vertex {
            pos: [0.0, height, 0.0],
        };

        // Generate indices
        for i in 0..N {
            let to = top_offset as u32;

            //Bottom triangle
            indices[i * 3] = i as u32;
            indices[i * 3 + 1] = (i as u32 + 1) % N as u32;
            indices[i * 3 + 2] = N as u32;

            // Top Triangle
            indices[N * 3 + i * 3] = to + (i as u32 + 1) % N as u32;
            indices[N * 3 + i * 3 + 1] = to + i as u32;
            indices[N * 3 + i * 3 + 2] = to + N as u32;
        }

        Self { vertices, indices }
    }
}
