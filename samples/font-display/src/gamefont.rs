use std::io::Read;

use rusttype::{Font, gpu_cache::Cache};
use dbsdk_rs::{vdp::{Texture, TextureFormat, Color32, Rectangle, PackedVertex, self}, db::log, math::{Vector4, Vector2, Matrix4x4}, field_offset::offset_of};

pub struct GameFont<'a> {
    font: Font<'a>,
    font_cache: Cache<'a>,
    font_texture: Texture,
}

impl<'a> GameFont<'a> {
    pub fn new<R>(reader: &mut R, atlas_size: u32) -> GameFont<'a> where R : Read {
        let mut font_buf: Vec<u8> = Vec::new();
        reader.read_to_end(&mut font_buf).expect("Failed reading font file");

        let font = Font::try_from_vec(font_buf).expect("Failed to load font");
        log("Font loaded");

        let cache = Cache::builder()
            .dimensions(atlas_size, atlas_size)
            .pad_glyphs(true)
            .build();

        let cache_texture = Texture::new(atlas_size as i32, atlas_size as i32, false, TextureFormat::RGBA8888)
            .expect("Failed allocating font texture");

        return GameFont { font: font, font_cache: cache, font_texture: cache_texture };
    }

    pub fn layout_text(
        &mut self,
        scale: rusttype::Scale,
        text: &str,
    ) -> Vec<rusttype::PositionedGlyph<'a>> {
        let mut result = Vec::new();
        let v_metrics = self.font.v_metrics(scale);
        let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
        let mut caret = rusttype::point(0.0, v_metrics.ascent);
        let mut last_glyph_id = None;
        for c in text.chars() {
            if c.is_control() {
                match c {
                    '\r' => {
                        caret = rusttype::point(0.0, caret.y + advance_height);
                    }
                    '\n' => {}
                    _ => {}
                }
                continue;
            }
            let base_glyph = self.font.glyph(c);
            if let Some(id) = last_glyph_id.take() {
                caret.x += self.font.pair_kerning(scale, id, base_glyph.id());
            }
            last_glyph_id = Some(base_glyph.id());
            let glyph = base_glyph.scaled(scale).positioned(caret);
            caret.x += glyph.unpositioned().h_metrics().advance_width;
            result.push(glyph);
        }
        result
    }

    pub fn draw_text(&mut self, x: i32, y: i32, size: i32, text: &str) {
        // iterate Unicode codepoints in string
        let glyphs = self.layout_text(rusttype::Scale::uniform(size as f32), text);
        // queue glyphs in cache
        for glyph in &glyphs {
            self.font_cache.queue_glyph(0, glyph.clone());
        }
        let tex = &self.font_texture;
        self.font_cache.cache_queued(|rect, data| {
            // data is single channel u8, so we have to transform into destination RGBA8888 format
            let mut pixdata: Vec<Color32> = vec![Color32::new(0, 0, 0, 0);data.len()];
            for i in 0..data.len() {
                pixdata[i] = Color32::new(255, 255, 255, data[i]);
            }
            let rx = rect.min.x as i32;
            let ry = rect.min.y as i32;
            let rw = (rect.max.x - rect.min.x) as i32;
            let rh = (rect.max.y - rect.min.y) as i32;
            tex.set_texture_data_region(0, Some(Rectangle::new(rx, ry, rw, rh)), pixdata.as_slice());
        }).expect("Failed updating font texture");

        let mut vertices: Vec<PackedVertex> = glyphs
            .iter()
            .filter_map(|g| self.font_cache.rect_for(0, g).ok().flatten())
            .flat_map(|(uvrect, screenrect)| {
                let minx = x + screenrect.min.x;
                let miny = y + screenrect.min.y;
                let maxx = x + screenrect.max.x;
                let maxy = y + screenrect.max.y;
                vec![
                    PackedVertex::new(Vector4::new(minx as f32, maxy as f32, 0.0, 1.0), Vector2::new(uvrect.min.x, uvrect.max.y), Color32::new(255, 255, 255, 255), Color32::new(0, 0, 0, 0)),
                    PackedVertex::new(Vector4::new(minx as f32, miny as f32, 0.0, 1.0), Vector2::new(uvrect.min.x, uvrect.min.y), Color32::new(255, 255, 255, 255), Color32::new(0, 0, 0, 0)),
                    PackedVertex::new(Vector4::new(maxx as f32, miny as f32, 0.0, 1.0), Vector2::new(uvrect.max.x, uvrect.min.y), Color32::new(255, 255, 255, 255), Color32::new(0, 0, 0, 0)),
                    PackedVertex::new(Vector4::new(maxx as f32, miny as f32, 0.0, 1.0), Vector2::new(uvrect.max.x, uvrect.min.y), Color32::new(255, 255, 255, 255), Color32::new(0, 0, 0, 0)),
                    PackedVertex::new(Vector4::new(maxx as f32, maxy as f32, 0.0, 1.0), Vector2::new(uvrect.max.x, uvrect.max.y), Color32::new(255, 255, 255, 255), Color32::new(0, 0, 0, 0)),
                    PackedVertex::new(Vector4::new(minx as f32, maxy as f32, 0.0, 1.0), Vector2::new(uvrect.min.x, uvrect.max.y), Color32::new(255, 255, 255, 255), Color32::new(0, 0, 0, 0)),
                ]
            }).collect();

        let screen_transform = Matrix4x4::projection_ortho(0.0, 640.0, 0.0, 480.0, 0.0, 1.0);
        Matrix4x4::load_simd(&screen_transform);
        Matrix4x4::transform_vertex_simd(&mut vertices.as_mut_slice(), offset_of!(PackedVertex => position));

        vdp::blend_func(vdp::BlendFactor::SrcAlpha, vdp::BlendFactor::OneMinusSrcAlpha);
        vdp::bind_texture(Some(&self.font_texture));
        vdp::draw_geometry_packed(vdp::Topology::TriangleList, vertices.as_slice());
    }
}