use anyhow::Result;
use cinder::{
    resources::{pipeline::graphics::GraphicsPipeline, shader::Shader},
    ResourceId,
};
use notify_debouncer_mini::{
    new_debouncer,
    notify::{self, RecommendedWatcher, RecursiveMode},
    DebouncedEvent, Debouncer,
};
use rust_shader_tools::{EnvVersion, OptimizationLevel, ShaderCompiler, ShaderStage};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{mpsc::Receiver, Arc, Mutex, MutexGuard},
    time::Duration,
};

#[derive(Debug)]
pub struct UpdateData {
    pub shader_handle: ResourceId<Shader>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct UpdateList {
    inner: Option<Vec<UpdateData>>,
}

impl UpdateList {
    pub fn push(&mut self, data: UpdateData) {
        if self.inner.is_none() {
            self.inner = Some(Default::default());
        }
        self.inner.as_mut().unwrap().push(data);
    }

    pub fn into_iter(&mut self) -> impl Iterator<Item = UpdateData> {
        self.inner.take().unwrap_or_default().into_iter()
    }
}

pub struct ShaderHotReloader {
    _watcher: Debouncer<RecommendedWatcher>,
    program_map: HashMap<ResourceId<Shader>, PipelineShaderIdSet>,
    // TODO: If I make the Device thread-safe, I don't need this
    to_be_updated: Arc<Mutex<UpdateList>>,
}

#[derive(Debug, Clone, Copy)]
pub struct PipelineShaderIdSet {
    pub pipeline_handle: ResourceId<GraphicsPipeline>,
    pub vertex_handle: ResourceId<Shader>,
    pub fragment_handle: ResourceId<Shader>,
}

pub struct ShaderHotReloaderRunner {
    watcher: Debouncer<RecommendedWatcher>,
    receiver: Receiver<Result<Vec<DebouncedEvent>, Vec<notify::Error>>>,
    shader_map: HashMap<PathBuf, (ResourceId<Shader>, ShaderStage)>,
    program_map: HashMap<ResourceId<Shader>, PipelineShaderIdSet>,
}

impl ShaderHotReloaderRunner {
    pub fn new() -> Result<Self> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let watcher = new_debouncer(Duration::from_secs_f64(0.1), None, sender)?;
        Ok(Self {
            watcher,
            receiver,
            shader_map: Default::default(),
            program_map: Default::default(),
        })
    }

    pub fn set_graphics(
        &mut self,
        absolute_vertex_path: impl AsRef<Path>,
        vertex_handle: ResourceId<Shader>,
        absolute_fragment_path: impl AsRef<Path>,
        fragment_handle: ResourceId<Shader>,
        pipeline_handle: ResourceId<GraphicsPipeline>,
    ) -> Result<()> {
        let pipeline_shader_set = PipelineShaderIdSet {
            pipeline_handle,
            vertex_handle,
            fragment_handle,
        };

        let absolute_vertex_path = absolute_vertex_path.as_ref();
        debug_assert!(
            absolute_vertex_path.is_absolute(),
            "paths passed to shader hot reloader must be absolute: {absolute_vertex_path:?}"
        );
        self.watcher
            .watcher()
            .watch(absolute_vertex_path, RecursiveMode::NonRecursive)?;
        self.shader_map.insert(
            absolute_vertex_path.to_path_buf(),
            (vertex_handle, ShaderStage::Vertex),
        );
        self.program_map.insert(vertex_handle, pipeline_shader_set);

        let absolute_fragment_path = absolute_fragment_path.as_ref();
        debug_assert!(
            absolute_fragment_path.is_absolute(),
            "paths passed to shader hot reloader must be absolute: {absolute_fragment_path:?}"
        );
        self.watcher
            .watcher()
            .watch(absolute_fragment_path, RecursiveMode::NonRecursive)?;
        self.shader_map.insert(
            absolute_fragment_path.to_path_buf(),
            (fragment_handle, ShaderStage::Fragment),
        );
        self.program_map
            .insert(fragment_handle, pipeline_shader_set);

        Ok(())
    }

    pub fn run(self) -> ShaderHotReloader {
        let Self {
            watcher,
            receiver,
            mut shader_map,
            program_map,
        } = self;

        let shader_compiler =
            ShaderCompiler::new(EnvVersion::Vulkan1_2, OptimizationLevel::Zero, None)
                .expect("Could not create shader compiler");
        let to_be_updated = Arc::<Mutex<_>>::default();
        let to_be_updated_arc = Arc::clone(&to_be_updated);
        std::thread::spawn(move || loop {
            match receiver.recv() {
                Ok(event) => {
                    match event {
                        Ok(events) => {
                            for event in &events {
                                if let Some((handle, stage)) = shader_map.get_mut(&event.path) {
                                    println!("{event:#?}");
                                    let artifact = shader_compiler
                                        .compile_shader(&event.path, *stage)
                                        .expect("failed to compiler shader");
                                    let mut lock: MutexGuard<UpdateList> =
                                        to_be_updated_arc.lock().expect("mutex lock poisoned");
                                    lock.push(UpdateData {
                                        shader_handle: *handle,
                                        bytes: artifact.as_binary_u8().to_vec(),
                                    });
                                }
                            }
                        }
                        Err(err) => {
                            println!("Shader hot-reload error: {err:?}");
                        }
                    };
                }
                Err(_) => {
                    println!("Shader Hot-Reloader Stopped");
                    break;
                }
            }
        });

        ShaderHotReloader {
            _watcher: watcher,
            program_map,
            to_be_updated,
        }
    }
}

impl ShaderHotReloader {
    pub fn drain(&mut self) -> Result<impl Iterator<Item = UpdateData>> {
        let mut lock: MutexGuard<UpdateList> =
            self.to_be_updated.lock().expect("Mutex lock poisoned");
        Ok(lock.into_iter())
    }

    pub fn get_pipeline(&self, handle: ResourceId<Shader>) -> Option<&PipelineShaderIdSet> {
        self.program_map.get(&handle)
    }
}
