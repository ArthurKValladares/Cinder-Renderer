use anyhow::Result;
use cinder::device::Vertex;
use std::path::Path;

pub struct Mesh {
    indices: Vec<u32>,
    vertices: Vec<Vertex>,
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
                let positions = {
                    let m_positions = &mesh.positions;
                    let mut positions = Vec::with_capacity(num_positions);
                    for i in 0..num_positions {
                        positions.push([
                            m_positions[i],
                            m_positions[i + 1],
                            m_positions[i + 2],
                            0.0,
                        ]);
                    }
                    positions
                };

                let colors = if mesh.vertex_color.is_empty() {
                    vec![[0.0f32, 0.0, 0.0, 0.0]; num_positions]
                } else {
                    let num_colors = mesh.vertex_color.len() / 3;
                    assert!(
                        num_colors == num_positions,
                        "Mesh contains a different number of colors and positions"
                    );
                    let m_colors = &mesh.vertex_color;
                    let mut colors = Vec::with_capacity(num_colors);
                    for i in 0..num_colors {
                        colors.push([m_colors[i], m_colors[i + 1], m_colors[i + 2], 1.0]);
                    }
                    colors
                };

                let uvs = if mesh.texcoords.is_empty() {
                    vec![[0.0f32, 0.0]; num_positions]
                } else {
                    let num_uvs = mesh.texcoords.len() / 2;
                    assert!(
                        num_uvs == num_positions,
                        "Mesh contains a different number of uvs and positions"
                    );
                    let m_uvs = &mesh.texcoords;
                    let mut uvs = Vec::with_capacity(num_uvs);
                    for i in 0..num_uvs {
                        uvs.push([m_uvs[i], m_uvs[i + 1]]);
                    }
                    uvs
                };

                let vertices = positions
                    .into_iter()
                    .zip(colors.into_iter())
                    .zip(uvs.into_iter())
                    .map(|((pos, color), uv)| Vertex { pos, color, uv })
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
