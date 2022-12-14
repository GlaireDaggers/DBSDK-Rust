#[macro_use]
extern crate lazy_static;
extern crate dbsdk_rs;
extern crate byteorder;
extern crate roq_dec;

use std::sync::RwLock;

use dbsdk_rs::{vdp::{self, Texture, TextureFormat}, math::Vector4, db::{self, log}, io::{FileStream, self}, audio::{AudioSample, self}};
use roq_dec::roq_dec::{RoqDecoder, ColorspaceBgr565, RoqEvent};

const LOOKAHEAD_TIME: f64 = 0.25;

struct MyApp {
    roq_player: RoqDecoder<FileStream,u16,ColorspaceBgr565>,
    frame_timer: f32,
    vid_texture: Option<Texture>,
    audio_buf: [[Option<AudioSample>;3];2],
    audio_queue: [Option<Vec<i16>>;2],
    audio_schedule_time: f64,
    next_buf: usize,
    playing: bool,
}

impl MyApp {
    pub fn new() -> MyApp {
        let roqstream = FileStream::open("/cd/content/HERO.roq", io::FileMode::Read).expect("Failed opening ROQ video");
        let roqplayer = RoqDecoder::new(roqstream).expect("Failed creating ROQ player");

        return MyApp {
            roq_player: roqplayer,
            frame_timer: 0.0,
            vid_texture: None,
            audio_buf: [[None, None, None], [None, None, None]],
            audio_queue: [None, None],
            audio_schedule_time: -1.0,
            next_buf: 0,
            playing: true,
        };
    }

    fn schedule_voice(handle: i32, slot: i32, pan: f32, t: f64) {
        audio::queue_set_voice_param_i(slot, audio::AudioVoiceParam::SampleData, handle, t);
        audio::queue_set_voice_param_i(slot, audio::AudioVoiceParam::Samplerate, 22050, t);
        audio::queue_set_voice_param_i(slot, audio::AudioVoiceParam::LoopEnabled, 0, t);
        audio::queue_set_voice_param_i(slot, audio::AudioVoiceParam::Reverb, 0, t);
        audio::queue_set_voice_param_f(slot, audio::AudioVoiceParam::Volume, 1.0, t);
        audio::queue_set_voice_param_f(slot, audio::AudioVoiceParam::Pitch, 1.0, t);
        audio::queue_set_voice_param_f(slot, audio::AudioVoiceParam::Detune, 0.0, t);
        audio::queue_set_voice_param_f(slot, audio::AudioVoiceParam::Pan, pan, t);
        audio::queue_set_voice_param_f(slot, audio::AudioVoiceParam::FadeInDuration, 0.0, t);
        audio::queue_set_voice_param_f(slot, audio::AudioVoiceParam::FadeOutDuration, 0.0, t);

        audio::queue_stop_voice(slot, t);
        audio::queue_start_voice(slot, t);
    }

    pub fn next_frame(&mut self) {
        if self.playing {
            self.frame_timer -= 1.0 / 60.0;
            if self.frame_timer <= 0.0 {
                // keep reading until we hit EOF or a Video chunk
                loop {
                    match self.roq_player.read_next().expect("Failed reading next event") {
                        RoqEvent::InitVideo => {
                            // DreamBox only supports power of two textures
                            assert!(self.roq_player.width & (self.roq_player.width - 1) == 0, "Unsupported ROQ player width (must be power of two)");
                            assert!(self.roq_player.height & (self.roq_player.height - 1) == 0, "Unsupported ROQ player height (must be power of two)");

                            log(format!("Initializing video {}x{} (framerate: {})", self.roq_player.width, self.roq_player.height, self.roq_player.framerate).as_str());
                            // set up video texture
                            self.vid_texture = Some(Texture::new(
                                self.roq_player.width as i32,
                                self.roq_player.height as i32,
                                false, TextureFormat::RGB565).expect("Failed allocating texture"));
                        },
                        RoqEvent::Video(framebuffer) => {
                            // upload frame data
                            let tex = self.vid_texture.as_ref().expect("Received video event before init event - this is invalid");
                            tex.set_texture_data(0, framebuffer);
                            break;
                        },
                        RoqEvent::Audio(channels, samples) => {
                            if self.audio_schedule_time < audio::get_time() {
                                log(format!("Audio schedule time fell behind real time, recovering...").as_str());
                                self.audio_schedule_time = audio::get_time();
                            }

                            // note: ROQ audio is always 22050 Hz
                            let t = self.audio_schedule_time + LOOKAHEAD_TIME;
                            self.audio_schedule_time += ((samples.len() / channels as usize) as f64) / 22050.0;

                            // schedule audio buffers
                            if channels == 1 {
                                // mono

                                // we have a rotating buffer of audio samples we use to upload audio data
                                // NOTE: this will automatically deallocate the previous buffer here

                                // this is a little tricky:
                                // basically, instead of queueing audio chunks right away, we actually stuff them into a buffer and wait
                                // then, when we get the next buffer, we actually take its first sample and append it to the start of the LAST buffer and submit that
                                // this is all to make DreamBox's 2-tap sampling play nicely - b/c at the end of one of our submitted samples, DreamBox doesn't take the next sample we queue up into account,
                                // so there's a single sample of aliasing in between every single buffer we submit and it ends up sounding scratchy
                                // this fixes that by basically making each buffer end with the next buffer's starting sample

                                match &mut self.audio_queue[0] {
                                    Some(v) => {
                                        // had a previous buffer, append the first sample of this new buffer to the end and queue that
                                        v.push(samples[0]);
                                        let buf_l = &mut self.audio_buf[0][self.next_buf % 3];
                                        let newbuf_l = AudioSample::create_s16(v, 22050).expect("Failed creating audio sample");
                                        let handle_l = newbuf_l.handle;
                                        *buf_l = Some(newbuf_l);
                                        MyApp::schedule_voice(handle_l, 30, -1.0, t);
                                    },
                                    None => {
                                    }
                                };
                                
                                // replace audio in the queue with new chunk
                                let mut buf: Vec<i16> = Vec::new();
                                buf.extend_from_slice(samples);
                                self.audio_queue[0] = Some(buf);
                            } else if channels == 2 {
                                // stereo

                                // this is a little more involved b/c we need to basically unzip the interleaved stereo buffer into *two* mono buffers
                                // (remember: DreamBox audio samples are ALWAYS mono, so we actually play two different audio samples planned left+right)

                                let sample_cnt = samples.len() / 2;
                                let mut data_l: Vec<i16> = vec![0;sample_cnt];
                                let mut data_r: Vec<i16> = vec![0;sample_cnt];

                                for i in 0..sample_cnt {
                                    data_l[i] = samples[i * 2];
                                    data_r[i] = samples[i * 2 + 1];
                                }

                                // we have a rotating buffer of audio samples we use to upload audio data
                                // NOTE: this will automatically deallocate the previous buffers here

                                // this is a little tricky:
                                // basically, instead of queueing audio chunks right away, we actually stuff them into a buffer and wait
                                // then, when we get the next buffer, we actually take its first sample and append it to the start of the LAST buffer and submit that
                                // this is all to make DreamBox's 2-tap sampling play nicely - b/c at the end of one of our submitted samples, DreamBox doesn't take the next sample we queue up into account,
                                // so there's a single sample of aliasing in between every single buffer we submit and it ends up sounding scratchy
                                // this fixes that by basically making each buffer end with the next buffer's starting sample

                                match &mut self.audio_queue[0] {
                                    Some(v) => {
                                        // had a previous buffer, append the first sample of this new buffer to the end and queue that
                                        v.push(data_l[0]);
                                        let buf_l = &mut self.audio_buf[0][self.next_buf % 3];
                                        let newbuf_l = AudioSample::create_s16(v, 22050).expect("Failed creating audio sample");
                                        let handle_l = newbuf_l.handle;
                                        *buf_l = Some(newbuf_l);
                                        MyApp::schedule_voice(handle_l, 30, -1.0, t);
                                    },
                                    None => {
                                    }
                                };

                                match &mut self.audio_queue[1] {
                                    Some(v) => {
                                        // had a previous buffer, append the first sample of this new buffer to the end and queue that
                                        v.push(data_r[0]);
                                        let buf_r = &mut self.audio_buf[1][self.next_buf % 3];
                                        let newbuf_r = AudioSample::create_s16(v, 22050).expect("Failed creating audio sample");
                                        let handle_r = newbuf_r.handle;
                                        *buf_r = Some(newbuf_r);
                                        MyApp::schedule_voice(handle_r, 31, 1.0, t);
                                    },
                                    None => {
                                    }
                                };

                                // replace audio in the queue with new chunk
                                self.audio_queue[0] = Some(data_l);
                                self.audio_queue[1] = Some(data_r);
                            }

                            self.next_buf += 1;
                        },
                        RoqEvent::EndOfFile => {
                            // video is done playing
                            log("Reached end of file");
                            self.playing = false;

                            // drop audio buffers
                            self.audio_buf = [[None, None, None], [None, None, None]];

                            // drop video texture
                            self.vid_texture = None;
                            return;
                        },
                        RoqEvent::Custom(_chunk_id, _chunk_arg, _chunk_data) => {
                        }
                    };
                }

                self.frame_timer = 1.0 / (self.roq_player.framerate as f32);
            }
        }
    }
}

lazy_static! {
    static ref MY_APP: RwLock<MyApp> = RwLock::new(MyApp::new());
}

fn tick() {
    let mut my_app = MY_APP.write().unwrap();
    my_app.next_frame();

    vdp::clear_color(vdp::Color32::new(128, 128, 255, 255));
    vdp::clear_depth(1.0);

    vdp::depth_write(false);
    vdp::depth_func(vdp::Compare::Always);

    let triangles = [
        vdp::Vertex::new(
            Vector4::new(-1.0, 1.0, 0.0, 1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
            Vector4::zero(),
            Vector4::new(0.0, 0.0, 0.0, 0.0),
        ),
        vdp::Vertex::new(
            Vector4::new(-1.0, -3.0, 0.0, 1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
            Vector4::zero(),
            Vector4::new(0.0, 2.0, 0.0, 0.0),
        ),
        vdp::Vertex::new(
            Vector4::new(3.0, 1.0, 0.0, 1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
            Vector4::zero(),
            Vector4::new(2.0, 0.0, 0.0, 0.0),
        ),
    ];

    match &my_app.vid_texture {
        Some(v) => {
            vdp::bind_texture(Some(v));
        },
        None => {
            vdp::bind_texture(None);
        }
    }
    vdp::draw_geometry(vdp::Topology::TriangleList, &triangles);
}

#[no_mangle]
pub fn main(_: i32, _: i32) -> i32 {
    db::register_panic();
    vdp::set_vsync_handler(Some(tick));
    return 0;
}