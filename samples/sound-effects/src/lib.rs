#[macro_use]
extern crate lazy_static;
extern crate dbsdk_rs;

use std::sync::{RwLock, Arc, Weak};

use dbsdk_rs::{vdp, io, math::{Vector4, Matrix4x4}, field_offset::offset_of, db, sounddriver::{SoundDriver, self, SoundEmitter}, audio::AudioSample, gamepad::{Gamepad, GamepadSlot, GamepadState, GamepadButton}};

lazy_static! {
    static ref SOUND_DRIVER: RwLock<SoundDriver> = RwLock::new(SoundDriver::new(32));
    static ref WATER_SOUND: Arc<AudioSample> = {
        let mut wav_file = io::FileStream::open("/cd/content/stream.wav", io::FileMode::Read)
            .expect("Failed to open wav file");
        let sfx = sounddriver::load_wav(&mut wav_file).expect("Failed to load wav file");
        Arc::new(sfx)
    };
    static ref EXPLOSION_SOUND: Arc<AudioSample> = {
        let mut wav_file = io::FileStream::open("/cd/content/explosion.wav", io::FileMode::Read)
            .expect("Failed to open wav file");
        let sfx = sounddriver::load_wav(&mut wav_file).expect("Failed to load wav file");
        Arc::new(sfx)
    };
    static ref WATER_EMITTER: Weak<RwLock<SoundEmitter>> = {
        let mut sound_driver = SOUND_DRIVER.write().unwrap();
        sound_driver.play(128, &WATER_SOUND, true, false, 1.0, 1.0, 0.5)
    };
    static ref GAMEPAD: Gamepad = Gamepad::new(GamepadSlot::SlotA);
    static ref GAMEPAD_PREV_STATE: RwLock<GamepadState> = RwLock::new(GAMEPAD.read_state());
}

fn tick() {
    let mut sound_driver = SOUND_DRIVER.write().unwrap();
    sound_driver.update();

    let mut prev_gp_state = GAMEPAD_PREV_STATE.write().unwrap();
    let cur_gp_state = GAMEPAD.read_state();

    if cur_gp_state.button_mask.contains(GamepadButton::A) && !prev_gp_state.button_mask.contains(GamepadButton::A) {
        sound_driver.play(128, &EXPLOSION_SOUND, false, false, 1.0, 1.0, 0.0);
    }

    *prev_gp_state = cur_gp_state;

    vdp::clear_color(vdp::Color32::new(128, 128, 255, 255));
    vdp::clear_depth(1.0);

    vdp::depth_write(false);
    vdp::depth_func(vdp::Compare::Always);

    let mut triangles = [
        vdp::Vertex::new(
            Vector4::new(0.0, 0.5, 0.0, 1.0),
            Vector4::new(1.0, 0.0, 0.0, 1.0),
            Vector4::zero(),
            Vector4::zero()
        ),
        vdp::Vertex::new(
            Vector4::new(-0.5, -0.5, 0.0, 1.0),
            Vector4::new(0.0, 1.0, 0.0, 1.0),
            Vector4::zero(),
            Vector4::zero()
        ),
        vdp::Vertex::new(
            Vector4::new(0.5, -0.5, 0.0, 1.0),
            Vector4::new(0.0, 0.0, 1.0, 1.0),
            Vector4::zero(),
            Vector4::zero()
        ),
    ];

    let ortho = Matrix4x4::projection_ortho_aspect(640.0 / 480.0, 1.0, 0.0, 1.0);
    Matrix4x4::load_simd(&ortho);
    Matrix4x4::transform_vertex_simd(&mut triangles, offset_of!(vdp::Vertex => position));

    vdp::draw_geometry(vdp::Topology::TriangleList, 0, 3, &triangles);
}

#[no_mangle]
pub fn main(_: i32, _: i32) -> i32 {
    db::register_panic();
    lazy_static::initialize(&SOUND_DRIVER);
    lazy_static::initialize(&WATER_EMITTER);
    vdp::set_vsync_handler(Some(tick));
    return 0;
}