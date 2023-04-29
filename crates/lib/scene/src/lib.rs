mod material;
mod mesh;
pub mod primitives;
mod vertex;

use anyhow::Result;
use rayon::iter::*;
use rkyv::{Archive, Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use zero_copy_assets::{try_decoded_file, ImageData, LoadFromPath, ZeroCopyError};
pub use {material::*, mesh::*, vertex::*};

#[derive(Debug, Error)]
pub enum SceneError {
    #[error("Could not open file at {path}: {err}")]
    FileError { err: std::io::Error, path: String },
    #[error(transparent)]
    ImageError(#[from] zero_copy_assets::ZeroCopyError),
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct Scene<V: Vertex> {
    pub min_pos: [f32; 3],
    pub max_pos: [f32; 3],
    pub meshes: Vec<Mesh<V>>,
    pub materials: Vec<Material>,
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
                    let diffuse: Result<Option<ImageData>, SceneError> =
                        if material.diffuse_texture.is_empty() {
                            Ok(None)
                        } else {
                            let material_path = material.diffuse_texture.replace("\\", &format!("{}", std::path::MAIN_SEPARATOR));
                            let image_path = path.join(material_path);
                            let image_stem = image_path.file_stem().unwrap();
                            let image = try_decoded_file::<ImageData>(
                                &image_path,
                                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                                    .join("assets")
                                    .join("gen")
                                    .join(format!("{}.adi", image_stem.to_str().unwrap())),
                            )
                            .unwrap();
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

impl<V> LoadFromPath for Scene<V>
where
    V: Vertex,
{
    fn from_resource_path(
        path: impl AsRef<Path>,
    ) -> std::result::Result<Self, zero_copy_assets::ZeroCopyError> {
        let path = path.as_ref();

        let file = path
            .file_name()
            .ok_or_else(|| ZeroCopyError::InvalidUtf8(path.to_owned()))?;
        let parent = path
            .parent()
            .ok_or_else(|| ZeroCopyError::InvalidUtf8(path.to_owned()))?;

        let ret = Scene::<V>::from_obj(parent, file)
            .map_err(|err| ZeroCopyError::Fallback(err.to_string()))?;
        Ok(ret)
    }
}
