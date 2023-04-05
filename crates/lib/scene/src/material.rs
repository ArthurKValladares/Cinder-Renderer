use image::DynamicImage;

#[derive(Debug)]
pub struct Material {
    pub diffuse: Option<DynamicImage>,
}
