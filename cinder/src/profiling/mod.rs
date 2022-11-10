use crate::device::Device;
use anyhow::Result;
use ash::vk;

const TIMESTAMP_COUNT: u32 = 128;

pub struct QueryPool {
    pub raw: vk::QueryPool,
    pub count: u32,
}

pub struct Profiling {
    pub timestamp_query_pool: QueryPool,
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
            },
        })
    }
}
