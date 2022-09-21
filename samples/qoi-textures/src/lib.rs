#[macro_use]
extern crate lazy_static;

extern crate dbsdk_rs;
use std::{sync::RwLock, convert::TryInto};

use dbsdk_rs::{vdp, math::{Vector4, Matrix4x4}, field_offset::offset_of, db, io};

lazy_static! {
    static ref BRICK_TEXTURE: RwLock<vdp::Texture> = {
        let brick_tex_file = io::FileStream::open("/cd/content/brickTex.qoi", io::FileMode::Read).expect("Failed to open brickTex.qoi");

        // decode qoi image
        let mut decoder = qoi::Decoder::from_stream(brick_tex_file).expect("Failed to decode brickTex.qoi").with_channels(qoi::Channels::Rgba);
        let data = decoder.decode_to_vec().expect("Failed to decode brickTex.qoi");
        let header = decoder.header();

        let tex = vdp::Texture::new(
            header.width.try_into().unwrap(),
            header.height.try_into().unwrap(),
            false,
            vdp::TextureFormat::RGBA8888).expect("Failed allocating texture");

        // RGBA can just be copied as-is
        tex.set_texture_data(0, &data.as_slice());
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