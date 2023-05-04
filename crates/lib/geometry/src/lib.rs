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
        let bci = N as u32;
        let tci = top_offset as u32 + N as u32;
        let ws = N * 6;
        for i in 0..N {
            let bi = i as u32;
            let nbi = (i as u32 + 1) % N as u32;

            let ti = top_offset as u32 + i as u32;
            let nti = top_offset as u32 + (i as u32 + 1) % N as u32;

            //Bottom triangle
            indices[i * 3] = bi;
            indices[i * 3 + 1] = nbi;
            indices[i * 3 + 2] = bci;

            // Top Triangle
            indices[N * 3 + i * 3] = nti;
            indices[N * 3 + i * 3 + 1] = ti;
            indices[N * 3 + i * 3 + 2] = tci;

            // Wall
            // First Triangle
            indices[ws + i * 6] = bi;
            indices[ws + i * 6 + 1] = ti;
            indices[ws + i * 6 + 2] = nbi;
            // Second Triangle
            indices[ws + i * 6 + 3] = nbi;
            indices[ws + i * 6 + 4] = ti;
            indices[ws + i * 6 + 5] = nti;
        }

        Self { vertices, indices }
    }
}
