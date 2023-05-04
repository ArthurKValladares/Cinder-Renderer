mod vertex;
use vertex::Vertex;

pub struct SurfaceMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl SurfaceMesh {
    pub fn cylinder<const N: usize>(height: f32, radius: f32) -> Self {
        let mut vertices: Vec<Vertex> = Default::default();
        let mut indices: Vec<u32> = Default::default();

        // Generate top and bottom vertices
        let mut bottom_vertices = [Vertex::default(); N];
        let mut top_vertices = [Vertex::default(); N];
        for i in 0..N - 1 {
            let ratio = i as f32 / N as f32;
            let angle = ratio * std::f32::consts::PI * 2.0;
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;
            bottom_vertices[i] = Vertex { pos: [x, 0.0, z] };
            top_vertices[i] = Vertex {
                pos: [x, height, z],
            };
        }
        bottom_vertices[N - 1] = Vertex {
            pos: [0.0, 0.0, 0.0],
        };
        top_vertices[N - 1] = Vertex {
            pos: [0.0, height, 0.0],
        };

        // Generate indices
        let last_idx = (N - 1) as u32;
        let mut bottom_indices = vec![];
        let mut top_indices = vec![];
        for i in 0..N - 1 {
            let i = i as u32;
            bottom_indices.extend([i, i + 1, last_idx]);
            top_indices.extend([i + 1, i, last_idx]);
        }
        bottom_indices.extend([last_idx - 1, 0, last_idx]);
        top_indices.extend([0, last_idx - 1, last_idx]);

        // Move all to final vector (wont exist as step at the end)
        vertices.extend(bottom_vertices);
        vertices.extend(top_vertices);

        indices.extend(bottom_indices);
        indices.extend(top_indices);

        Self { vertices, indices }
    }
}
