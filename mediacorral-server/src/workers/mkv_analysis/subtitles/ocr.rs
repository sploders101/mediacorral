use std::{
    collections::HashMap,
    io::Cursor,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use image::GrayImage;
use leptess::{leptonica, LepTess, Variable};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PartessError {
    #[error("Failed to initialize leptess:\n{0}")]
    InitError(#[from] leptess::tesseract::TessInitError),
    #[error("Failed to set a leptess variable:\n{0}")]
    VariableError(#[from] leptess::tesseract::TessSetVariableError),
    #[error("OCR output was not valid UTF-8")]
    InvalidOcrOutput,
    #[error("An error occurred while processing the image:\n{0}")]
    ImageError(#[from] image::ImageError),
    #[error("Leptonica could not process the image:\n{0}")]
    LeptonicaPixError(#[from] leptonica::PixError),
}

pub struct PartessInstance(Arc<PartessInner>, Option<LepTess>);
impl PartessInstance {
    pub fn ocr_image(&mut self, image: GrayImage) -> Result<String, PartessError> {
        let leptess = self.1.as_mut().unwrap();
        let mut img_bytes: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        image.write_to(&mut img_bytes, image::ImageFormat::Pnm)?;
        leptess.set_image_from_mem(img_bytes.get_ref())?;
        return Ok(leptess
            .get_utf8_text()
            .map_err(|_| PartessError::InvalidOcrOutput)?);
    }
}
impl Deref for PartessInstance {
    type Target = LepTess;
    fn deref(&self) -> &Self::Target {
        return self.1.as_ref().unwrap();
    }
}
impl DerefMut for PartessInstance {
    fn deref_mut(&mut self) -> &mut Self::Target {
        return self.1.as_mut().unwrap();
    }
}
impl Drop for PartessInstance {
    fn drop(&mut self) {
        let leptess = self.1.take().unwrap();
        let mut instances = self.0.instances.lock().unwrap();
        instances.push(leptess);
    }
}

/// A paralellizable LepTess instance that creates a new instance
/// whenever it is used on a new thread.
pub struct Partess(Arc<PartessInner>);
struct PartessInner {
    instances: Mutex<Vec<LepTess>>,
    language: String,
    variables: Vec<(Variable, String)>,
}
impl Partess {
    pub fn new(language: String, variables: Vec<(Variable, String)>) -> Self {
        return Partess(Arc::new(PartessInner {
            instances: Mutex::new(Vec::new()),
            language,
            variables,
        }));
    }
    fn new_instance(&self) -> Result<LepTess, PartessError> {
        let mut leptess = LepTess::new(None, &self.0.language)?;
        for (key, value) in self.0.variables.iter() {
            leptess.set_variable(key.clone(), value)?;
        }
        return Ok(leptess);
    }
    pub fn get(&self) -> Result<PartessInstance, PartessError> {
        let mut instances = self.0.instances.lock().unwrap();
        return Ok(match instances.pop() {
            Some(instance) => PartessInstance(Arc::clone(&self.0), Some(instance)),
            None => {
                let new_instance = self.new_instance()?;
                PartessInstance(Arc::clone(&self.0), Some(new_instance))
            }
        });
    }
}
impl Clone for Partess {
    fn clone(&self) -> Self {
        return Self(Arc::clone(&self.0));
    }
}

pub struct PartessCache {
    pub cache: Arc<Mutex<HashMap<String, Partess>>>,
}
impl PartessCache {
    pub fn new() -> Self {
        return Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        };
    }
}
impl Clone for PartessCache {
    fn clone(&self) -> Self {
        return Self {
            cache: Arc::clone(&self.cache),
        };
    }
}
