use anyhow::Result;
use image::DynamicImage;
use rayon::iter::*;
use std::path::Path;
use thiserror::Error;
pub use tobj::Mesh as ObjMesh;

#[derive(Debug, Error)]
pub enum SceneError {
    #[error("Could not open file at {path}: {err}")]
    FileError { err: std::io::Error, path: String },
    #[error(transparent)]
    ImageError(#[from] image::error::ImageError),
}

pub trait Vertex {
    fn from_obj_mesh_index(mesh: &ObjMesh, i: usize) -> Self;

    fn pos_3d(&self) -> [f32; 3];
}

#[derive(Debug)]
pub struct Material {
    diffuse: Option<DynamicImage>,
}

pub struct Mesh<V: Vertex> {
    pub indices: Vec<u32>,
    pub vertices: Vec<V>,
    pub min_pos: [f32; 3],
    pub max_pos: [f32; 3],
}

pub struct Scene<V: Vertex> {
    pub meshes: Vec<Mesh<V>>,
    pub materials: Vec<Material>,
    pub min_pos: [f32; 3],
    pub max_pos: [f32; 3],
}

impl<V> Scene<V>
where
    V: Vertex,
{
    pub fn from_obj(path: impl AsRef<Path>, file: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file = file.as_ref();
        let file_path = path.join(file);
        debug_assert!(file_path.exists(), "Path does not exist: {file_path:?}");
        let (models, materials) = tobj::load_obj(file_path, &tobj::GPU_LOAD_OPTIONS)?;
        let materials = materials;
        let materials = if let Ok(materials) = materials {
            materials
                .into_par_iter()
                .map(|material| {
                    let diffuse: Result<Option<DynamicImage>, SceneError> =
                        if material.diffuse_texture.is_empty() {
                            Ok(None)
                        } else {
                            let image_data = std::fs::read(path.join(&material.diffuse_texture))
                                .map_err(|err| SceneError::FileError {
                                    err,
                                    path: material.diffuse_texture,
                                })
                                .unwrap();
                            let image = image::load_from_memory(&image_data).unwrap();
                            Ok(Some(image))
                        };
                    let diffuse = diffuse?;

                    Ok(Material { diffuse })
                })
                .collect::<Result<Vec<_>, SceneError>>()?
        } else {
            vec![]
        };

        let mut min_pos = [f32::INFINITY; 3];
        let mut max_pos = [f32::NEG_INFINITY; 3];
        let mut meshes = Vec::new();
        for model in models {
            let mut mesh_min_pos = [f32::INFINITY; 3];
            let mut mesh_max_pos = [f32::NEG_INFINITY; 3];
            let mut vertices = Vec::new();

            let obj_mesh = model.mesh;

            for i in 0..obj_mesh.positions.len() / 3 {
                let vertex = V::from_obj_mesh_index(&obj_mesh, i);
                let pos = vertex.pos_3d();

                vertices.push(vertex);

                for i in 0..3 {
                    mesh_min_pos[i] = f32::min(mesh_min_pos[i], pos[i]);
                    mesh_max_pos[i] = f32::max(mesh_max_pos[i], pos[i]);
                }
            }

            for i in 0..3 {
                min_pos[i] = f32::min(min_pos[i], mesh_min_pos[i]);
                max_pos[i] = f32::max(max_pos[i], mesh_max_pos[i]);
            }

            meshes.push(Mesh {
                indices: obj_mesh.indices,
                vertices,
                min_pos: mesh_min_pos,
                max_pos: mesh_max_pos,
            });
        }

        Ok(Self {
            meshes,
            materials,
            min_pos,
            max_pos,
        })
    }
}
