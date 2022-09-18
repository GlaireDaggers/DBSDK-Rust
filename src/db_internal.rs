use std::{alloc::Layout, convert::TryInto, os::raw::c_char, ffi::c_void};
use crate::vdp::*;
use crate::math::*;

extern {
    pub fn db_log(strptr: *const c_char);
    pub fn vdp_setVsyncHandler(tick: unsafe extern "C" fn());
    pub fn vdp_clearColor(colorptr: *const Color32);
    pub fn vdp_clearDepth(depth: f32);
    pub fn vdp_depthWrite(enable: bool);
    pub fn vdp_depthFunc(compare: Compare);
    pub fn vdp_blendEquation(mode: BlendEquation);
    pub fn vdp_blendFunc(srcFactor: BlendFactor, dstFactor: BlendFactor);
    pub fn vdp_setWinding(winding: WindingOrder);
    pub fn vdp_setCulling(enabled: bool);
    pub fn vdp_drawGeometry(topology: Topology, first: i32, count: i32, vertexptr: *const Vertex);
    pub fn vdp_allocTexture(mipmap: bool, format: TextureFormat, width: i32, height: i32) -> i32;
    pub fn vdp_releaseTexture(handle: i32);
    pub fn vdp_getUsage() -> i32;
    pub fn vdp_setTextureData(handle: i32, level: i32, data: *const c_void, dataLen: i32);
    pub fn vdp_copyFbToTexture(srcRect: *const Rectangle, dstRect: *const Rectangle, dstTexture: i32);
    pub fn vdp_setSampleParams(filter: TextureFilter, wrapU: TextureWrap, wrapV: TextureWrap);
    pub fn vdp_bindTexture(handle: i32);
    pub fn vdp_viewport(x: i32, y: i32, w: i32, h: i32);
    pub fn vdp_submitDepthQuery(refVal: f32, compare: Compare, x: i32, y: i32, w: i32, h: i32);
    pub fn vdp_getDepthQueryResult() -> i32;
    pub fn mat4_loadSIMD(mat: *const Matrix4x4);
    pub fn mat4_storeSIMD(mat: *mut Matrix4x4);
    pub fn mat4_mulSIMD(mat: *const Matrix4x4);
    pub fn mat4_transformSIMD(invec: *const Vector4, outvec: *const Vector4, count: i32, stride: i32);
}

#[used]
pub static mut ERRNO: i32 = 0;

#[no_mangle]
pub fn __errno_location() -> *mut i32 {
    unsafe {
        let ptr: *mut i32 = &mut ERRNO;
        return ptr;
    }
}

#[no_mangle]
pub fn malloc(size: i32) -> i32 {
    unsafe {
        // basically we just allocate a block of memory with a 4-byte preamble that stores the length
        // that way, we can pass the raw pointer to C, and then when we get the pointer back we do some arithmetic to get at the original preamble
        // and then we can reconstruct the Layout that was passed to alloc

        let layout = Layout::array::<u8>((size + 4).try_into().unwrap()).unwrap();
        let mem = std::alloc::alloc(layout);

        let size_ptr: *mut i32 = std::mem::transmute(mem);
        let data_ptr: usize = std::mem::transmute(mem);
        *size_ptr = size;
  
        return (data_ptr + 4).try_into().unwrap();
    }
}

#[no_mangle]
pub fn free(ptr: i32) {
    unsafe {
        // back up by 4 bytes to get at the preamble, which contains the allocated size
        
        let realptr: usize = (ptr - 4).try_into().unwrap();
        let size_ptr: *mut i32 = std::mem::transmute(realptr);
        let mem: *mut u8 = std::mem::transmute(size_ptr);
        let size = *size_ptr;

        let layout = Layout::array::<u8>((size + 4).try_into().unwrap()).unwrap();
        std::alloc::dealloc(mem, layout);
    }
}