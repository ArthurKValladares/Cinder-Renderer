use crate::{context::render_context::RenderContext, device::Device};
use anyhow::Result;
use ash::vk;

const TIMESTAMP_COUNT: u32 = 128;

pub struct QueryPool {
    pub raw: vk::QueryPool,
    pub count: u32,
    pub current_query: u32,
}

pub struct Profiling {
    timestamp_query_pool: QueryPool,
}

impl Profiling {
    pub fn new(device: &Device) -> Result<Self> {
        let timestamp_query_pool_ci = vk::QueryPoolCreateInfo::builder()
            .query_type(vk::QueryType::TIMESTAMP)
            .query_count(TIMESTAMP_COUNT)
            .build();

        let timestamp_query_pool =
            unsafe { device.create_query_pool(&timestamp_query_pool_ci, None) }?;

        Ok(Self {
            timestamp_query_pool: QueryPool {
                raw: timestamp_query_pool,
                count: TIMESTAMP_COUNT,
                current_query: 0,
            },
        })
    }

    pub fn write_timestamp(&mut self, device: &Device, context: &RenderContext) {
        context.write_timestamp(device, &self.timestamp_query_pool);
        self.timestamp_query_pool.current_query += 1;
    }

    pub fn reset(&mut self, device: &Device, context: &RenderContext) {
        context.reset_query_pool(device, &self.timestamp_query_pool);
        self.timestamp_query_pool.current_query = 0;
    }
}
