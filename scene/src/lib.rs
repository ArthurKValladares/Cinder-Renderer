use anyhow::Result;
use cinder::cinder::Vertex;
use memmap::MmapOptions;
use meshopt::VertexDataAdapter;
use rkyv::{with::Skip, Archive, Deserialize, Serialize};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
use thiserror::Error;

const COMPILED_DIR: &str = "compiled_scenes";

#[derive(Debug, Error)]
pub enum CompiledSceneError {
    #[error("path did not have valid file name: {0:?}")]
    NoFileName(std::path::PathBuf),
    #[error("path contained invalid utf-8: {0:?}")]
    InvalidUtf8(std::path::PathBuf),
}

#[derive(Debug)]
pub struct Material {
    pub diffuse_texture: String,
}

#[derive(Debug, Archive, Deserialize, Serialize)]
pub struct Mesh {
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex>,
    pub material_index: Option<usize>,
}

#[derive(Debug, Archive, Deserialize, Serialize)]
pub struct ObjScene {
    #[with(Skip)]
    pub root: PathBuf,
    pub meshes: Vec<Mesh>,
}

impl ObjScene {
    pub fn from_obj_path(root: impl AsRef<Path>, obj_relative: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        let obj_relative = obj_relative.as_ref();
        let path = root.join(obj_relative);
        let (obj_models, obj_materials) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS)?;

        let materials = obj_materials?
            .into_iter()
            .map(|material| Material {
                diffuse_texture: material.diffuse_texture,
            })
            .collect::<Vec<_>>();

        let meshes = obj_models
            .into_iter()
            .map(|model| {
                // TODO: The inner loop here could be more efficient, but I will instead serialize this to a
                // zero-copy custom file format after the first load
                let mesh = &model.mesh;

                let num_positions = mesh.positions.len() / 3;
                let src_vertices = (0..num_positions)
                    .map(|i| {
                        let pos = [
                            mesh.positions[i * 3],
                            mesh.positions[i * 3 + 1],
                            mesh.positions[i * 3 + 2],
                            1.0,
                        ];
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
                        Vertex { pos, color, uv }
                    })
                    .collect::<Vec<_>>();

                let (total_vertices, vertex_remap) =
                    meshopt::generate_vertex_remap(&src_vertices, Some(&mesh.indices));

                let mut indices =
                    meshopt::remap_index_buffer(Some(&mesh.indices), total_vertices, &vertex_remap);

                let mut vertices =
                    meshopt::remap_vertex_buffer(&src_vertices, src_vertices.len(), &vertex_remap);

                meshopt::optimize_vertex_cache_in_place(&mut indices, vertices.len());

                let vertex_data_adapter = {
                    let position_offset = util::offset_of!(Vertex, pos);
                    let vertex_stride = std::mem::size_of::<Vertex>();
                    let vertex_data = util::typed_to_bytes(&vertices);

                    VertexDataAdapter::new(vertex_data, vertex_stride, position_offset)
                        .expect("failed to create vertex data reader")
                };
                let threshold = 1.05f32;
                meshopt::optimize_overdraw_in_place(&indices, &vertex_data_adapter, threshold);

                Mesh {
                    indices,
                    vertices,
                    material_index: mesh.material_id,
                }
            })
            .collect::<Vec<_>>();

        Ok(Self {
            root: root.to_owned(),
            meshes,
        })
    }

    // TODO: this can be a much more general pattern
    pub fn from_archive_file(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        let archived = unsafe { rkyv::archived_root::<Self>(&mmap[..]) };
        let ret = archived.deserialize(&mut rkyv::Infallible)?;
        Ok(ret)
    }

    pub fn archive_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        // TODO: figure out N, use ScratchTracker.
        const N: usize = 256;
        let bytes = rkyv::to_bytes::<_, N>(self)?;
        let path = path.as_ref();
        let mut file = File::create(path)?;
        file.write_all(&bytes)?;
        Ok(())
    }

    pub fn load_or_achive(root: impl AsRef<Path>, obj_relative: impl AsRef<Path>) -> Result<Self> {
        let obj_relative = obj_relative.as_ref();
        let file_stem = obj_relative
            .file_stem()
            .ok_or_else(|| CompiledSceneError::NoFileName(obj_relative.to_owned()))?
            .to_str()
            .ok_or_else(|| CompiledSceneError::InvalidUtf8(obj_relative.to_owned()))?;
        let compiled_path = Path::new(COMPILED_DIR).join(format!("{}.akv", file_stem));
        if compiled_path.exists() {
            Self::from_archive_file(compiled_path)
        } else {
            let scene = Self::from_obj_path(root, obj_relative)?;
            std::fs::create_dir_all(COMPILED_DIR)?;
            scene.archive_to_file(&compiled_path)?;
            Ok(scene)
        }
    }
}
