use std::mem::{size_of, align_of};
use std::{alloc::Layout, os::raw::c_char, ffi::c_void};

use crate::clock::DateTime;
use crate::gamepad::GamepadSlot;
use crate::gamepad::GamepadState;
use crate::io::FileMode;
use crate::io::SeekOrigin;
use crate::vdp::*;
use crate::math::*;
use crate::audio::*;

#[repr(C)]
pub struct NativeDirectoryInfo {
    pub name: [i8;32],
    pub created: u64,
    pub modified: u64,
    pub size: i32,
    pub is_directory: u32,
}

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
    pub fn vdp_drawGeometryPacked(topology: Topology, first: i32, count: i32, vertexptr: *const PackedVertex);
    pub fn vdp_allocTexture(mipmap: bool, format: TextureFormat, width: i32, height: i32) -> i32;
    pub fn vdp_releaseTexture(handle: i32);
    pub fn vdp_getUsage() -> i32;
    pub fn vdp_setTextureData(handle: i32, level: i32, data: *const c_void, dataLen: i32);
    pub fn vdp_setTextureDataYUV(handle: i32, yData: *const c_void, yDataLen: i32, uData: *const c_void, uDataLen: i32, vData: *const c_void, vDataLen: i32);
    pub fn vdp_setTextureDataRegion(handle: i32, level: i32, dstRect: *const Rectangle, data: *const c_void, dataLen: i32);
    pub fn vdp_copyFbToTexture(srcRect: *const Rectangle, dstRect: *const Rectangle, dstTexture: i32);
    pub fn vdp_setSampleParams(filter: TextureFilter, wrapU: TextureWrap, wrapV: TextureWrap);
    pub fn vdp_setVUCData(offset: i32, data: *const c_void);
    pub fn vdp_setVULayout(slot: i32, offset: i32, format: VertexSlotFormat);
    pub fn vdp_setVUStride(stride: i32);
    pub fn vdp_uploadVUProgram(program: *const c_void, programLen: i32);
    pub fn vdp_submitVU(topology: Topology, data: *const c_void, dataLen: i32);
    pub fn vdp_setSampleParamsSlot(slot: TextureUnit, filter: TextureFilter, wrap_u: TextureWrap, wrap_v: TextureWrap);
    pub fn vdp_bindTextureSlot(slot: TextureUnit, handle: i32);
    pub fn vdp_setTexCombine(tex_combine: TexCombine, vtx_combine: TexCombine);
    pub fn vdp_allocRenderTexture(width: i32, height: i32) -> i32;
    pub fn vdp_setRenderTarget(handle: i32);
    pub fn vdp_bindTexture(handle: i32);
    pub fn vdp_viewport(x: i32, y: i32, w: i32, h: i32);
    pub fn vdp_submitDepthQuery(refVal: f32, compare: Compare, x: i32, y: i32, w: i32, h: i32);
    pub fn vdp_getDepthQueryResult() -> i32;
    pub fn mat4_loadSIMD(mat: *const Matrix4x4);
    pub fn mat4_storeSIMD(mat: *mut Matrix4x4);
    pub fn mat4_mulSIMD(mat: *const Matrix4x4);
    pub fn mat4_transformSIMD(invec: *const Vector4, outvec: *const Vector4, count: i32, stride: i32);
    pub fn audio_alloc(data: *const c_void, dataLen: i32, audioFmt: i32) -> i32;
    pub fn audio_allocCompressed(data: *const c_void, dataLen: i32, chunkLen: i32) -> i32;
    pub fn audio_free(handle: i32);
    pub fn audio_getUsage() -> i32;
    pub fn audio_queueSetParam_i(slot: i32, param: AudioVoiceParam, value: i32, time: f64);
    pub fn audio_queueSetParam_f(slot: i32, param: AudioVoiceParam, value: f32, time: f64);
    pub fn audio_queueStartVoice(slot: i32, time: f64);
    pub fn audio_queueStopVoice(slot: i32, time: f64);
    pub fn audio_getVoiceState(slot: i32) -> bool;
    pub fn audio_getTime() -> f64;
    pub fn audio_setReverbParams(roomSize: f32, damping: f32, width: f32, wet: f32, dry: f32);
    pub fn audio_initSynth(dataPtr: *const u8, dataLen: i32) -> bool;
    pub fn audio_playMidi(dataPtr: *const u8, dataLen: i32, looping: bool) -> bool;
    pub fn audio_setMidiReverb(enable: bool);
    pub fn audio_setMidiVolume(volume: f32);
    pub fn gamepad_isConnected(slot: GamepadSlot) -> bool;
    pub fn gamepad_readState(slot: GamepadSlot, ptr: *mut GamepadState);
    pub fn gamepad_setRumble(slot: GamepadSlot, enable: bool);
    pub fn fs_deviceExists(devstr: *const c_char) -> bool;
    pub fn fs_deviceEject(devstr: *const c_char);
    pub fn fs_fileExists(pathstr: *const c_char) -> bool;
    pub fn fs_open(pathstr: *const c_char, mode: FileMode) -> i32;
    pub fn fs_read(handle: i32, buffer: *mut c_void, bufferLen: i32) -> i32;
    pub fn fs_write(handle: i32, buffer: *const c_void, bufferLen: i32) -> i32;
    pub fn fs_seek(handle: i32, position: i32, whence: SeekOrigin) -> i32;
    pub fn fs_tell(handle: i32) -> i32;
    pub fn fs_flush(handle: i32);
    pub fn fs_close(handle: i32);
    pub fn fs_eof(handle: i32) -> bool;
    pub fn fs_openDir(pathstr: *const c_char) -> i32;
    pub fn fs_readDir(dir: i32) -> *const NativeDirectoryInfo;
    pub fn fs_rewindDir(dir: i32);
    pub fn fs_closeDir(dir: i32);
    pub fn fs_allocMemoryCard(filenamestr: *const c_char, icondata: *const u8, iconpalette: *const u16, blocks: i32) -> i32;
    pub fn clock_getTimestamp() -> u64;
    pub fn clock_timestampToDatetime(timestamp: u64, datetime: *mut DateTime);
    // pub fn clock_datetimeToTimestamp(datetime: *const DateTime) -> u64;
}

const SIZE_SIZE: usize = size_of::<i64>();
const MAX_ALIGN: usize = align_of::<i64>();

#[used]
pub static mut ERRNO: i32 = 0;

#[no_mangle]
pub fn __errno_location() -> *mut i32 {
    &raw mut ERRNO
}

#[no_mangle]
pub fn malloc(size: i32) -> *mut c_void {
    // basically we just allocate a block of memory with an 8-byte preamble that stores the length (we use 8 bytes to maintain alignment) 
    // that way, we can pass the raw pointer to C, and then when we get the pointer back we do some arithmetic to get at the original preamble
    // and then we can reconstruct the Layout that was passed to alloc

    // NOTE: we align to align_of::<i64>() which is the equivalent of C's max_align_t for wasm32
    // this matches the behavior of C's malloc

    // NOTE: removed write_unaligned b/c it is no longer necessary - malloc is already 8-byte aligned

    let actual_size = SIZE_SIZE + usize::try_from(size).unwrap();
    let layout = Layout::array::<u8>(actual_size).unwrap().align_to(MAX_ALIGN).unwrap();
    let mem = unsafe { std::alloc::alloc(layout) };
    if !mem.is_null() {
        unsafe { mem.cast::<i64>().write(size.into()) };
    }
    unsafe { mem.add(SIZE_SIZE) }.cast()
}

#[no_mangle]
pub fn free(ptr: *mut c_void) {
    // back up by 8 bytes to get at the preamble, which contains the allocated size

    // NOTE: removed read_unaligned b/c it is no longer necessary - malloc is already 8-byte aligned

    let ptr = unsafe { ptr.sub(SIZE_SIZE) }.cast::<u8>();
    let size = unsafe { ptr.cast::<i64>().read() };
    let actual_size = SIZE_SIZE + usize::try_from(size).unwrap();
    let layout = Layout::array::<u8>(actual_size).unwrap().align_to(MAX_ALIGN).unwrap();
    unsafe { std::alloc::dealloc(ptr, layout) };
}