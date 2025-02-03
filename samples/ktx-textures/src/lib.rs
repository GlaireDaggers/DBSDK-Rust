#[macro_use]
extern crate lazy_static;
extern crate ktx;
extern crate dbsdk_rs;

use std::{sync::RwLock, convert::TryInto};
use ktx::KtxInfo;

use dbsdk_rs::{vdp, math::{Vector4, Matrix4x4}, field_offset::offset_of, db, io};

const GL_RGB: u32 = 0x1907;
const GL_RGBA: u32 = 0x1908;
const GL_UNSIGNED_BYTE: u32 = 0x1401;
const GL_UNSIGNED_SHORT_5_6_5: u32 = 0x8363;
const GL_UNSIGNED_SHORT_4_4_4_4: u32 = 0x8033;
const GL_COMPRESSED_RGB_S3TC_DXT1_EXT: u32 = 0x83F0;
const GL_COMPRESSED_RGBA_S3TC_DXT1_EXT: u32 = 0x83F1;
const GL_COMPRESSED_RGBA_S3TC_DXT3_EXT: u32 = 0x83F2;

lazy_static! {
    static ref BRICK_TEXTURE: RwLock<vdp::Texture> = {
        let brick_tex_file = io::FileStream::open("/cd/content/brickTex.ktx", io::FileMode::Read).expect("Failed to open brickTex.ktx");

        // decode KTX texture
        let decoder = ktx::Decoder::new(brick_tex_file).expect("Failed decoding KTX image");

        // find appropriate VDP format
        let tex_fmt = if decoder.gl_type() == GL_UNSIGNED_BYTE && decoder.gl_format() == GL_RGBA {
            vdp::TextureFormat::RGBA8888
        } else if decoder.gl_type() == GL_UNSIGNED_SHORT_5_6_5 && decoder.gl_format() == GL_RGB {
            vdp::TextureFormat::RGB565
        } else if decoder.gl_type() == GL_UNSIGNED_SHORT_4_4_4_4 && decoder.gl_format() == GL_RGBA {
            vdp::TextureFormat::RGBA4444
        } else if decoder.gl_internal_format() == GL_COMPRESSED_RGB_S3TC_DXT1_EXT || decoder.gl_internal_format() == GL_COMPRESSED_RGBA_S3TC_DXT1_EXT {
            vdp::TextureFormat::DXT1
        } else if decoder.gl_internal_format() == GL_COMPRESSED_RGBA_S3TC_DXT3_EXT {
            vdp::TextureFormat::DXT3
        } else {
            panic!("Failed decoding KTX image: format is unsupported");
        };

        // allocate VDP texture
        let tex = vdp::Texture::new(
            decoder.pixel_width().try_into().unwrap(),
            decoder.pixel_height().try_into().unwrap(),
            decoder.mipmap_levels() > 1, tex_fmt)
            .expect("Failed allocating VDP texture");

        // upload each mip slice
        let mut level: i32 = 0;
        for tex_level in decoder.read_textures() {
            tex.set_texture_data(level, &tex_level);
            level += 1;
        }

        RwLock::new(tex)
    };
}

fn tick() {
    vdp::clear_color(vdp::Color32::new(128, 128, 255, 255));
    vdp::clear_depth(1.0);

    vdp::depth_write(false);
    vdp::depth_func(vdp::Compare::Always);

    let mut triangles = [
        vdp::Vertex::new(
            Vector4::new(0.0, 0.5, 0.0, 1.0),
            Vector4::new(1.0, 0.0, 0.0, 1.0),
            Vector4::zero(),
            Vector4::new(0.5, 0.0, 0.0, 0.0)
        ),
        vdp::Vertex::new(
            Vector4::new(-0.5, -0.5, 0.0, 1.0),
            Vector4::new(0.0, 1.0, 0.0, 1.0),
            Vector4::zero(),
            Vector4::new(0.0, 1.0, 0.0, 0.0)
        ),
        vdp::Vertex::new(
            Vector4::new(0.5, -0.5, 0.0, 1.0),
            Vector4::new(0.0, 0.0, 1.0, 1.0),
            Vector4::zero(),
            Vector4::new(1.0, 1.0, 0.0, 0.0)
        ),
    ];

    let ortho = Matrix4x4::projection_ortho_aspect(640.0 / 480.0, 1.0, 0.0, 1.0);
    Matrix4x4::load_simd(&ortho);
    Matrix4x4::transform_vertex_simd(&mut triangles, offset_of!(vdp::Vertex => position));

    let texref = BRICK_TEXTURE.read().unwrap();
    vdp::bind_texture(Some(&texref));
    vdp::draw_geometry(vdp::Topology::TriangleList, &triangles);
}

#[no_mangle]
pub fn main(_: i32, _: i32) -> i32 {
    db::register_panic();
    lazy_static::initialize(&BRICK_TEXTURE);
    vdp::set_vsync_handler(Some(tick));
    return 0;
}