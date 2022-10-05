use anyhow::Result;
use cinder::cinder::Vertex;
use std::path::Path;

pub struct Mesh {
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex>,
}

impl Mesh {
    pub fn from_obj_path(path: impl AsRef<Path>) -> Result<Vec<Self>> {
        let path = path.as_ref();
        let (obj_models, _obj_materials) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS)?;

        let meshes = obj_models
            .into_iter()
            .map(|model| {
                // TODO: The inner loop here could be more efficient, but I will instead serialize this to a
                // zero-copy custom file format after the first load
                let mesh = &model.mesh;

                let num_positions = mesh.positions.len() / 3;
                let vertices = (0..num_positions)
                    .map(|i| {
                        let color = if mesh.vertex_color.is_empty() {
                            [1.0, 1.0, 1.0, 1.0]
                        } else {
                            [
                                mesh.vertex_color[i * 3],
                                mesh.vertex_color[i * 3 + 1],
                                mesh.vertex_color[i * 3 + 2],
                                1.0,
                            ]
                        };
                        let uv = if mesh.texcoords.is_empty() {
                            [0.0, 0.0]
                        } else {
                            [mesh.texcoords[i * 2], mesh.texcoords[i * 2 + 1]]
                        };
                        Vertex {
                            pos: [
                                mesh.positions[i * 3],
                                mesh.positions[i * 3 + 1],
                                mesh.positions[i * 3 + 2],
                                1.0,
                            ],
                            color,
                            uv,
                        }
                    })
                    .collect::<Vec<_>>();

                Mesh {
                    indices: mesh.indices.clone(),
                    vertices,
                }
            })
            .collect::<Vec<_>>();

        Ok(meshes)
    }
}
