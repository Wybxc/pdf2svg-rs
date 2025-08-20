use std::{borrow::Cow, cell::RefCell, rc::Rc};

use mupdf::*;
use quick_xml::{
    Writer,
    escape::escape,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

pub fn render(page: &Page, writer: Rc<RefCell<Writer<Vec<u8>>>>) {
    let renderer = TextRender::new(writer);
    let device = Device::from_native(renderer).unwrap();
    page.run(&device, &Matrix::IDENTITY).unwrap();
}

struct TextRender {
    writer: Rc<RefCell<Writer<Vec<u8>>>>,
}

impl TextRender {
    pub fn new(writer: Rc<RefCell<Writer<Vec<u8>>>>) -> Self {
        TextRender { writer }
    }
}

impl NativeDevice for TextRender {
    fn fill_text(
        &mut self,
        text: &Text,
        cmt: Matrix,
        _color_space: &Colorspace,
        _color: &[f32],
        _alpha: f32,
        _cp: ColorParams,
    ) {
        render_text(text, cmt, &mut self.writer.borrow_mut());
    }

    fn stroke_text(
        &mut self,
        text: &Text,
        _stroke_state: &StrokeState,
        cmt: Matrix,
        _color_space: &Colorspace,
        _color: &[f32],
        _alpha: f32,
        _cp: ColorParams,
    ) {
        render_text(text, cmt, &mut self.writer.borrow_mut());
    }

    fn clip_text(&mut self, text: &Text, cmt: Matrix, _scissor: Rect) {
        render_text(text, cmt, &mut self.writer.borrow_mut());
    }

    fn clip_stroke_text(
        &mut self,
        text: &Text,
        _stroke_state: &StrokeState,
        cmt: Matrix,
        _scissor: Rect,
    ) {
        render_text(text, cmt, &mut self.writer.borrow_mut());
    }
}

fn render_text(text: &Text, cmt: Matrix, output: &mut Writer<Vec<u8>>) {
    for span in text.spans() {
        let trm = span.trm();
        let fontsize = trm.expansion();

        let Matrix { a, b, c, d, .. } = trm;
        let inv = Matrix::new(
            d / fontsize,
            -b / fontsize,
            -c / fontsize,
            -a / fontsize,
            0.,
            0.,
        );
        let transform = concat_matrix(&inv, &cmt);

        let font = span.font();
        let mut fontfamily = font.name();
        if let Some(prefix) = fontfamily.find('+') {
            fontfamily = &fontfamily[prefix + 1..];
        }
        if let Some(suffix) = fontfamily.rfind('-') {
            fontfamily = &fontfamily[..suffix];
        }

        let mut elem = BytesStart::new("text");
        elem.push_attribute(("xml:space", "preserve"));
        elem.push_attribute(("transform", Cow::Owned(format_matrix(&transform))));
        elem.push_attribute(("font-family", fontfamily));
        elem.push_attribute(("font-size", Cow::Owned(format!("{}pt", fontsize))));
        if matches!(span.wmode(), WriteMode::Vertical) {
            elem.push_attribute(("writing-mode", "tb"));
        }
        elem.push_attribute(("opacity", "0"));

        output.write_event(Event::Start(elem)).unwrap();

        struct GlyphMetrics {
            width: f32,
            primary: f32,
            followings: usize,
        }
        struct Line<'a> {
            elem: BytesStart<'a>,
            glyphs: Vec<GlyphMetrics>,
            secondary: f32,
            text: String,
        }
        let mut line: Option<Line> = None;
        for item in span
            .items()
            .filter(|c| c.ucs() >= 0 && char::from_u32(c.ucs() as u32).is_some())
        {
            let (x, y) = transform_point(&inv, (item.x(), item.y()));
            let (primary, secondary) = match span.wmode() {
                WriteMode::Horizontal => (x, y),
                WriteMode::Vertical => (y, x),
            };
            if line.as_ref().is_none_or(|s| s.secondary != secondary) {
                if let Some(line) = line.take() {
                    line.emit(span.wmode(), output);
                }

                let mut elem = BytesStart::new("tspan");
                elem.push_attribute((
                    match span.wmode() {
                        WriteMode::Horizontal => "y",
                        WriteMode::Vertical => "x",
                    },
                    Cow::Owned(format!("{secondary}")),
                ));
                line = Some(Line {
                    elem,
                    glyphs: Vec::new(),
                    secondary,
                    text: String::new(),
                });
            }
            let line = line.as_mut().unwrap();
            if item.gid() >= 0 {
                let width = font
                    .advance_glyph_with_wmode(item.gid(), span.wmode() as u32 != 0)
                    .unwrap_or(0.3);
                line.glyphs.push(GlyphMetrics {
                    width,
                    primary,
                    followings: 0,
                });
            } else if let Some(s) = line.glyphs.last_mut() {
                s.followings += 0;
            } else {
                line.glyphs.push(GlyphMetrics {
                    width: 0.,
                    primary,
                    followings: 0,
                });
            }
            line.text.push(char::from_u32(item.ucs() as u32).unwrap());
        }
        impl Line<'_> {
            fn emit(self, wmode: WriteMode, output: &mut Writer<Vec<u8>>) {
                let mut offsets = vec![];
                for glyph in self.glyphs {
                    offsets.push(glyph.primary);
                    let adv = glyph.width / ((glyph.followings + 1) as f32);
                    for i in 0..glyph.followings {
                        offsets.push(glyph.primary + adv * ((i + 1) as f32));
                    }
                }

                let mut elem = self.elem;
                elem.push_attribute((
                    match wmode {
                        WriteMode::Horizontal => "x",
                        WriteMode::Vertical => "y",
                    },
                    Cow::Owned(
                        offsets
                            .iter()
                            .map(f32::to_string)
                            .collect::<Vec<_>>()
                            .join(" "),
                    ),
                ));

                output.write_event(Event::Start(elem)).unwrap();
                output
                    .write_event(Event::Text(BytesText::from_escaped(escape(self.text))))
                    .unwrap();
                output
                    .write_event(Event::End(BytesEnd::new("tspan")))
                    .unwrap();
            }
        }
        if let Some(line) = line.take() {
            line.emit(span.wmode(), output);
        }

        output
            .write_event(Event::End(BytesEnd::new("text")))
            .unwrap();
    }
}

fn format_matrix(m: &Matrix) -> String {
    format!("matrix({},{},{},{},{},{})", m.a, m.b, m.c, m.d, m.e, m.f)
}

fn concat_matrix(m1: &Matrix, m2: &Matrix) -> Matrix {
    Matrix::new(
        m1.a * m2.a + m1.b * m2.c,
        m1.a * m2.b + m1.b * m2.d,
        m1.c * m2.a + m1.d * m2.c,
        m1.c * m2.b + m1.d * m2.d,
        m1.e * m2.a + m1.f * m2.c + m2.e,
        m1.e * m2.b + m1.f * m2.d + m2.f,
    )
}

fn transform_point(m: &Matrix, (x, y): (f32, f32)) -> (f32, f32) {
    let mx = m.a * x + m.c * y + m.e;
    let my = m.b * x + m.d * y + m.f;
    (mx, my)
}

/*static void
svg_dev_text_span(fz_context *ctx, svg_device *sdev, fz_matrix ctm, const fz_text_span *span)
{
    fz_buffer *out = sdev->out;
    char font_family[100];
    int is_bold, is_italic;
    fz_matrix tm, inv_tm, final_tm;
    fz_point p;
    float font_size;
    fz_text_item *it;
    int start, end, i;
    float cluster_advance = 0;

    if (span->len == 0)
    {
        fz_append_printf(ctx, out, "/>\n");
        return;
    }

    tm = span->trm;
    font_size = fz_matrix_expansion(tm);
    final_tm.a = tm.a / font_size;
    final_tm.b = tm.b / font_size;
    final_tm.c = -tm.c / font_size;
    final_tm.d = -tm.d / font_size;
    final_tm.e = 0;
    final_tm.f = 0;
    inv_tm = fz_invert_matrix(final_tm);
    final_tm = fz_concat(final_tm, ctm);

    tm.e = span->items[0].x;
    tm.f = span->items[0].y;

    svg_font_family(ctx, font_family, sizeof font_family, fz_font_name(ctx, span->font));
    is_bold = fz_font_is_bold(ctx, span->font);
    is_italic = fz_font_is_italic(ctx, span->font);

    fz_append_printf(ctx, out, " xml:space=\"preserve\"");
    fz_append_printf(ctx, out, " transform=\"matrix(%M)\"", &final_tm);
    fz_append_printf(ctx, out, " font-size=\"%g\"", font_size);
    fz_append_printf(ctx, out, " font-family=\"%s\"", font_family);
    if (is_bold) fz_append_printf(ctx, out, " font-weight=\"bold\"");
    if (is_italic) fz_append_printf(ctx, out, " font-style=\"italic\"");
    if (span->wmode != 0) fz_append_printf(ctx, out, " writing-mode=\"tb\"");

    fz_append_byte(ctx, out, '>');

    start = find_first_char(ctx, span, 0);
    while (start < span->len)
    {
        end = find_next_line_break(ctx, span, inv_tm, start);

        p.x = span->items[start].x;
        p.y = span->items[start].y;
        p = fz_transform_point(p, inv_tm);
        if (span->items[start].gid >= 0)
            cluster_advance = svg_cluster_advance(ctx, span, start, end);
        if (span->wmode == 0)
            fz_append_printf(ctx, out, "<tspan y=\"%g\" x=\"%g", p.y, p.x);
		else
			fz_append_printf(ctx, out, "<tspan x=\"%g\" y=\"%g", p.x, p.y);
        for (i = start + 1; i < end; ++i)
        {
            it = &span->items[i];
            if (it->gid >= 0)
                cluster_advance = svg_cluster_advance(ctx, span, i, end);
            if (it->ucs >= 0)
            {
                if (it->gid >= 0)
                {
                    p.x = it->x;
                    p.y = it->y;
                    p = fz_transform_point(p, inv_tm);
                }
                else
                {
                    /* we have no glyph (such as in a ligature) -- advance a bit */
                    if (span->wmode == 0)
                        p.x += font_size * cluster_advance;
                    else
                        p.y += font_size * cluster_advance;
                }
                fz_append_printf(ctx, out, " %g", span->wmode == 0 ? p.x : p.y);
            }
        }
        fz_append_printf(ctx, out, "\">");
		for (i = start; i < end; ++i)
		{
			it = &span->items[i];
			if (it->ucs >= 0)
			{
				int c = it->ucs;
				if (c >= 32 && c <= 127 && c != '<' && c != '&' && c != '>')
					fz_append_byte(ctx, out, c);
				else
					fz_append_printf(ctx, out, "&#x%04x;", c);
			}
		}
		fz_append_printf(ctx, out, "</tspan>");

		start = find_first_char(ctx, span, end);
	}

	fz_append_printf(ctx, out, "</text>\n");
} */
