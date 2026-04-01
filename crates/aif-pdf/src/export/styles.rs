/// Page size presets.
#[derive(Debug, Clone)]
pub enum PageSize {
    A4,
    Letter,
    Custom { width_pt: f32, height_pt: f32 },
}

impl PageSize {
    pub fn dimensions_pt(&self) -> (f32, f32) {
        match self {
            PageSize::A4 => (595.28, 841.89),
            PageSize::Letter => (612.0, 792.0),
            PageSize::Custom { width_pt, height_pt } => (*width_pt, *height_pt),
        }
    }
}

impl Default for PageSize {
    fn default() -> Self {
        PageSize::A4
    }
}

/// Margins in points.
#[derive(Debug, Clone)]
pub struct Margins {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Default for Margins {
    fn default() -> Self {
        Self {
            top: 72.0,
            right: 72.0,
            bottom: 72.0,
            left: 72.0,
        }
    }
}

/// PDF export options.
#[derive(Debug, Clone)]
pub struct PdfOptions {
    pub page_size: PageSize,
    pub margins: Margins,
    pub base_font_size: f32,
    pub tagged: bool,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            page_size: PageSize::default(),
            margins: Margins::default(),
            base_font_size: 12.0,
            tagged: false,
        }
    }
}
