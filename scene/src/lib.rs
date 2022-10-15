use anyhow::Result;
use cinder::cinder::Vertex;
use memmap::MmapOptions;
use rkyv::{Archive, Deserialize, Serialize};
use std::{fs::File, io::Write, path::Path};
use thiserror::Error;

const COMPILED_DIR: &str = "compiled_scenes";

#[derive(Debug, Error)]
pub enum CompiledSceneError {
    #[error("path did not have valid file name: {0:?}")]
    NoFileName(std::path::PathBuf),
    #[error("path contained invalid utf-8: {0:?}")]
    InvalidUtf8(std::path::PathBuf),
}

#[derive(Debug, Archive, Deserialize, Serialize)]
pub struct Mesh {
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex>,
}

#[derive(Debug, Archive, Deserialize, Serialize)]
pub struct ObjScene {
    pub meshes: Vec<Mesh>,
}

impl ObjScene {
    pub fn from_obj_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let (obj_models, _obj_materials) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS)?;

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

                let total_indices = mesh.indices.len();
                let (total_vertices, vertex_remap) =
                    meshopt::generate_vertex_remap(&src_vertices, Some(&mesh.indices));

                let indices = Vec::with_capacity(total_indices);
                unsafe {
                    meshopt::ffi::meshopt_remapIndexBuffer(
                        indices.as_ptr() as *mut ::std::os::raw::c_uint,
                        ::std::ptr::null(),
                        total_indices,
                        vertex_remap.as_ptr() as *const ::std::os::raw::c_uint,
                    );
                }

                let vertices = Vec::with_capacity(total_vertices);
                unsafe {
                    meshopt::ffi::meshopt_remapVertexBuffer(
                        vertices.as_ptr() as *mut ::std::os::raw::c_void,
                        src_vertices.as_ptr() as *const ::std::os::raw::c_void,
                        total_indices,
                        std::mem::size_of::<Vertex>(),
                        vertex_remap.as_ptr() as *const ::std::os::raw::c_uint,
                    );
                }

                Mesh { indices, vertices }
            })
            .collect::<Vec<_>>();

        Ok(Self { meshes })
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

    pub fn load_or_achive(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file_stem = path
            .file_stem()
            .ok_or_else(|| CompiledSceneError::NoFileName(path.to_owned()))?
            .to_str()
            .ok_or_else(|| CompiledSceneError::InvalidUtf8(path.to_owned()))?;
        let compiled_path = Path::new(COMPILED_DIR).join(format!("{}.akv", file_stem));
        if compiled_path.exists() {
            Self::from_archive_file(compiled_path)
        } else {
            let scene = Self::from_obj_path(path)?;
            std::fs::create_dir_all(COMPILED_DIR)?;
            scene.archive_to_file(&compiled_path)?;
            Ok(scene)
        }
    }
}
