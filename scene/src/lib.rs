use anyhow::Result;
use cinder::cinder::Vertex;
use memmap::MmapOptions;
use meshopt::VertexDataAdapter;
use rkyv::{with::Skip, AlignedVec, Archive, Deserialize, Fallible, Serialize};
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

fn archive_to_file(bytes: AlignedVec, path: impl AsRef<Path>) -> Result<()> {
    // TODO: figure out N, use ScratchTracker.
    const N: usize = 256;
    let path = path.as_ref();
    let mut file = File::create(path).expect(&format!("Could not create file: {:?}", path));
    file.write_all(&bytes)?;
    Ok(())
}

#[derive(Debug, Archive, Deserialize, Serialize)]
pub struct ImageBuffer {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl ImageBuffer {
    // TODO: this can be a much more general pattern
    pub fn from_archive_file(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        let archived = unsafe { rkyv::archived_root::<Self>(&mmap[..]) };
        let ret = archived.deserialize(&mut rkyv::Infallible)?;
        Ok(ret)
    }
}
#[derive(Debug, Archive, Deserialize, Serialize)]
pub struct Material {
    pub diffuse_texture: String,
    pub archive_path: String,
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
    pub materials: Vec<Material>,
}

impl ObjScene {
    pub fn from_obj_path(
        root: impl AsRef<Path>,
        obj_relative: impl AsRef<Path>,
        archive_dir: impl AsRef<Path>,
    ) -> Result<Self> {
        let root = root.as_ref();
        let obj_relative = obj_relative.as_ref();
        let archive_dir = archive_dir.as_ref();

        let path = root.join(obj_relative);
        let (obj_models, obj_materials) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS)?;

        let materials = obj_materials?
            .into_iter()
            .map(|material| {
                let archive_path = if !material.diffuse_texture.is_empty() {
                    let diffuse_path = PathBuf::from(&material.diffuse_texture);
                    let material_stem = diffuse_path
                        .file_stem()
                        .ok_or_else(|| CompiledSceneError::NoFileName(obj_relative.to_owned()))?
                        .to_str()
                        .ok_or_else(|| CompiledSceneError::InvalidUtf8(obj_relative.to_owned()))?;
                    archive_dir.join(format!("{}.akvi", material_stem))
                } else {
                    archive_dir.join(format!("white.akvi"))
                };

                Ok(Material {
                    diffuse_texture: material.diffuse_texture,
                    archive_path: archive_path.to_str().unwrap().to_owned(),
                })
            })
            .collect::<Result<Vec<_>>>()?;

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
            materials,
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

    pub fn load_or_achive(
        root: impl AsRef<Path>,
        obj_relative: impl AsRef<Path>,
    ) -> Result<(Self, Vec<ImageBuffer>)> {
        const N: usize = 256;

        let root = root.as_ref();
        let obj_relative = obj_relative.as_ref();
        let file_stem = obj_relative
            .file_stem()
            .ok_or_else(|| CompiledSceneError::NoFileName(obj_relative.to_owned()))?
            .to_str()
            .ok_or_else(|| CompiledSceneError::InvalidUtf8(obj_relative.to_owned()))?;
        let scene_dir = Path::new(COMPILED_DIR).join(file_stem);
        let scene_path = scene_dir.join(format!("{}.akvs", file_stem));
        if scene_path.exists() {
            let mut ret = Self::from_archive_file(scene_path)?;
            let image_buffers = ret
                .materials
                .iter()
                .map(|material| ImageBuffer::from_archive_file(&material.archive_path))
                .collect::<Result<Vec<_>>>()?;
            ret.root = root.to_owned();
            Ok((ret, image_buffers))
        } else {
            let scene = Self::from_obj_path(root, obj_relative, &scene_dir)?;
            std::fs::create_dir_all(&scene_dir)?;
            rkyv::to_bytes::<_, N>(&scene)?;
            archive_to_file(rkyv::to_bytes::<_, N>(&scene)?, &scene_path)?;
            let image_buffers = scene
                .materials
                .iter()
                .map(|material| {
                    let image_buffer = {
                        let path = if material.diffuse_texture.is_empty() {
                            PathBuf::from("assets/textures/white.png")
                        } else {
                            root.join(&material.diffuse_texture)
                        };
                        let image = image::open(&path)
                            .expect(&format!("could not find image path: {:?}", path));
                        let image = image.flipv();
                        let image = image.to_rgba8();

                        let (image_width, image_height) = image.dimensions();
                        let image_data = image.into_raw();

                        ImageBuffer {
                            width: image_width,
                            height: image_height,
                            data: image_data,
                        }
                    };

                    archive_to_file(
                        rkyv::to_bytes::<_, N>(&image_buffer)?,
                        &material.archive_path,
                    )?;

                    Ok(image_buffer)
                })
                .collect::<Result<Vec<_>>>()?;

            Ok((scene, image_buffers))
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn materials(&self) -> &Vec<Material> {
        &self.materials
    }
}
