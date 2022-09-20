#[macro_use]
extern crate lazy_static;

extern crate dbsdk_rs;
use std::{sync::RwLock, convert::TryInto};

use dbsdk_rs::{vdp, math::{Vector4, Matrix4x4}, field_offset::offset_of, db, io};

lazy_static! {
    static ref BRICK_TEXTURE: RwLock<vdp::Texture> = {
        let brick_tex_file = io::FileStream::open("/cd/content/brickTex.qoi", io::FileMode::Read).expect("Failed to open brickTex.qoi");

        // read in entirety of file
        let size = brick_tex_file.seek(0, io::SeekOrigin::End).unwrap();
        brick_tex_file.seek(0, io::SeekOrigin::Begin).unwrap();

        let mut brick_tex_bytes: Vec<u8> = vec![0;size.try_into().unwrap()];
        brick_tex_file.read(brick_tex_bytes.as_mut_slice()).unwrap();

        // decode qoi image
        let (header, decoded) = qoi::decode_to_vec(brick_tex_bytes).expect("Failed to decode brickTex.qoi");

        let tex = vdp::Texture::new(
            header.width.try_into().unwrap(),
            header.height.try_into().unwrap(),
            false,
            vdp::TextureFormat::RGBA8888).expect("Failed allocating texture");

        match header.channels {
            qoi::Channels::Rgba => {
                // RGBA can just be copied as-is
                tex.set_texture_data(0, &decoded.as_slice());
            }
            qoi::Channels::Rgb => {
                // need to convert from RGB to RGBA
                // qoi crate is *supposed* to support this on our behalf, but support for creating a custom Decoder seems broken right now.
                // oh well!
                let mut pix_data: Vec<vdp::Color32> = vec![vdp::Color32::new(0, 0, 0, 0);decoded.len() / 3];
                for i in 0..pix_data.len() {
                    pix_data[i] = vdp::Color32::new(
                        decoded[i * 3],
                        decoded[(i * 3) + 1],
                        decoded[(i * 3) + 2],
                        255);
                }
                tex.set_texture_data(0, &pix_data.as_slice());
            }
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