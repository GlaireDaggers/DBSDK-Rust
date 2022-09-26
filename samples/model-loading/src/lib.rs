#[macro_use]
extern crate lazy_static;
extern crate dbsdk_rs;
extern crate half;
extern crate byteorder;
extern crate ktx;

use std::{sync::{RwLock, Arc}, convert::TryInto};

use dbanim::{DBAnimationClip, AnimationCurveLoopMode};
use dbmesh::{DBMesh, DBMeshPart, DBSkeleton, DBSkelNode};
use dbsdk_rs::{vdp::{self, Vertex, Texture, WindingOrder, BlendEquation, BlendFactor}, math::{Vector4, Matrix4x4, Vector3, Quaternion}, field_offset::offset_of, db::{self, log}, io::{FileStream, FileMode}};
use lazy_static::initialize;
use sh::SphericalHarmonics;
use ktx::KtxInfo;

mod dbmesh;
mod dbanim;
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
    anim: DBAnimationClip,
}

fn sample_anim_node(node: &DBSkelNode, anim: &DBAnimationClip, time: f32, loopmode: AnimationCurveLoopMode, parent_mat: Matrix4x4, bonepalette: &mut [Matrix4x4]) {
    let mut local_pos = Vector3::zero();
    let mut local_rot = Quaternion::identity();
    let mut local_scale = Vector3::new(1.0, 1.0, 1.0);

    match anim.get_channel_vec3(node.bone_index as u32, 0) {
        Some(channel) => {
            local_pos = match channel.sample(time, loopmode) {
                Ok(v) => { v }
                Err(_) => { Vector3::zero() }
            };
        }
        None => {
        }
    };

    match anim.get_channel_quat(node.bone_index as u32, 1) {
        Some(channel) => {
            local_rot = match channel.sample(time, loopmode) {
                Ok(v) => { v }
                Err(_) => { Quaternion::identity() }
            };
        }
        None => {
        }
    };

    match anim.get_channel_vec3(node.bone_index as u32, 2) {
        Some(channel) => {
            local_scale = match channel.sample(time, loopmode) {
                Ok(v) => { v }
                Err(_) => { Vector3::new(1.0, 1.0, 1.0) }
            };
        }
        None => {
        }
    };

    // compute skinning matrix
    // in order, this matrix:
    // - transforms vertex into bone local space
    // - applies animation transform relative to rest pose
    // - applies rest pose
    // - transforms vertex back into object space (using accumulated parent transform)

    let object_to_bone = node.inv_bind_pose;

    // compute bone to object
    let mut bone_to_object = Matrix4x4::identity();
    Matrix4x4::load_simd(&Matrix4x4::scale(local_scale));
    Matrix4x4::mul_simd(&Matrix4x4::rotation(local_rot));
    Matrix4x4::mul_simd(&Matrix4x4::translation(local_pos));
    Matrix4x4::mul_simd(&node.local_rest_pose);
    Matrix4x4::mul_simd(&parent_mat);
    Matrix4x4::store_simd(&mut bone_to_object);

    // compute skinning matrix
    let mut skin_mat = Matrix4x4::identity();
    Matrix4x4::load_simd(&object_to_bone);
    Matrix4x4::mul_simd(&bone_to_object);
    Matrix4x4::store_simd(&mut skin_mat);

    // write result to bone matrix palette
    bonepalette[node.bone_index as usize] = skin_mat;

    // iterate children
    for child in &node.children {
        sample_anim_node(child, anim, time, loopmode, bone_to_object, bonepalette);
    }
}

fn sample_anim(skeleton: &DBSkeleton, anim: &DBAnimationClip, time: f32, loopmode: AnimationCurveLoopMode, bonepalette: &mut [Matrix4x4]) {
    for root in skeleton.nodes.as_slice() {
        sample_anim_node(root, anim, time, loopmode, Matrix4x4::identity(), bonepalette);
    }
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

        let mut animfile = FileStream::open("/cd/content/leigh_run.dba", FileMode::Read).expect("Failed opening animation");
        let anim = DBAnimationClip::new(&mut animfile).expect("Failed parsing animation file");

        return MyApp {
            t: 0.0,
            mesh: mesh,
            anim: anim,
        };
    }
}

fn draw_meshpart(meshpart: &DBMeshPart, mvp: &Matrix4x4, light: &SphericalHarmonics, bonepalette: &[Matrix4x4]) {
    let mut light_dir = Vector3::new(0.5, 0.5, 0.5);
    light_dir.normalize();

    // unpack mesh part vertices into GPU vertices
    let mut vtx_buffer: Vec<Vertex> = Vec::new();
    for vertex in meshpart.vertices.as_slice() {
        let mut nrm = Vector4::new(vertex.nrm[0].to_f32(), vertex.nrm[1].to_f32(), vertex.nrm[2].to_f32(), 0.0);

        // skinning
        let mut vtx = Vector4::new(vertex.pos[0].to_f32(), vertex.pos[1].to_f32(), vertex.pos[2].to_f32(), 1.0);
        let mut sk0 = vtx;
        let mut sk1 = vtx;
        let mut nrm0 = nrm;
        let mut nrm1 = nrm;

        if vertex.bweight[0] > 0 {
            sk0 = bonepalette[vertex.bidx[0] as usize] * sk0;
            nrm0 = bonepalette[vertex.bidx[0] as usize] * nrm0;
        }

        if vertex.bweight[1] > 0 {
            sk1 = bonepalette[vertex.bidx[1] as usize] * sk1;
            nrm1 = bonepalette[vertex.bidx[1] as usize] * nrm1;
        }

        let weight0 = (vertex.bweight[0] as f32) / 255.0;
        let weight1 = (vertex.bweight[1] as f32) / 255.0;

        vtx = (sk0 * weight0) + (sk1 * weight1);
        nrm = (nrm0 * weight0) + (nrm1 * weight1);

        vtx_buffer.push(Vertex::new(
            vtx,
            nrm,
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
    //let rot = Matrix4x4::identity();
    let translate = Matrix4x4::translation(Vector3::new(0.0, -1.25, -4.0));
    let proj = Matrix4x4::projection_perspective(640.0 / 480.0, (60.0 as f32).to_radians(), 0.1, 500.0);
    
    let mut mvp = Matrix4x4::identity();
    Matrix4x4::load_simd(&rot);
    Matrix4x4::mul_simd(&translate);
    Matrix4x4::mul_simd(&proj);
    Matrix4x4::store_simd(&mut mvp);

    // compute animation
    let mut bone_palette: Vec<Matrix4x4> = vec![Matrix4x4::identity();my_app.mesh.skeleton.as_ref().unwrap().bone_count as usize];
    sample_anim(&my_app.mesh.skeleton.as_ref().unwrap(), &my_app.anim, my_app.t, AnimationCurveLoopMode::Repeat, &mut bone_palette.as_mut_slice());

    let mut sh = SphericalHarmonics::new();
    sh.add_ambient_light(Vector3::new(0.05, 0.05, 0.1));
    sh.add_directional_light(Vector3::new(0.5, 0.5, 0.5), Vector3::new(2.0, 2.0, 2.0));
    sh.add_directional_light(Vector3::new(-0.5, -0.5, -0.5), Vector3::new(0.5, 0.1, 1.0));

    for meshpart in my_app.mesh.mesh_parts.as_slice() {
        draw_meshpart(meshpart, &mvp, &sh, bone_palette.as_slice());
    }
}

#[no_mangle]
pub fn main(_: i32, _: i32) -> i32 {
    db::register_panic();
    initialize(&MY_APP);
    vdp::set_vsync_handler(Some(tick));
    return 0;
}