use super::shader::Program;
use crate::resource_pool::Handle;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct PipelineState {
    pub program_handle: Handle<Program>,
    // TODO: There will be a ton more stuff in here later
}
