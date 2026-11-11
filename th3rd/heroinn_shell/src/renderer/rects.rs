use std::collections::HashMap;
use std::mem;

use crossfont::Metrics;

use heroinn_shell_terminal::grid::Dimensions;
use heroinn_shell_terminal::index::{Column, Point};
use heroinn_shell_terminal::term::cell::Flags;
use heroinn_shell_terminal::term::color::Rgb;

use crate::display::content::RenderableCell;
use crate::display::SizeInfo;
use crate::gl;
use crate::gl::types::*;
use crate::renderer::shader::{ShaderError, ShaderProgram, ShaderVersion};
use crate::renderer::{self, cstr};

#[derive(Debug, Copy, Clone)]
pub struct RenderRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: Rgb,
    pub alpha: f32,
    pub kind: RectKind,
}

impl RenderRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32, color: Rgb, alpha: f32) -> Self {
        RenderRect { kind: RectKind::Normal, x, y, width, height, color, alpha }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RenderLine {
    pub start: Point<usize>,
    pub end: Point<usize>,
    pub color: Rgb,
}

// NOTE: These flags must be in sync with their usage in the rect.*.glsl shaders.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RectKind {
    Normal = 0,
    Undercurl = 1,
    DottedUnderline = 2,
    DashedUnderline = 3,
    NumKinds = 4,
}

impl RenderLine {
    pub fn rects(&self, flag: Flags, metrics: &Metrics, size: &SizeInfo) -> Vec<RenderRect> {
        let mut rects = Vec::new();

        let mut start = self.start;
        while start.line < self.end.line {
            let end = Point::new(start.line, size.last_column());
            Self::push_rects(&mut rects, metrics, size, flag, start, end, self.color);
            start = Point::new(start.line + 1, Column(0));
        }
        Self::push_rects(&mut rects, metrics, size, flag, start, self.end, self.color);

        rects
    }

    /// Push all rects required to draw the cell's line.
    fn push_rects(
        rects: &mut Vec<RenderRect>,
        metrics: &Metrics,
        size: &SizeInfo,
        flag: Flags,
        start: Point<usize>,
        end: Point<usize>,
        color: Rgb,
    ) {
        let (position, thickness, ty) = match flag {
            Flags::DOUBLE_UNDERLINE => {
                // Position underlines so each one has 50% of descent available.
                let top_pos = 0.25 * metrics.descent;
                let bottom_pos = 0.75 * metrics.descent;

                rects.push(Self::create_rect(
                    size,
                    metrics.descent,
                    start,
                    end,
                    top_pos,
                    metrics.underline_thickness,
                    color,
                ));

                (bottom_pos, metrics.underline_thickness, RectKind::Normal)
            },
            // Make undercurl occupy the entire descent area.
            Flags::UNDERCURL => (metrics.descent, metrics.descent.abs(), RectKind::Undercurl),
            Flags::UNDERLINE => {
                (metrics.underline_position, metrics.underline_thickness, RectKind::Normal)
            },
            // Make dotted occupy the entire descent area.
            Flags::DOTTED_UNDERLINE => {
                (metrics.descent, metrics.descent.abs(), RectKind::DottedUnderline)
            },
            Flags::DASHED_UNDERLINE => {
                (metrics.underline_position, metrics.underline_thickness, RectKind::DashedUnderline)
            },
            Flags::STRIKEOUT => {
                (metrics.strikeout_position, metrics.strikeout_thickness, RectKind::Normal)
            },
            _ => unimplemented!("Invalid flag for cell line drawing specified"),
        };

        let mut rect =
            Self::create_rect(size, metrics.descent, start, end, position, thickness, color);
        rect.kind = ty;
        rects.push(rect);
    }

    /// Create a line's rect at a position relative to the baseline.
    fn create_rect(
        size: &SizeInfo,
        descent: f32,
        start: Point<usize>,
        end: Point<usize>,
        position: f32,
        mut thickness: f32,
        color: Rgb,
    ) -> RenderRect {
        let start_x = start.column.0 as f32 * size.cell_width();
        let end_x = (end.column.0 + 1) as f32 * size.cell_width();
        let width = end_x - start_x;

        // Make sure lines are always visible.
        thickness = thickness.max(1.);

        let line_bottom = (start.line as f32 + 1.) * size.cell_height();
        let baseline = line_bottom + descent;

        let mut y = (baseline - position - thickness / 2.).round();
        let max_y = line_bottom - thickness;
        if y > max_y {
            y = max_y;
        }

        RenderRect::new(
            start_x + size.padding_x(),
            y + size.padding_y(),
            width,
            thickness,
            color,
            1.,
        )
    }
}

/// Lines for underline and strikeout.
#[derive(Default)]
pub struct RenderLines {
    inner: HashMap<Flags, Vec<RenderLine>>,
}

impl RenderLines {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn rects(&self, metrics: &Metrics, size: &SizeInfo) -> Vec<RenderRect> {
        self.inner
            .iter()
            .flat_map(|(flag, lines)| {
                lines.iter().flat_map(move |line| line.rects(*flag, metrics, size))
            })
            .collect()
    }

    /// Update the stored lines with the next cell info.
    #[inline]
    pub fn update(&mut self, cell: &RenderableCell) {
        self.update_flag(cell, Flags::UNDERLINE);
        self.update_flag(cell, Flags::DOUBLE_UNDERLINE);
        self.update_flag(cell, Flags::STRIKEOUT);
        self.update_flag(cell, Flags::UNDERCURL);
        self.update_flag(cell, Flags::DOTTED_UNDERLINE);
        self.update_flag(cell, Flags::DASHED_UNDERLINE);
    }

    /// Update the lines for a specific flag.
    fn update_flag(&mut self, cell: &RenderableCell, flag: Flags) {
        if !cell.flags.contains(flag) {
            return;
        }

        // The underline color escape does not apply to strikeout.
        let color = if flag.contains(Flags::STRIKEOUT) { cell.fg } else { cell.underline };

        // Include wide char spacer if the current cell is a wide char.
        let mut end = cell.point;
        if cell.flags.contains(Flags::WIDE_CHAR) {
            end.column += 1;
        }

        // Check if there's an active line.
        if let Some(line) = self.inner.get_mut(&flag).and_then(|lines| lines.last_mut()) {
            if color == line.color
                && cell.point.column == line.end.column + 1
                && cell.point.line == line.end.line
            {
                // Update the length of the line.
                line.end = end;
                return;
            }
        }

        // Start new line if there currently is none.
        let line = RenderLine { start: cell.point, end, color };
        match self.inner.get_mut(&flag) {
            Some(lines) => lines.push(line),
            None => {
                self.inner.insert(flag, vec![line]);
            },
        }
    }
}

/// Shader sources for rect rendering program.
static RECT_SHADER_F: &str = include_str!("../../res/rect.f.glsl");
static RECT_SHADER_V: &str = include_str!("../../res/rect.v.glsl");

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Vertex {
    // Normalized screen coordinates.
    x: f32,
    y: f32,

    // Color.
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[derive(Debug)]
pub struct RectRenderer {
    // GL buffer objects.
    vao: GLuint,
    vbo: GLuint,

    program: RectShaderProgram,

    vertices: [Vec<Vertex>; 4],
}

impl RectRenderer {
    pub fn new(shader_version: ShaderVersion) -> Result<Self, renderer::Error> {
        let mut vao: GLuint = 0;
        let mut vbo: GLuint = 0;
        let program = RectShaderProgram::new(shader_version)?;

        unsafe {
            // Allocate buffers.
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);

            gl::BindVertexArray(vao);

            // VBO binding is not part of VAO itself, but VBO binding is stored in attributes.
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

            let mut attribute_offset = 0;

            // Position.
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                attribute_offset as *const _,
            );
            gl::EnableVertexAttribArray(0);
            attribute_offset += mem::size_of::<f32>() * 2;

            // Color.
            gl::VertexAttribPointer(
                1,
                4,
                gl::UNSIGNED_BYTE,
                gl::TRUE,
                mem::size_of::<Vertex>() as i32,
                attribute_offset as *const _,
            );
            gl::EnableVertexAttribArray(1);

            // Reset buffer bindings.
            gl::BindVertexArray(0);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }

        Ok(Self { vao, vbo, program, vertices: Default::default() })
    }

    pub fn draw(&mut self, size_info: &SizeInfo, metrics: &Metrics, rects: Vec<RenderRect>) {
        unsafe {
            // Bind VAO to enable vertex attribute slots.
            gl::BindVertexArray(self.vao);

            // Bind VBO only once for buffer data upload only.
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);

            gl::UseProgram(self.program.id());
            self.program.update_uniforms(size_info, metrics);
        }

        let half_width = size_info.width() / 2.;
        let half_height = size_info.height() / 2.;

        // Build rect vertices vector.
        self.vertices.iter_mut().for_each(|vertices| vertices.clear());
        for rect in &rects {
            Self::add_rect(&mut self.vertices[rect.kind as usize], half_width, half_height, rect);
        }

        unsafe {
            // We iterate in reverse order to draw plain rects at the end, since we want visual
            // bell or damage rects be above the lines.
            for rect_kind in (RectKind::Normal as u8..RectKind::NumKinds as u8).rev() {
                let vertices = &mut self.vertices[rect_kind as usize];
                if vertices.is_empty() {
                    continue;
                }

                self.program.set_rect_kind(rect_kind);

                // Upload accumulated undercurl vertices.
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (vertices.len() * mem::size_of::<Vertex>()) as isize,
                    vertices.as_ptr() as *const _,
                    gl::STREAM_DRAW,
                );

                // Draw all vertices as list of triangles.
                gl::DrawArrays(gl::TRIANGLES, 0, vertices.len() as i32);
            }

            // Disable program.
            gl::UseProgram(0);

            // Reset buffer bindings to nothing.
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }
    }

    fn add_rect(vertices: &mut Vec<Vertex>, half_width: f32, half_height: f32, rect: &RenderRect) {
        // Calculate rectangle vertices positions in normalized device coordinates.
        // NDC range from -1 to +1, with Y pointing up.
        let x = rect.x / half_width - 1.0;
        let y = -rect.y / half_height + 1.0;
        let width = rect.width / half_width;
        let height = rect.height / half_height;
        let Rgb { r, g, b } = rect.color;
        let a = (rect.alpha * 255.) as u8;

        // Make quad vertices.
        let quad = [
            Vertex { x, y, r, g, b, a },
            Vertex { x, y: y - height, r, g, b, a },
            Vertex { x: x + width, y, r, g, b, a },
            Vertex { x: x + width, y: y - height, r, g, b, a },
        ];

        // Append the vertices to form two triangles.
        vertices.push(quad[0]);
        vertices.push(quad[1]);
        vertices.push(quad[2]);
        vertices.push(quad[2]);
        vertices.push(quad[3]);
        vertices.push(quad[1]);
    }
}

impl Drop for RectRenderer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}

/// Rectangle drawing program.
#[derive(Debug)]
pub struct RectShaderProgram {
    /// Shader program.
    program: ShaderProgram,

    /// Kind of rect we're drawing.
    u_rect_kind: GLint,

    /// Cell width.
    u_cell_width: GLint,

    /// Cell height.
    u_cell_height: GLint,

    /// Terminal padding.
    u_padding_x: GLint,

    /// A padding from the bottom of the screen to viewport.
    u_padding_y: GLint,

    /// Underline position.
    u_underline_position: GLint,

    /// Underline thickness.
    u_underline_thickness: GLint,

    /// Undercurl position.
    u_undercurl_position: GLint,
}

impl RectShaderProgram {
    pub fn new(shader_version: ShaderVersion) -> Result<Self, ShaderError> {
        let program = ShaderProgram::new(shader_version, RECT_SHADER_V, RECT_SHADER_F)?;

        Ok(Self {
            u_rect_kind: program.get_uniform_location(cstr!("rectKind"))?,
            u_cell_width: program.get_uniform_location(cstr!("cellWidth"))?,
            u_cell_height: program.get_uniform_location(cstr!("cellHeight"))?,
            u_padding_x: program.get_uniform_location(cstr!("paddingX"))?,
            u_padding_y: program.get_uniform_location(cstr!("paddingY"))?,
            u_underline_position: program.get_uniform_location(cstr!("underlinePosition"))?,
            u_underline_thickness: program.get_uniform_location(cstr!("underlineThickness"))?,
            u_undercurl_position: program.get_uniform_location(cstr!("undercurlPosition"))?,
            program,
        })
    }

    fn id(&self) -> GLuint {
        self.program.id()
    }

    fn set_rect_kind(&self, ty: u8) {
        unsafe {
            gl::Uniform1i(self.u_rect_kind, ty as i32);
        }
    }

    pub fn update_uniforms(&self, size_info: &SizeInfo, metrics: &Metrics) {
        let position = (0.5 * metrics.descent).abs();
        let underline_position = metrics.descent.abs() - metrics.underline_position.abs();

        let viewport_height = size_info.height() - size_info.padding_y();
        let padding_y = viewport_height
            - (viewport_height / size_info.cell_height()).floor() * size_info.cell_height();

        unsafe {
            gl::Uniform1f(self.u_cell_width, size_info.cell_width());
            gl::Uniform1f(self.u_cell_height, size_info.cell_height());
            gl::Uniform1f(self.u_padding_y, padding_y);
            gl::Uniform1f(self.u_padding_x, size_info.padding_x());
            gl::Uniform1f(self.u_underline_position, underline_position);
            gl::Uniform1f(self.u_underline_thickness, metrics.underline_thickness);
            gl::Uniform1f(self.u_undercurl_position, position);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heroinn_shell_terminal::index::Column;

    fn metrics() -> Metrics {
        Metrics {
            average_advance: 10.0,
            line_height: 20.0,
            descent: 4.0,
            underline_position: 2.0,
            underline_thickness: 0.5,
            strikeout_position: 8.0,
            strikeout_thickness: 1.5,
        }
    }

    fn size() -> SizeInfo {
        SizeInfo::new(100.0, 100.0, 10.0, 20.0, 5.0, 5.0, false)
    }

    fn color() -> Rgb {
        Rgb { r: 12, g: 34, b: 56 }
    }

    fn underline_color() -> Rgb {
        Rgb { r: 200, g: 100, b: 50 }
    }

    fn cell(line: usize, column: usize, flags: Flags) -> RenderableCell {
        RenderableCell {
            character: 'x',
            point: Point::new(line, Column(column)),
            fg: color(),
            bg: Rgb { r: 0, g: 0, b: 0 },
            bg_alpha: 1.0,
            underline: underline_color(),
            flags,
            extra: None,
        }
    }

    #[test]
    fn render_rect_new_defaults_to_a_normal_rect() {
        let rect = RenderRect::new(1.0, 2.0, 3.0, 4.0, color(), 0.5);
        assert_eq!(rect.x, 1.0);
        assert_eq!(rect.y, 2.0);
        assert_eq!(rect.width, 3.0);
        assert_eq!(rect.height, 4.0);
        assert_eq!(rect.color, color());
        assert_eq!(rect.alpha, 0.5);
        assert_eq!(rect.kind, RectKind::Normal);
    }

    #[test]
    fn render_line_rects_wrap_at_last_column_and_clamp_thickness() {
        let line = RenderLine {
            start: Point::new(0, Column(1)),
            end: Point::new(2, Column(3)),
            color: color(),
        };
        let rects = line.rects(Flags::UNDERLINE, &metrics(), &size());
        assert_eq!(rects.len(), 3);

        assert_eq!(rects[0].x, 15.0);
        assert_eq!(rects[0].y, 24.0);
        assert_eq!(rects[0].width, 80.0);
        assert_eq!(rects[0].height, 1.0);
        assert_eq!(rects[0].kind, RectKind::Normal);

        assert_eq!(rects[1].x, 5.0);
        assert_eq!(rects[1].y, 44.0);
        assert_eq!(rects[1].width, 90.0);

        assert_eq!(rects[2].x, 5.0);
        assert_eq!(rects[2].y, 64.0);
        assert_eq!(rects[2].width, 40.0);
        assert_eq!(rects[2].color, color());
        assert_eq!(rects[2].alpha, 1.0);
    }

    #[test]
    fn every_supported_line_flag_maps_to_the_expected_rect_kind() {
        let line = RenderLine {
            start: Point::new(0, Column(0)),
            end: Point::new(0, Column(2)),
            color: color(),
        };

        let double = line.rects(Flags::DOUBLE_UNDERLINE, &metrics(), &size());
        assert_eq!(double.len(), 2);
        assert!(double.iter().all(|rect| rect.kind == RectKind::Normal));

        let cases = [
            (Flags::UNDERLINE, RectKind::Normal, 1),
            (Flags::UNDERCURL, RectKind::Undercurl, 1),
            (Flags::DOTTED_UNDERLINE, RectKind::DottedUnderline, 1),
            (Flags::DASHED_UNDERLINE, RectKind::DashedUnderline, 1),
            (Flags::STRIKEOUT, RectKind::Normal, 1),
        ];
        for (flag, expected_kind, expected_len) in cases {
            let rects = line.rects(flag, &metrics(), &size());
            assert_eq!(rects.len(), expected_len);
            assert_eq!(rects[0].kind, expected_kind);
        }
    }

    #[test]
    fn create_rect_enforces_minimum_thickness_and_accounts_for_padding() {
        let size = size();
        let rect = RenderLine::create_rect(
            &size,
            4.0,
            Point::new(0, Column(2)),
            Point::new(0, Column(3)),
            2.0,
            0.0,
            color(),
        );
        assert_eq!(rect.x, 25.0);
        assert_eq!(rect.width, 20.0);
        assert_eq!(rect.height, 1.0);
        assert_eq!(rect.y, 24.0);
    }

    #[test]
    fn render_lines_merge_adjacent_cells_and_split_on_gaps_or_color_changes() {
        let mut lines = RenderLines::new();
        lines.update(&cell(1, 2, Flags::UNDERLINE));
        lines.update(&cell(1, 3, Flags::UNDERLINE));
        lines.update(&cell(1, 4, Flags::UNDERLINE));

        let stored = lines.inner.get(&Flags::UNDERLINE).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].start, Point::new(1, Column(2)));
        assert_eq!(stored[0].end, Point::new(1, Column(4)));
        assert_eq!(stored[0].color, underline_color());

        lines.update(&cell(1, 6, Flags::UNDERLINE));
        assert_eq!(lines.inner.get(&Flags::UNDERLINE).unwrap().len(), 2);

        let mut other_color = cell(1, 7, Flags::UNDERLINE);
        other_color.underline = Rgb { r: 1, g: 2, b: 3 };
        lines.update(&other_color);
        let stored = lines.inner.get(&Flags::UNDERLINE).unwrap();
        assert_eq!(stored.len(), 3);
        assert_eq!(stored[2].color, Rgb { r: 1, g: 2, b: 3 });
    }

    #[test]
    fn render_lines_handle_wide_chars_and_strikeout_color_semantics() {
        let mut lines = RenderLines::new();
        lines.update(&cell(0, 3, Flags::UNDERLINE | Flags::UNDERCURL | Flags::WIDE_CHAR));

        let underline = lines.inner.get(&Flags::UNDERLINE).unwrap();
        assert_eq!(underline[0].end, Point::new(0, Column(4)));
        let undercurl = lines.inner.get(&Flags::UNDERCURL).unwrap();
        assert_eq!(undercurl[0].end, Point::new(0, Column(4)));

        lines.update(&cell(2, 1, Flags::STRIKEOUT));
        let strikeout = lines.inner.get(&Flags::STRIKEOUT).unwrap();
        assert_eq!(strikeout[0].color, color());

        let rects = lines.rects(&metrics(), &size());
        assert!(!rects.is_empty());
        assert!(rects.iter().any(|rect| rect.kind == RectKind::Undercurl));
    }

    #[test]
    fn render_lines_ignore_cells_without_line_flags() {
        let mut lines = RenderLines::new();
        lines.update(&cell(0, 0, Flags::BOLD));
        assert!(lines.inner.is_empty());
        assert!(lines.rects(&metrics(), &size()).is_empty());
    }

    #[test]
    fn add_rect_emits_two_triangles_in_normalized_device_coordinates() {
        let rect = RenderRect {
            x: 0.0,
            y: 0.0,
            width: 20.0,
            height: 10.0,
            color: Rgb { r: 255, g: 128, b: 0 },
            alpha: 0.5,
            kind: RectKind::Normal,
        };
        let mut vertices = Vec::new();
        RectRenderer::add_rect(&mut vertices, 100.0, 50.0, &rect);
        assert_eq!(vertices.len(), 6);

        assert_eq!(vertices[0].x, -1.0);
        assert_eq!(vertices[0].y, 1.0);
        assert_eq!(vertices[1].x, -1.0);
        assert_eq!(vertices[1].y, 0.8);
        assert_eq!(vertices[2].x, -0.8);
        assert_eq!(vertices[2].y, 1.0);
        assert_eq!(vertices[3].x, -0.8);
        assert_eq!(vertices[3].y, 1.0);
        assert_eq!(vertices[4].x, vertices[3].x);
        assert_eq!(vertices[4].y, 0.8);
        assert_eq!(vertices[5].x, vertices[1].x);
        assert_eq!(vertices[5].y, 0.8);

        assert_eq!(vertices[0].r, 255);
        assert_eq!(vertices[0].g, 128);
        assert_eq!(vertices[0].b, 0);
        assert_eq!(vertices[0].a, 127);
    }
}
