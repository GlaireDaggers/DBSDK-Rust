use crate::db_internal::{vdp_clearColor, vdp_setVsyncHandler, vdp_clearDepth, vdp_depthWrite, vdp_depthFunc, vdp_blendEquation, vdp_blendFunc, vdp_setWinding, vdp_setCulling, vdp_drawGeometry};
use crate::math::Vector4;

static mut VSYNC_HANDLER: Option<fn()> = Option::None;

#[repr(C)]
pub struct Color32 {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Color32 {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Color32 {
        return Color32 { r: r, g: g, b: b, a: a };
    }
}

#[repr(C)]
pub struct Vertex {
    position: Vector4,
    color: Vector4,
    ocolor: Vector4,
    texcoord: Vector4,
}

impl Vertex {
    pub fn new(position: Vector4, color: Vector4, ocolor: Vector4, texcoord: Vector4) -> Vertex {
        return Vertex { position: position, color: color, ocolor: ocolor, texcoord: texcoord };
    }
}

#[repr(C)]
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
pub enum BlendEquation {
    Add                 = 0x8006,
    Subtract            = 0x800A,
    ReverseSubtract     = 0x800B,
}

#[repr(C)]
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
pub enum WindingOrder {
    Clockwise  = 0x0900,
    CounterClockwise = 0x0901,
}

#[repr(C)]
pub enum Topology {
    LineList       = 0x0000,
    LineStrip      = 0x0001,
    TriangleList   = 0x0002,
    TriangleStrip  = 0x0003,
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

/// Set an optional handler for vertical sync
pub fn set_vsync_handler(handler: Option<fn()>) {
    unsafe {
        VSYNC_HANDLER = handler;
        vdp_setVsyncHandler(real_vsync_handler);
    }
}