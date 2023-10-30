use egui_integration::{EguiIntegration, SharedEguiMenu};
use render_graph::PresentContext;
use renderer::shader_hot_reloader::HotReloaderState;
use sdl2::{event::Event, keyboard::Keycode, video::Window};
use util::SdlContext;

pub use egui_integration::egui::Context as DebugUiContext;
pub use render_graph::{AttachmentType, RenderGraph, RenderPass, RenderPassResource};
pub use renderer::{
    command_queue::{AttachmentLoadOp, AttachmentStoreOp, ClearValue, RenderAttachmentDesc},
    resources::{
        bind_group::{BindGroup, BindGroupBindInfo, BindGroupData, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::{Format, Image, ImageDescription, ImageUsage, Layout},
        pipeline::{
            graphics::{
                GraphicsPipeline, GraphicsPipelineDescription, VertexAttributeDescription,
                VertexBindingDesc, VertexDescription, VertexInputRate,
            },
            PipelineError,
        },
        sampler::{AddressMode, BorderColor, MipmapMode, Sampler, SamplerDescription},
        shader::ShaderDesc,
    },
    Renderer, ResourceId,
};
// TODO: Wrap
pub use bumpalo::Bump;

pub struct InitContext<'a> {
    pub renderer: &'a mut Renderer,
    pub shader_hot_reloader: &'a mut HotReloaderState,
}

pub trait App: Sized {
    // TODO: Explicit error type
    fn new(context: InitContext<'_>) -> anyhow::Result<Self>;
    fn draw<'a>(
        &'a mut self,
        allocator: &'a Bump,
        graph: &mut RenderGraph<'a>,
    ) -> anyhow::Result<()>;

    fn draw_debug_ui(&mut self, _context: &DebugUiContext) {}

    fn on_frame_start(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
    fn update(&mut self, _renderer: &mut Renderer) -> anyhow::Result<()> {
        Ok(())
    }
    fn on_event(&mut self, _event: &Event) -> anyhow::Result<()> {
        Ok(())
    }
    fn resize(
        &mut self,
        _renderer: &mut Renderer,
        _width: u32,
        _height: u32,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    fn cleanup(&mut self, _renderer: &mut Renderer) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct Cinder<A: App> {
    renderer: Renderer,
    allocator: Bump,
    egui: EguiIntegration,
    shared_egui_menu: SharedEguiMenu,
    // TODO: feature flag to disable, off by default in release (i.e. shader-hot-reload and shader-hot-reload-release features)
    shader_hot_reloader: HotReloaderState,
    app: A,
}

impl<A> Cinder<A>
where
    A: App,
{
    pub fn new(window: &Window) -> anyhow::Result<Self> {
        let (width, height) = window.drawable_size();
        // TODO: Pull ResourceManager out of renderer
        let mut renderer = Renderer::new(window, width, height)?;
        let allocator = Bump::new();
        let egui = EguiIntegration::new(
            &mut renderer.resource_manager,
            &renderer.device,
            &renderer.swapchain,
        )?;
        let shared_egui_menu = SharedEguiMenu::default();
        let mut shader_hot_reloader = HotReloaderState::new()?;

        let context = InitContext {
            renderer: &mut renderer,
            shader_hot_reloader: &mut shader_hot_reloader,
        };
        let app = A::new(context)?;

        Ok(Self {
            renderer,
            allocator,
            egui,
            shared_egui_menu,
            shader_hot_reloader,
            app,
        })
    }

    // TODO: Update function

    fn draw(&mut self) -> anyhow::Result<bool> {
        let present_context: anyhow::Result<PresentContext> = {
            let mut graph = RenderGraph::new(&self.allocator);
            self.app.draw(&self.allocator, &mut graph)?;
            let present_context = graph.run(&self.allocator, &mut self.renderer)?;
            Ok(present_context)
        };
        let present_context = present_context?;

        self.egui.run(
            &mut self.renderer.resource_manager,
            &self.renderer.device,
            &present_context.cmd_list,
            present_context.present_rect,
            present_context.swapchain_image,
            |ctx| {
                // TODO: Conditional draw
                self.shared_egui_menu.draw(ctx);
                self.app.draw_debug_ui(ctx);
            },
        )?;

        present_context.present(&mut self.renderer)
    }

    fn update(&mut self) -> anyhow::Result<()> {
        self.shared_egui_menu.update(&mut self.egui);
        self.app.update(&mut self.renderer)
    }

    fn resize(&mut self, width: u32, height: u32) -> anyhow::Result<()> {
        self.renderer.resize(width, height)?;
        self.egui.resize(width, height);
        self.app.resize(&mut self.renderer, width, height)?;
        Ok(())
    }

    fn init_hot_reloader(&mut self) {
        take_mut::take(&mut self.shader_hot_reloader, |hot_reloader| {
            hot_reloader.run()
        });
    }

    fn update_hot_reloader(&mut self) -> anyhow::Result<()> {
        for update_data in self.shader_hot_reloader.drain()? {
            if let Some(pipeline_shader_set) = self
                .shader_hot_reloader
                .get_pipeline(update_data.shader_handle)
            {
                self.renderer.device.recreate_shader(
                    &mut self.renderer.resource_manager,
                    update_data.shader_handle,
                    &update_data.bytes,
                )?;
                self.renderer.device.recreate_graphics_pipeline(
                    &mut self.renderer.resource_manager,
                    pipeline_shader_set.pipeline_handle,
                    pipeline_shader_set.vertex_handle,
                    Some(pipeline_shader_set.fragment_handle),
                )?;
            }
        }
        Ok(())
    }

    pub fn run_game_loop(&mut self, sdl: &mut SdlContext) -> anyhow::Result<()> {
        self.init_hot_reloader();

        'running: loop {
            self.allocator.reset();
            self.renderer.start_frame()?;

            self.app.on_frame_start()?;

            for event in sdl.event_pump.poll_iter() {
                self.app.on_event(&event)?;
                let response = self.egui.on_event(&event);
                if !response.consumed {
                    match event {
                        Event::Quit { .. }
                        | Event::KeyDown {
                            keycode: Some(Keycode::Escape),
                            ..
                        } => {
                            break 'running;
                        }
                        Event::Window {
                            win_event: sdl2::event::WindowEvent::SizeChanged(width, height),
                            ..
                        } => {
                            self.resize(width as u32, height as u32)?;
                        }
                        _ => {}
                    }
                }
            }

            self.update_hot_reloader()?;
            self.update()?;

            self.draw()?;

            self.renderer.end_frame();
        }
        Ok(())
    }
}

impl<A> Drop for Cinder<A>
where
    A: App,
{
    fn drop(&mut self) {
        self.renderer.device.wait_idle().ok();
        self.app.cleanup(&mut self.renderer).ok();
    }
}
