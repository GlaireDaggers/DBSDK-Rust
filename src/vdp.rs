use std::convert::TryInto;
use std::fmt::Debug;

use crate::db_internal::{vdp_clearColor, vdp_setVsyncHandler, vdp_clearDepth, vdp_depthWrite, vdp_depthFunc, vdp_blendEquation, vdp_blendFunc, vdp_setWinding, vdp_setCulling, vdp_drawGeometry, vdp_allocTexture, vdp_releaseTexture, vdp_getUsage, vdp_setTextureData, vdp_copyFbToTexture, vdp_setSampleParams, vdp_bindTexture, vdp_viewport, vdp_submitDepthQuery, vdp_getDepthQueryResult};
use crate::math::Vector4;

static mut VSYNC_HANDLER: Option<fn()> = Option::None;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Color32 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color32 {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Color32 {
        return Color32 { r: r, g: g, b: b, a: a };
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: Vector4,
    pub color: Vector4,
    pub ocolor: Vector4,
    pub texcoord: Vector4,
}

impl Vertex {
    pub const fn new(position: Vector4, color: Vector4, ocolor: Vector4, texcoord: Vector4) -> Vertex {
        return Vertex { position: position, color: color, ocolor: ocolor, texcoord: texcoord };
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rectangle {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Rectangle {
        return Rectangle { x: x, y: y, width: width, height: height };
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TextureError {
    DimensionsInvalid,
    AllocationFailed
}

#[repr(C)]
pub struct Texture {
    pub format: TextureFormat,
    pub width: i32,
    pub height: i32,
    pub mipmap: bool,
    handle: i32,
}

impl Texture {
    pub fn new(width: i32, height: i32, mipmap: bool, format: TextureFormat) -> Result<Texture,TextureError> {
        // dimensions must be power of two
        if (width & (width - 1)) != 0 || (height & (height - 1)) != 0 {
            return Result::Err(TextureError::DimensionsInvalid);
        }

        // allocate and check to see if allocation failed
        let handle = unsafe { vdp_allocTexture(mipmap, format, width, height) };
        if handle == -1 {
            return Result::Err(TextureError::AllocationFailed);
        }

        return Result::Ok(Texture {
            format: format,
            mipmap: mipmap,
            width: width,
            height: height,
            handle: handle
        });
    }

    /// Upload texture data for the given mip level of this texture
    pub fn set_texture_data<T>(&self, level: i32, data: &[T]) {
        unsafe {
            vdp_setTextureData(self.handle, level, data.as_ptr().cast(), data.len().try_into().unwrap());
        }
    }

    /// Copy a region of the framebuffer into a region of the given texture
    pub fn copy_framebuffer_to_texture(target: &Texture, src_rect: Rectangle, dst_rect: Rectangle) {
        unsafe {
            vdp_copyFbToTexture(&src_rect, &dst_rect, target.handle);
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe { vdp_releaseTexture(self.handle) };
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum Compare {
    Never           = 0x0200,
    Less            = 0x0201,
    Equal           = 0x0202,
    LessOrEqual     = 0x0203,
    Greater         = 0x0204,
    NotEqual        = 0x0205,
    GreaterOrEqual  = 0x0206,
    Always          = 0x0207,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum BlendEquation {
    Add                 = 0x8006,
    Subtract            = 0x800A,
    ReverseSubtract     = 0x800B,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum BlendFactor {
    Zero                = 0,
    One                 = 1,
    SrcColor            = 0x0300,
    OneMinusSrcColor    = 0x0301,
    SrcAlpha            = 0x0302,
    OneMinusSrcAlpha    = 0x0303,
    DstAlpha            = 0x0304,
    OneMinusDstAlpha    = 0x0305,
    DstColor            = 0x0306,
    OneMinusDstColor    = 0x0307,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum WindingOrder {
    Clockwise  = 0x0900,
    CounterClockwise = 0x0901,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum Topology {
    LineList       = 0x0000,
    LineStrip      = 0x0001,
    TriangleList   = 0x0002,
    TriangleStrip  = 0x0003,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum TextureFormat {
    RGB565   = 0,
    RGBA4444 = 1,
    RGBA8888 = 2,
    DXT1     = 3,
    DXT3     = 4,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum TextureFilter {
    Nearest     = 0x2600,
    Linear      = 0x2601,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum TextureWrap {
    Clamp       = 0x812F,
    Repeat      = 0x2901,
    Mirror      = 0x8370,
}

unsafe extern "C" fn real_vsync_handler() {
    if VSYNC_HANDLER.is_some() {
        VSYNC_HANDLER.unwrap()();
    }
}

/// Clear the backbuffer to the given color
pub fn clear_color(color: Color32) {
    unsafe { vdp_clearColor(&color); }
}

/// Clear the depth buffer to the given depth value
pub fn clear_depth(depth: f32) {
    unsafe { vdp_clearDepth(depth); }
}

/// Set whether depth writes are enabled
pub fn depth_write(enable: bool) {
    unsafe { vdp_depthWrite(enable) };
}

/// Set the current depth test comparison
pub fn depth_func(compare: Compare) {
    unsafe { vdp_depthFunc(compare) };
}

/// Set the blend equation mode
pub fn blend_equation(mode: BlendEquation) {
    unsafe { vdp_blendEquation(mode) };
}

/// Set the source and destination blend factors
pub fn blend_func(src_factor: BlendFactor, dst_factor: BlendFactor) {
    unsafe { vdp_blendFunc(src_factor, dst_factor) };
}

/// Set the winding order for backface culling
pub fn set_winding(winding: WindingOrder) {
    unsafe { vdp_setWinding(winding) };
}

/// Set backface culling enabled or disabled
pub fn set_culling(enabled: bool) {
    unsafe { vdp_setCulling(enabled) };
}

/// Submit a buffer of geometry to draw
pub fn draw_geometry(topology: Topology, first: i32, count: i32, vertex_data: &[Vertex]) {
    unsafe { vdp_drawGeometry(topology, first, count, vertex_data.as_ptr()) };
}

/// Get total texture memory usage in bytes
pub fn get_usage() -> i32 {
    unsafe { return vdp_getUsage() };
}

/// Set currently active texture sampling parameters
pub fn set_sample_params(filter: TextureFilter, wrap_u: TextureWrap, wrap_v: TextureWrap) {
    unsafe { vdp_setSampleParams(filter, wrap_u, wrap_v) };
}

/// Bind a texture for drawing
pub fn bind_texture(texture: Option<&Texture>) {
    if texture.is_some() {
        unsafe { vdp_bindTexture(texture.unwrap().handle) };
    } else {
        unsafe { vdp_bindTexture(-1) };
    }
}

/// Set the current viewport rect
pub fn viewport(rect: Rectangle) {
    unsafe {
        vdp_viewport(rect.x, rect.y, rect.width, rect.height);
    }
}

/// Compare a region of the depth buffer against the given reference value
pub fn submit_depth_query(ref_val: f32, compare: Compare, rect: Rectangle) {
    unsafe {
        vdp_submitDepthQuery(ref_val, compare, rect.x, rect.y, rect.width, rect.height);
    }
}

/// Get the number of pixels which passed the submitted depth query
pub fn get_depth_query_result() -> i32 {
    unsafe {
        return vdp_getDepthQueryResult();
    }
}

/// Set an optional handler for vertical sync
pub fn set_vsync_handler(handler: Option<fn()>) {
    unsafe {
        VSYNC_HANDLER = handler;
        vdp_setVsyncHandler(real_vsync_handler);
    }
}