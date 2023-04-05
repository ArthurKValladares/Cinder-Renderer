mod material;
mod mesh;
pub mod primitives;
mod vertex;

use anyhow::Result;
use image::DynamicImage;
use rayon::iter::*;
use std::path::Path;
use thiserror::Error;
pub use {material::*, mesh::*, vertex::*};

#[derive(Debug, Error)]
pub enum SceneError {
    #[error("Could not open file at {path}: {err}")]
    FileError { err: std::io::Error, path: String },
    #[error(transparent)]
    ImageError(#[from] image::error::ImageError),
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
        let mut meshes = Vec::with_capacity(models.len());
        for model in models {
            let mesh = Mesh::from_obj_model(model);

            for i in 0..3 {
                min_pos[i] = f32::min(min_pos[i], mesh.min_pos[i]);
                max_pos[i] = f32::max(max_pos[i], mesh.max_pos[i]);
            }

            meshes.push(mesh);
        }

        Ok(Self {
            meshes,
            materials,
            min_pos,
            max_pos,
        })
    }
}
