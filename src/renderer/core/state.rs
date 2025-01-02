use color_eyre::Result;

/// Contains often-mutated flags and other state information
pub struct RenderState {
    pub resize_requested: bool,
}

impl RenderState {
    pub fn new() -> Result<Self> {
        Ok(Self {
            resize_requested: false,
        })
    }
}