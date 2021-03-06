use lyon::path::Path;
use lyon::tessellation::{
    self,
    geometry_builder::{BuffersBuilder, FillVertexConstructor, VertexBuffers},
    FillAttributes, FillTessellator, StrokeAttributes, StrokeTessellator, StrokeVertexConstructor,
};
use lyon::tessellation::{FillOptions, StrokeOptions};
use ruffle_core::backend::render::swf::{self, FillStyle, Twips};
use ruffle_core::shape_utils::{DistilledShape, DrawCommand, DrawPath};

pub struct ShapeTessellator {
    fill_tess: FillTessellator,
    stroke_tess: StrokeTessellator,
}

impl ShapeTessellator {
    pub fn new() -> Self {
        Self {
            fill_tess: FillTessellator::new(),
            stroke_tess: StrokeTessellator::new(),
        }
    }

    pub fn tessellate_shape<F>(&mut self, shape: DistilledShape, get_bitmap_dimensions: F) -> Mesh
    where
        F: Fn(swf::CharacterId) -> Option<(u32, u32)>,
    {
        let mut mesh = Vec::new();

        let mut lyon_mesh: VertexBuffers<_, u32> = VertexBuffers::new();

        fn flush_draw(draw: DrawType, mesh: &mut Mesh, lyon_mesh: &mut VertexBuffers<Vertex, u32>) {
            if lyon_mesh.vertices.is_empty() {
                return;
            }

            let draw_mesh = std::mem::replace(lyon_mesh, VertexBuffers::new());
            mesh.push(Draw {
                draw_type: draw,
                vertices: draw_mesh.vertices,
                indices: draw_mesh.indices,
            });
        }

        for path in shape.paths {
            match path {
                DrawPath::Fill { style, commands } => match style {
                    FillStyle::Color(color) => {
                        let color = ((color.a as u32) << 24)
                            | ((color.b as u32) << 16)
                            | ((color.g as u32) << 8)
                            | (color.r as u32);

                        let mut buffers_builder =
                            BuffersBuilder::new(&mut lyon_mesh, RuffleVertexCtor { color });

                        if let Err(e) = self.fill_tess.tessellate_path(
                            &ruffle_path_to_lyon_path(commands, true),
                            &FillOptions::even_odd(),
                            &mut buffers_builder,
                        ) {
                            // This may just be a degenerate path; skip it.
                            log::error!("Tessellation failure: {:?}", e);
                            continue;
                        }
                    }
                    FillStyle::LinearGradient(gradient) => {
                        flush_draw(DrawType::Color, &mut mesh, &mut lyon_mesh);

                        let mut buffers_builder = BuffersBuilder::new(
                            &mut lyon_mesh,
                            RuffleVertexCtor { color: 0xffff_ffff },
                        );

                        if let Err(e) = self.fill_tess.tessellate_path(
                            &ruffle_path_to_lyon_path(commands, true),
                            &FillOptions::even_odd(),
                            &mut buffers_builder,
                        ) {
                            // This may just be a degenerate path; skip it.
                            log::error!("Tessellation failure: {:?}", e);
                            continue;
                        }

                        let mut colors: Vec<[f32; 4]> = Vec::with_capacity(8);
                        let mut ratios: Vec<f32> = Vec::with_capacity(8);
                        for record in &gradient.records {
                            colors.push([
                                f32::from(record.color.r) / 255.0,
                                f32::from(record.color.g) / 255.0,
                                f32::from(record.color.b) / 255.0,
                                f32::from(record.color.a) / 255.0,
                            ]);
                            ratios.push(f32::from(record.ratio) / 255.0);
                        }

                        let gradient = Gradient {
                            gradient_type: GradientType::Linear,
                            ratios,
                            colors,
                            num_colors: gradient.records.len() as u32,
                            matrix: swf_to_gl_matrix(gradient.matrix.clone()),
                            repeat_mode: gradient.spread,
                            focal_point: 0.0,
                        };

                        flush_draw(DrawType::Gradient(gradient), &mut mesh, &mut lyon_mesh);
                    }
                    FillStyle::RadialGradient(gradient) => {
                        flush_draw(DrawType::Color, &mut mesh, &mut lyon_mesh);

                        let mut buffers_builder = BuffersBuilder::new(
                            &mut lyon_mesh,
                            RuffleVertexCtor { color: 0xffff_ffff },
                        );

                        if let Err(e) = self.fill_tess.tessellate_path(
                            &ruffle_path_to_lyon_path(commands, true),
                            &FillOptions::even_odd(),
                            &mut buffers_builder,
                        ) {
                            // This may just be a degenerate path; skip it.
                            log::error!("Tessellation failure: {:?}", e);
                            continue;
                        }

                        let mut colors: Vec<[f32; 4]> = Vec::with_capacity(8);
                        let mut ratios: Vec<f32> = Vec::with_capacity(8);
                        for record in &gradient.records {
                            colors.push([
                                f32::from(record.color.r) / 255.0,
                                f32::from(record.color.g) / 255.0,
                                f32::from(record.color.b) / 255.0,
                                f32::from(record.color.a) / 255.0,
                            ]);
                            ratios.push(f32::from(record.ratio) / 255.0);
                        }

                        let gradient = Gradient {
                            gradient_type: GradientType::Radial,
                            ratios,
                            colors,
                            num_colors: gradient.records.len() as u32,
                            matrix: swf_to_gl_matrix(gradient.matrix.clone()),
                            repeat_mode: gradient.spread,
                            focal_point: 0.0,
                        };

                        flush_draw(DrawType::Gradient(gradient), &mut mesh, &mut lyon_mesh);
                    }
                    FillStyle::FocalGradient {
                        gradient,
                        focal_point,
                    } => {
                        flush_draw(DrawType::Color, &mut mesh, &mut lyon_mesh);

                        let mut buffers_builder = BuffersBuilder::new(
                            &mut lyon_mesh,
                            RuffleVertexCtor { color: 0xffff_ffff },
                        );

                        if let Err(e) = self.fill_tess.tessellate_path(
                            &ruffle_path_to_lyon_path(commands, true),
                            &FillOptions::even_odd(),
                            &mut buffers_builder,
                        ) {
                            // This may just be a degenerate path; skip it.
                            log::error!("Tessellation failure: {:?}", e);
                            continue;
                        }

                        let mut colors: Vec<[f32; 4]> = Vec::with_capacity(8);
                        let mut ratios: Vec<f32> = Vec::with_capacity(8);
                        for record in &gradient.records {
                            colors.push([
                                f32::from(record.color.r) / 255.0,
                                f32::from(record.color.g) / 255.0,
                                f32::from(record.color.b) / 255.0,
                                f32::from(record.color.a) / 255.0,
                            ]);
                            ratios.push(f32::from(record.ratio) / 255.0);
                        }

                        let gradient = Gradient {
                            gradient_type: GradientType::Focal,
                            ratios,
                            colors,
                            num_colors: gradient.records.len() as u32,
                            matrix: swf_to_gl_matrix(gradient.matrix.clone()),
                            repeat_mode: gradient.spread,
                            focal_point: *focal_point,
                        };

                        flush_draw(DrawType::Gradient(gradient), &mut mesh, &mut lyon_mesh);
                    }
                    FillStyle::Bitmap {
                        id,
                        matrix,
                        is_smoothed,
                        is_repeating,
                    } => {
                        flush_draw(DrawType::Color, &mut mesh, &mut lyon_mesh);

                        let mut buffers_builder = BuffersBuilder::new(
                            &mut lyon_mesh,
                            RuffleVertexCtor { color: 0xffff_ffff },
                        );

                        if let Err(e) = self.fill_tess.tessellate_path(
                            &ruffle_path_to_lyon_path(commands, true),
                            &FillOptions::even_odd(),
                            &mut buffers_builder,
                        ) {
                            // This may just be a degenerate path; skip it.
                            log::error!("Tessellation failure: {:?}", e);
                            continue;
                        }

                        let (bitmap_width, bitmap_height) =
                            (get_bitmap_dimensions)(*id).unwrap_or((1, 1));

                        let bitmap = Bitmap {
                            matrix: swf_bitmap_to_gl_matrix(
                                matrix.clone(),
                                bitmap_width,
                                bitmap_height,
                            ),
                            id: *id,
                            is_smoothed: *is_smoothed,
                            is_repeating: *is_repeating,
                        };

                        flush_draw(DrawType::Bitmap(bitmap), &mut mesh, &mut lyon_mesh);
                    }
                },
                DrawPath::Stroke {
                    style,
                    commands,
                    is_closed,
                } => {
                    let color = ((style.color.a as u32) << 24)
                        | ((style.color.b as u32) << 16)
                        | ((style.color.g as u32) << 8)
                        | (style.color.r as u32);

                    let mut buffers_builder =
                        BuffersBuilder::new(&mut lyon_mesh, RuffleVertexCtor { color });

                    // TODO(Herschel): 0 width indicates "hairline".
                    let width = if style.width.to_pixels() >= 1.0 {
                        style.width.to_pixels() as f32
                    } else {
                        1.0
                    };

                    let mut options = StrokeOptions::default()
                        .with_line_width(width)
                        .with_line_join(match style.join_style {
                            swf::LineJoinStyle::Round => tessellation::LineJoin::Round,
                            swf::LineJoinStyle::Bevel => tessellation::LineJoin::Bevel,
                            swf::LineJoinStyle::Miter(_) => tessellation::LineJoin::MiterClip,
                        })
                        .with_start_cap(match style.start_cap {
                            swf::LineCapStyle::None => tessellation::LineCap::Butt,
                            swf::LineCapStyle::Round => tessellation::LineCap::Round,
                            swf::LineCapStyle::Square => tessellation::LineCap::Square,
                        })
                        .with_end_cap(match style.end_cap {
                            swf::LineCapStyle::None => tessellation::LineCap::Butt,
                            swf::LineCapStyle::Round => tessellation::LineCap::Round,
                            swf::LineCapStyle::Square => tessellation::LineCap::Square,
                        });

                    if let swf::LineJoinStyle::Miter(limit) = style.join_style {
                        options = options.with_miter_limit(limit);
                    }

                    if let Err(e) = self.stroke_tess.tessellate_path(
                        &ruffle_path_to_lyon_path(commands, is_closed),
                        &options,
                        &mut buffers_builder,
                    ) {
                        // This may just be a degenerate path; skip it.
                        log::error!("Tessellation failure: {:?}", e);
                        continue;
                    }
                }
            }
        }

        flush_draw(DrawType::Color, &mut mesh, &mut lyon_mesh);

        mesh
    }
}

impl Default for ShapeTessellator {
    fn default() -> Self {
        Self::new()
    }
}

type Mesh = Vec<Draw>;

pub struct Draw {
    pub draw_type: DrawType,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

pub enum DrawType {
    Color,
    Gradient(Gradient),
    Bitmap(Bitmap),
}

#[derive(Clone, Debug)]
pub struct Gradient {
    pub matrix: [[f32; 3]; 3],
    pub gradient_type: GradientType,
    pub ratios: Vec<f32>,
    pub colors: Vec<[f32; 4]>,
    pub num_colors: u32,
    pub repeat_mode: GradientSpread,
    pub focal_point: f32,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: u32,
}

#[derive(Clone, Debug)]
pub struct Bitmap {
    pub matrix: [[f32; 3]; 3],
    pub id: swf::CharacterId,
    pub is_smoothed: bool,
    pub is_repeating: bool,
}

#[allow(clippy::many_single_char_names)]
fn swf_to_gl_matrix(m: swf::Matrix) -> [[f32; 3]; 3] {
    let tx = m.translate_x.get() as f32;
    let ty = m.translate_y.get() as f32;
    let det = m.scale_x * m.scale_y - m.rotate_skew_1 * m.rotate_skew_0;
    let mut a = m.scale_y / det;
    let mut b = -m.rotate_skew_1 / det;
    let mut c = -(tx * m.scale_y - m.rotate_skew_1 * ty) / det;
    let mut d = -m.rotate_skew_0 / det;
    let mut e = m.scale_x / det;
    let mut f = (tx * m.rotate_skew_0 - m.scale_x * ty) / det;

    a *= 20.0 / 32768.0;
    b *= 20.0 / 32768.0;
    d *= 20.0 / 32768.0;
    e *= 20.0 / 32768.0;

    c /= 32768.0;
    f /= 32768.0;
    c += 0.5;
    f += 0.5;
    [[a, d, 0.0], [b, e, 0.0], [c, f, 1.0]]
}

#[allow(clippy::many_single_char_names)]
fn swf_bitmap_to_gl_matrix(m: swf::Matrix, bitmap_width: u32, bitmap_height: u32) -> [[f32; 3]; 3] {
    let bitmap_width = bitmap_width as f32;
    let bitmap_height = bitmap_height as f32;

    let tx = m.translate_x.get() as f32;
    let ty = m.translate_y.get() as f32;
    let det = m.scale_x * m.scale_y - m.rotate_skew_1 * m.rotate_skew_0;
    let mut a = m.scale_y / det;
    let mut b = -m.rotate_skew_1 / det;
    let mut c = -(tx * m.scale_y - m.rotate_skew_1 * ty) / det;
    let mut d = -m.rotate_skew_0 / det;
    let mut e = m.scale_x / det;
    let mut f = (tx * m.rotate_skew_0 - m.scale_x * ty) / det;

    a *= 20.0 / bitmap_width;
    b *= 20.0 / bitmap_width;
    d *= 20.0 / bitmap_height;
    e *= 20.0 / bitmap_height;

    c /= bitmap_width;
    f /= bitmap_height;

    [[a, d, 0.0], [b, e, 0.0], [c, f, 1.0]]
}

fn ruffle_path_to_lyon_path(commands: Vec<DrawCommand>, is_closed: bool) -> Path {
    fn point(x: Twips, y: Twips) -> lyon::math::Point {
        lyon::math::Point::new(x.to_pixels() as f32, y.to_pixels() as f32)
    }

    let mut builder = Path::builder();
    for cmd in commands {
        match cmd {
            DrawCommand::MoveTo { x, y } => {
                builder.move_to(point(x, y));
            }
            DrawCommand::LineTo { x, y } => {
                builder.line_to(point(x, y));
            }
            DrawCommand::CurveTo { x1, y1, x2, y2 } => {
                builder.quadratic_bezier_to(point(x1, y1), point(x2, y2));
            }
        }
    }

    if is_closed {
        builder.close();
    }

    builder.build()
}

struct RuffleVertexCtor {
    color: u32,
}

impl FillVertexConstructor<Vertex> for RuffleVertexCtor {
    fn new_vertex(&mut self, position: lyon::math::Point, _: FillAttributes) -> Vertex {
        Vertex {
            position: [position.x, position.y],
            color: self.color,
        }
    }
}

impl StrokeVertexConstructor<Vertex> for RuffleVertexCtor {
    fn new_vertex(&mut self, position: lyon::math::Point, _: StrokeAttributes) -> Vertex {
        Vertex {
            position: [position.x, position.y],
            color: self.color,
        }
    }
}

pub use swf::GradientSpread;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum GradientType {
    Linear,
    Radial,
    Focal,
}
