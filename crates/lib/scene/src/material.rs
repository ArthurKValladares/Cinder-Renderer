use rkyv::{Archive, Deserialize, Serialize};
use zero_copy_assets::ImageData;

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct Material {
    pub diffuse: Option<ImageData>,
}
