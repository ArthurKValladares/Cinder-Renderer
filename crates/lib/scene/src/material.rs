use zero_copy_assets::ImageData;

#[derive(Debug)]
pub struct Material {
    pub diffuse: Option<ImageData>,
}
