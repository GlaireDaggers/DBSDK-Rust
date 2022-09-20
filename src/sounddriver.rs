use std::{convert::TryInto, mem::{size_of, transmute}, cell::RefCell, sync::{Weak, RwLock}, sync::Arc};

use crate::{audio::{VOICE_COUNT, AudioSample, get_voice_state, queue_stop_voice, get_time, queue_start_voice, queue_set_voice_param_f, AudioVoiceParam, queue_set_voice_param_i}, math::{Vector3, Quaternion}, io::{FileStream, SeekOrigin}};

#[derive(Clone, Copy)]
pub enum AttenuationType {
    None,
    InverseDistance,
    Linear,
    ExponentialDistance,
}

#[derive(Clone, Copy)]
struct SoundVoice {
    slot: i32,
    priority: u8,
    _is_playing: bool,
    id: u32,
    play_time: f64,
}

pub struct SoundEmitter {
    pub is_valid: bool,
    pub priority: u8,
    pub looping: bool,
    pub reverb: bool,
    pub is_3d: bool,
    pub atten_type: AttenuationType,
    pub atten_min_dist: f32,
    pub atten_max_dist: f32,
    pub atten_rolloff: f32,
    pub position: Vector3,
    pub volume: f32,
    pub pitch: f32,
    pub pan: f32,
    sample: Arc<AudioSample>,
    id: u32,
    voice: Option<i32>,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct WavHeader {
    riff: [u8;4],
    overall_size: u32,
    wave: [u8;4],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct WavHeaderFormat {
    fmt_chunk_marker: [u8;4],
    length_of_fmt: u32,
    format_type: u16,
    channels: u16,
    samplerate: u32,
    byterate: u32,
    block_align: u16,
    bits_per_sample: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct WavChunkHeader {
    id: [u8;4],
    chunk_size: u32,
}

pub struct SoundDriver {
    max_voices: usize,
    voices: [SoundVoice;VOICE_COUNT],
    emitters: Vec<Arc<RwLock<SoundEmitter>>>,
    listener_position: Vector3,
    listener_orientation: Quaternion,
    search_offset: usize,
}

impl SoundDriver {
    /// Construct a new instance of the SoundDriver
    pub fn new(max_voices: usize) -> SoundDriver {
        assert!(max_voices < VOICE_COUNT, "Cannot init sound driver with more than {} voices", VOICE_COUNT);

        let mut driver = SoundDriver {
            max_voices: max_voices,
            voices: [
                SoundVoice {
                    slot: 0,
                    priority: 255,
                    _is_playing: false,
                    id: 0,
                    play_time: 0.0
                }; VOICE_COUNT
            ],
            emitters: Vec::new(),
            listener_position: Vector3::zero(),
            listener_orientation: Quaternion::identity(),
            search_offset: 0,
        };

        for i in 0..VOICE_COUNT {
            driver.voices[i].slot = i.try_into().unwrap();
        }

        return driver;
    }

    fn allocate_voice(voices: &mut [SoundVoice], search_offset: &mut usize, max_voice: usize, priority: u8) -> Option<usize> {
        // a little silly but:
        // voice stealing scheme can sometimes steal voices too early because we have to schedule playback in advance
        // a simple round-robin search offset helps alleviate this

        let mut ret: Option<usize> = None;

        for i in 0..max_voice {
            let idx = (i + *search_offset) % max_voice;
            let voice = &voices[idx];

            if !voice._is_playing && !get_voice_state(voice.slot) {
                ret = Some(idx);
                break;
            } else {
                match ret {
                    Some(r) => {
                        let rv = &voices[r];
                        if voice.play_time < rv.play_time && voice.priority >= priority {
                            ret = Some(idx);
                        }
                    },
                    None => {
                        ret = Some(idx);
                    }
                };
            }
        }

        *search_offset = (*search_offset + 1) % max_voice;

        match ret {
            Some(r) => {
                let rv = &mut voices[r];
                rv.priority = priority;
            },
            None => {
            }
        }

        return ret;
    }
   
    fn assign_hw_voice(listener_position: &Vector3, listener_orientation: &Quaternion, voices: &mut [SoundVoice], search_offset: &mut usize, max_voice: usize, emitter: &mut SoundEmitter) {
        let voice = SoundDriver::allocate_voice(voices, search_offset, max_voice, emitter.priority);

        if voice.is_some() {
            let idx = voice.unwrap();
            let t = get_time();
            voices[idx].play_time = t;
            voices[idx].id += 1;
            emitter.voice = Some(idx.try_into().unwrap());
            emitter.id = voices[idx].id;

            SoundDriver::update_voice(listener_position, listener_orientation, &voices[idx], emitter);
            queue_start_voice(idx.try_into().unwrap(), t);
        }
    }

    fn calc_3d(listener_position: &Vector3, listener_orientation: &Quaternion, position: &Vector3, atten_type: AttenuationType, atten_min_dist: f32, atten_max_dist: f32, atten_rolloff: f32) -> (f32, f32) {
        // calculate gain from distance
        let dist = Vector3::distance(position, listener_position).clamp(atten_min_dist, atten_max_dist);
        let gain = match atten_type {
            AttenuationType::Linear => {
                1.0 - atten_rolloff * (dist - atten_min_dist) / (atten_max_dist - atten_min_dist)
            }
            AttenuationType::InverseDistance => {
                atten_min_dist / (atten_min_dist + atten_rolloff * (dist - atten_min_dist))
            }
            AttenuationType::ExponentialDistance => {
                (dist / atten_min_dist).powf(-atten_rolloff)
            }
            AttenuationType::None => {
                1.0
            }
        };

        // calculate pan
        let mut local_pos = *position - *listener_position;
        let mut rot = *listener_orientation;
        rot.invert();
        local_pos = rot * local_pos;
        local_pos.normalize();

        let pan = local_pos.x;

        return (gain, pan);
    }

    fn update_voice(listener_position: &Vector3, listener_orientation: &Quaternion, voice: &SoundVoice, emitter: &mut SoundEmitter) {
        if emitter.id == voice.id {
            let t = get_time();
            let mut gain = emitter.volume;
            let mut pan = emitter.pan;

            if emitter.is_3d {
                let (gain3d, pan3d) = SoundDriver::calc_3d(listener_position, listener_orientation, &emitter.position, 
                    emitter.atten_type, emitter.atten_min_dist, emitter.atten_max_dist, emitter.atten_rolloff);

                gain *= gain3d;
                pan = pan3d;
            }

            let voice_slot = TryInto::<i32>::try_into(voice.slot).unwrap();
            queue_set_voice_param_i(voice_slot, AudioVoiceParam::SampleData, emitter.sample.handle, t);
            queue_set_voice_param_i(voice_slot, AudioVoiceParam::Samplerate, emitter.sample.samplerate, t);
            queue_set_voice_param_i(voice_slot, AudioVoiceParam::LoopEnabled, if emitter.looping { 1 } else { 0 }, t);
            queue_set_voice_param_i(voice_slot, AudioVoiceParam::LoopStart, 0, t);
            queue_set_voice_param_i(voice_slot, AudioVoiceParam::LoopEnd, 0, t);
            queue_set_voice_param_i(voice_slot, AudioVoiceParam::Reverb, if emitter.reverb { 1 } else { 0 }, t);
            queue_set_voice_param_f(voice_slot, AudioVoiceParam::Volume, gain, t);
            queue_set_voice_param_f(voice_slot, AudioVoiceParam::Detune, 0.0, t);
            queue_set_voice_param_f(voice_slot, AudioVoiceParam::Pitch, emitter.pitch, t);
            queue_set_voice_param_f(voice_slot, AudioVoiceParam::Pan, pan, t);
            queue_set_voice_param_f(voice_slot, AudioVoiceParam::FadeInDuration, 0.0, t);
            queue_set_voice_param_f(voice_slot, AudioVoiceParam::FadeOutDuration, 0.0, t);
        } else {
            // something may have stolen this emitter's voice
            emitter.voice = None;
        }
    }

    /// Update internal sound logic
    pub fn update(&mut self) {
        let emitters = self.emitters.as_mut_slice();
        for emitter_rc in emitters {
            let voice = {
                emitter_rc.read().unwrap().voice
            };
            {
                let mut emref = emitter_rc.write().unwrap();
                match voice {
                    Some(v) => {
                        SoundDriver::update_voice(&self.listener_position, &self.listener_orientation, &self.voices[TryInto::<usize>::try_into(v).unwrap()], &mut emref);
                    },
                    None => {
                        if emref.looping {
                            SoundDriver::assign_hw_voice(&self.listener_position, &self.listener_orientation, &mut self.voices, &mut self.search_offset, self.max_voices, &mut emref);
                        }
                    }
                }
            }
            {
                let mut emref = emitter_rc.write().unwrap();
                // for non-looping sounds: if the voice stops playing, or the sound's voice has been stolen, just stop emitter and remove from list
                if !emref.looping && (!voice.is_some() || !get_voice_state(voice.unwrap().try_into().unwrap())) {
                    emref.is_valid = false;
                }
            }
        }

        // remove any emitters which are no longer valid
        self.emitters.retain(|x| {
            x.read().unwrap().is_valid
        });
    }

    /// Start playing a sound effect and return a handle to it
    pub fn play(&mut self, priority: u8, sample: &Arc<AudioSample>, looping: bool, reverb: bool, volume: f32, pitch: f32, pan: f32) -> Weak<RwLock<SoundEmitter>> {
        let mut emitter = SoundEmitter {
            is_valid: true,
            priority: priority,
            looping: looping,
            reverb: reverb,
            is_3d: false,
            atten_type: AttenuationType::None,
            atten_min_dist: 0.0,
            atten_max_dist: 0.0,
            atten_rolloff: 0.0,
            position: Vector3::zero(),
            volume: volume,
            pitch: pitch,
            pan: pan,
            sample: sample.clone(),
            id: 0,
            voice: None
        };
        SoundDriver::assign_hw_voice(&self.listener_position, &self.listener_orientation, &mut self.voices, &mut self.search_offset, self.max_voices, &mut emitter);
        
        let rc = Arc::new(RwLock::new(emitter));
        let wr = Arc::downgrade(&rc);
        self.emitters.push(rc);

        return wr;
    }

    /// Start playing a 3D sound effect and return a handle to it
    pub fn play_3d(&mut self, priority: u8, sample: &Arc<AudioSample>, looping: bool, reverb: bool, volume: f32, pitch: f32,
        position: Vector3, atten_type: AttenuationType, atten_min_dist: f32, atten_max_dist: f32, atten_rolloff: f32) -> Weak<RwLock<SoundEmitter>> {
        let mut emitter = SoundEmitter {
            is_valid: true,
            priority: priority,
            looping: looping,
            reverb: reverb,
            is_3d: true,
            atten_type: atten_type,
            atten_min_dist: atten_min_dist,
            atten_max_dist: atten_max_dist,
            atten_rolloff: atten_rolloff,
            position: position,
            volume: volume,
            pitch: pitch,
            pan: 0.0,
            sample: sample.clone(),
            id: 0,
            voice: None
        };
        SoundDriver::assign_hw_voice(&self.listener_position, &self.listener_orientation, &mut self.voices, &mut self.search_offset, self.max_voices, &mut emitter);
        
        let rc = Arc::new(RwLock::new(emitter));
        let wr = Arc::downgrade(&rc);
        self.emitters.push(rc);

        return wr;
    }

    /// Stop the playing emitter
    pub fn stop(&mut self, emitter_ref: Weak<RefCell<SoundEmitter>>) {
        let rc = emitter_ref.upgrade();
        if !rc.is_some() {
            return;
        }

        let em = rc.unwrap();
        let mut emitter = em.borrow_mut();

        if !emitter.is_valid { return; }

        if emitter.voice.is_some() {
            let voiceid = TryInto::<usize>::try_into(emitter.voice.unwrap()).unwrap();
            let voice: &mut SoundVoice = &mut self.voices[voiceid];
            if voice.id == emitter.id {
                voice.priority = 255;
                queue_stop_voice(voice.slot.try_into().unwrap(), 0.0);
            }
        }

        emitter.is_valid = false;
    }
}

/// Load a wav file, returning an audio sample handle (supported encodings are unsigned 8-bit, signed 16-bit, and IMA ADPCM)
pub fn load_wav(file: &mut FileStream) -> Result<AudioSample,()> {
    let header = match file.read_element::<WavHeader>() {
        Ok(v) => { v },
        Err(_) => { return Err(()); }
    };

    // check riff string
    let riff = match std::str::from_utf8(&header.riff) {
        Ok(v) => { v },
        Err(_) => { return Err(()); }
    };

    if riff != "RIFF" {
        return Err(());
    }

    let fmt_header = match file.read_element::<WavHeaderFormat>() {
        Ok(v) => { v },
        Err(_) => { return Err(()); }
    };

    // check fmt string

    let fmt_str = match std::str::from_utf8(&fmt_header.fmt_chunk_marker) {
        Ok(v) => { v },
        Err(_) => { return Err(()); }
    };

    if fmt_str != "fmt " {
        return Err(());
    }

    // skip over header data
    let fmt_header_size: usize = fmt_header.length_of_fmt.try_into().unwrap();
    let header_size: usize = size_of::<WavHeader>() + fmt_header_size + 8;

    match file.seek(header_size.try_into().unwrap(), SeekOrigin::Begin) {
        Ok(_) => {  },
        Err(_) => { return Err(()); }
    }

    let mut data_found = false;
    let mut chunk_header: WavChunkHeader = WavChunkHeader { id: [0;4], chunk_size: 0 };

    while !file.end_of_file() {
        chunk_header = match file.read_element::<WavChunkHeader>() {
            Ok(v) => { v },
            Err(_) => { return Err(()); }
        };

        let chunk_id = match std::str::from_utf8(&chunk_header.id) {
            Ok(v) => { v },
            Err(_) => { return Err(()); }
        };

        if chunk_id == "data" {
            data_found = true;
            break;
        } else {
            // skip chunk data
            _ = file.seek(chunk_header.chunk_size.try_into().unwrap(), SeekOrigin::Begin);
        }
    }

    if !data_found {
        return Err(());
    }

    if fmt_header.format_type == 1 && fmt_header.bits_per_sample == 8 {
        // unsigned 8-bit PCM
        let mut pcm8: Vec<u8> = vec![0;chunk_header.chunk_size.try_into().unwrap()];
        match file.read(pcm8.as_mut_slice()) {
            Ok(_) => {},
            Err(_) => { return Err(()); }
        };

        // convert from unsigned 0 .. 255 to signed -128 .. 127
        for i in 0..pcm8.len() {
            pcm8[i] = pcm8[i].wrapping_sub(128);
        }

        let sample_handle = unsafe { AudioSample::create_s8(transmute(pcm8.as_slice()), 
            fmt_header.samplerate.try_into().unwrap())? };

        return Ok(sample_handle);
    } else if fmt_header.format_type == 1 && fmt_header.bits_per_sample == 16 {
        // signed 16-bit PCM
        let mut pcm16: Vec<u8> = vec![0, chunk_header.chunk_size.try_into().unwrap()];
        match file.read(pcm16.as_mut_slice()) {
            Ok(_) => {},
            Err(_) => { return Err(()); }
        };

        let sample_handle = unsafe { AudioSample::create_s16(transmute(pcm16.as_slice()), 
            fmt_header.samplerate.try_into().unwrap())? };
        
        return Ok(sample_handle);
    } else if fmt_header.format_type == 0x11 {
        // IMA ADPCM
        let mut adpcm: Vec<u8> = vec![0, chunk_header.chunk_size.try_into().unwrap()];
        match file.read(adpcm.as_mut_slice()) {
            Ok(_) => {},
            Err(_) => { return Err(()); }
        }

        let sample_handle = AudioSample::create_adpcm(adpcm.as_slice(), 
            fmt_header.block_align.try_into().unwrap(), 
            fmt_header.samplerate.try_into().unwrap())?;

        return Ok(sample_handle);
    }

    return Err(());
}