use velato::{RenderSink, model::fixed};
use kurbo::{Affine, PathEl};
use peniko::BlendMode;

pub struct VanillaSink {
    pub paths: Vec<PathData>,
}

pub struct PathData {
    pub elements: Vec<PathEl>,
    pub transform: Affine,
    pub brush: fixed::Brush,
    pub stroke: Option<fixed::Stroke>,
}

impl VanillaSink {
    pub fn new() -> Self {
        Self {
            paths: Vec::new(),
        }
    }
}

impl RenderSink for VanillaSink {
    fn push_layer(
        &mut self,
        _blend: impl Into<BlendMode>,
        _alpha: f32,
        _transform: Affine,
        _shape: &impl kurbo::Shape,
    ) {
        // SVG backend might need this for grouping
    }

    fn push_clip_layer(&mut self, _transform: Affine, _shape: &impl kurbo::Shape) {
        // SVG clipping
    }

    fn pop_layer(&mut self) {
        // End grouping/clipping
    }

    fn draw(
        &mut self,
        stroke: Option<&fixed::Stroke>,
        transform: Affine,
        brush: &fixed::Brush,
        shape: &impl kurbo::Shape,
    ) {
        self.paths.push(PathData {
            elements: shape.path_elements(0.1).collect(),
            transform,
            brush: brush.clone(),
            stroke: stroke.cloned(),
        });
    }
}
