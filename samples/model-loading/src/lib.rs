#[macro_use]
extern crate lazy_static;
extern crate dbsdk_rs;
extern crate half;
extern crate byteorder;
extern crate ktx;

use std::{sync::{RwLock, Arc}, convert::TryInto};

use dbmesh::{DBMesh, DBMeshPart};
use dbsdk_rs::{vdp::{self, Vertex, Texture, WindingOrder, BlendEquation, BlendFactor}, math::{Vector4, Matrix4x4, Vector3, Quaternion}, field_offset::offset_of, db::{self, log}, io::{FileStream, FileMode}};
use lazy_static::initialize;
use sh::SphericalHarmonics;
use ktx::KtxInfo;

mod dbmesh;
mod sh;

const GL_RGB: u32 = 0x1907;
const GL_RGBA: u32 = 0x1908;
const GL_UNSIGNED_BYTE: u32 = 0x1401;
const GL_UNSIGNED_SHORT_5_6_5: u32 = 0x8363;
const GL_UNSIGNED_SHORT_4_4_4_4: u32 = 0x8033;
const GL_COMPRESSED_RGB_S3TC_DXT1_EXT: u32 = 0x83F0;
const GL_COMPRESSED_RGBA_S3TC_DXT1_EXT: u32 = 0x83F1;
const GL_COMPRESSED_RGBA_S3TC_DXT3_EXT: u32 = 0x83F2;

struct MyApp {
    t: f32,
    mesh: DBMesh,
}

fn load_tex(id: &str) -> Result<Arc<Texture>,()> {
    let texfile = match FileStream::open(format!("/cd/content/{}.KTX", id).as_str(), FileMode::Read) {
        Ok(v) => { v },
        Err(e) => {
            log(format!("Failed opening texture file: {:?}", e).as_str());
            return Err(());
        }
    };

    // decode KTX texture
    let decoder = ktx::Decoder::new(texfile).expect("Failed decoding KTX image");

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

    // return
    Ok(Arc::new(tex))
}

lazy_static! {
    static ref MY_APP: RwLock<MyApp> = RwLock::new(MyApp::new());
}

impl MyApp {
    pub fn new() -> MyApp {
        let mut meshfile = FileStream::open("/cd/content/leigh.dbm", FileMode::Read).expect("Failed opening mesh");
        let mesh = DBMesh::new(&mut meshfile, |material_name| {
            load_tex(material_name)
        }).expect("Failed parsing mesh file");

        return MyApp {
            t: 0.0,
            mesh: mesh
        };
    }
}

fn draw_meshpart(meshpart: &DBMeshPart, mvp: &Matrix4x4, light: &SphericalHarmonics) {
    let mut light_dir = Vector3::new(0.5, 0.5, 0.5);
    light_dir.normalize();

    // unpack mesh part vertices into GPU vertices
    let mut vtx_buffer: Vec<Vertex> = Vec::new();
    for vertex in meshpart.vertices.as_slice() {
        let nrm = Vector3::new(vertex.nrm[0].to_f32(), vertex.nrm[1].to_f32(), vertex.nrm[2].to_f32());
        vtx_buffer.push(Vertex::new(
            Vector4::new(vertex.pos[0].to_f32(), vertex.pos[1].to_f32(), vertex.pos[2].to_f32(), 1.0),
            Vector4::new(nrm.x, nrm.y, nrm.z, 0.0),
            Vector4::zero(), 
            Vector4::new(vertex.tex[0].to_f32(), vertex.tex[1].to_f32(), 0.0, 0.0)));
    }

    // transform vertex positions & normals (note: normal has been packed into color field with w=0.0)
    Matrix4x4::load_simd(&meshpart.transform);
    Matrix4x4::mul_simd(mvp);
    Matrix4x4::transform_vertex_simd(vtx_buffer.as_mut_slice(), offset_of!(Vertex => position));
    Matrix4x4::transform_vertex_simd(vtx_buffer.as_mut_slice(), offset_of!(Vertex => color));

    // set each normal W to 1.0 for lighting transform
    for v in vtx_buffer.as_mut_slice() {
        v.color.w = 1.0;
    }

    // transform normals into light color
    Matrix4x4::load_simd(&light.coeff);
    Matrix4x4::transform_vertex_simd(vtx_buffer.as_mut_slice(), offset_of!(Vertex => color));

    // set render state
    vdp::set_culling(meshpart.material.enable_cull);
    vdp::set_winding(WindingOrder::CounterClockwise);
    match &meshpart.material.texture {
        Some(v) => {
            vdp::bind_texture(Some(v.as_ref()));
        },
        None => {
            vdp::bind_texture(None);
        }
    };

    vdp::blend_equation(BlendEquation::Add);

    if meshpart.material.blend_enable {
        vdp::blend_func(BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha);
        vdp::depth_write(false);
    } else {
        vdp::blend_func(BlendFactor::One, BlendFactor::Zero);
        vdp::depth_write(true);
    }

    // draw
    vdp::draw_geometry(vdp::Topology::TriangleList, vtx_buffer.as_slice());
}

fn tick() {
    let my_app = &mut MY_APP.write().unwrap();

    my_app.t += 1.0 / 60.0;

    vdp::clear_color(vdp::Color32::new(128, 128, 255, 255));
    vdp::clear_depth(1.0);

    vdp::depth_write(true);
    vdp::depth_func(vdp::Compare::Less);

    vdp::set_culling(true);

    // load MVP matrix into SIMD register
    let rot = Matrix4x4::rotation(Quaternion::from_euler(Vector3::new(0.0, (my_app.t * 45.0).to_radians(), 0.0)));
    let translate = Matrix4x4::translation(Vector3::new(0.0, -1.25, -4.0));
    let proj = Matrix4x4::projection_perspective(640.0 / 480.0, (60.0 as f32).to_radians(), 0.1, 500.0);
    Matrix4x4::load_simd(&rot);
    Matrix4x4::mul_simd(&translate);
    Matrix4x4::mul_simd(&proj);

    let mut mvp = Matrix4x4::identity();
    Matrix4x4::store_simd(&mut mvp);

    let mut sh = SphericalHarmonics::new();
    sh.add_ambient_light(Vector3::new(0.05, 0.05, 0.1));
    sh.add_directional_light(Vector3::new(0.5, 0.5, 0.5), Vector3::new(2.0, 2.0, 2.0));
    sh.add_directional_light(Vector3::new(-0.5, -0.5, -0.5), Vector3::new(0.5, 0.1, 1.0));

    for meshpart in my_app.mesh.mesh_parts.as_slice() {
        draw_meshpart(meshpart, &mvp, &sh);
    }
}

#[no_mangle]
pub fn main(_: i32, _: i32) -> i32 {
    db::register_panic();
    initialize(&MY_APP);
    vdp::set_vsync_handler(Some(tick));
    return 0;
}