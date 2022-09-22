use std::{convert::TryInto};

use crate::db_internal::{audio_alloc, audio_allocCompressed, audio_free, audio_getUsage, audio_queueSetParam_i, audio_queueSetParam_f, audio_queueStartVoice, audio_queueStopVoice, audio_getVoiceState, audio_getTime, audio_setReverbParams, audio_initSynth, audio_playMidi, audio_setMidiReverb, audio_setMidiVolume};

pub const VOICE_COUNT: usize = 32;

#[repr(C)]
#[derive(Clone)]
pub struct AudioSample {
    pub handle: i32,
    pub samplerate: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum AudioVoiceParam {
    Volume,
    Pitch,
    Detune,
    Pan,
    SampleData,
    Samplerate,
    LoopEnabled,
    LoopStart,
    LoopEnd,
    Reverb,
    FadeInDuration,
    FadeOutDuration,
    Start,
    Stop,
}

impl AudioSample {
    /// Create a new signed 8-bit PCM audio sample
    pub fn create_s8(pcm_data: &[i8], samplerate: i32) -> Result<AudioSample,()> {
        let handle = unsafe { audio_alloc(pcm_data.as_ptr().cast(), pcm_data.len().try_into().unwrap(), 0) };
        if handle == -1 {
            return Err(());
        }

        return Ok(AudioSample {
            handle: handle,
            samplerate: samplerate
        });
    }

    /// Create a new signed 16-bit PCM audio sample
    pub fn create_s16(pcm_data: &[i16], samplerate: i32) -> Result<AudioSample,()> {
        let handle = unsafe { audio_alloc(pcm_data.as_ptr().cast(), (pcm_data.len() * 2).try_into().unwrap(), 1) };
        if handle == -1 {
            return Err(());
        }

        return Ok(AudioSample {
            handle: handle,
            samplerate: samplerate
        });
    }

    /// Create a new IMA ADPCM encoded audio sample
    pub fn create_adpcm(adpcm_data: &[u8], chunk_size: i32, samplerate: i32) -> Result<AudioSample,()> {
        let handle = unsafe { audio_allocCompressed(adpcm_data.as_ptr().cast(), adpcm_data.len().try_into().unwrap(), chunk_size) };
        if handle == -1 {
            return Err(());
        }

        return Ok(AudioSample {
            handle: handle,
            samplerate: samplerate
        });
    }
}

impl Drop for AudioSample {
    fn drop(&mut self) {
        unsafe { audio_free(self.handle); }
    }
}

/// Get the current sample memory usage in bytes
pub fn get_usage() -> i32 {
    unsafe { return audio_getUsage(); }
}

/// Schedule an audio voice integer parameter change at some point in the future
pub fn queue_set_voice_param_i(slot: i32, param: AudioVoiceParam, value: i32, time: f64) {
    assert!(slot >= 0 && slot < VOICE_COUNT.try_into().unwrap(), "Tried to set parameter for invalid voice handle");
    unsafe { audio_queueSetParam_i(slot, param, value, time); }
}

/// Schedule an audio voice float parameter change at some point in the future
pub fn queue_set_voice_param_f(slot: i32, param: AudioVoiceParam, value: f32, time: f64) {
    assert!(slot >= 0 && slot < VOICE_COUNT.try_into().unwrap(), "Tried to set parameter for invalid voice handle");
    unsafe { audio_queueSetParam_f(slot, param, value, time); }
}

/// Schedule an audio voice to start playing at some point in the future
pub fn queue_start_voice(slot: i32, time: f64) {
    assert!(slot >= 0 && slot < VOICE_COUNT.try_into().unwrap(), "Tried to start invalid voice handle");
    unsafe { audio_queueStartVoice(slot, time) };
}

/// Schedule an audio voice to stop playing at some point in the future
pub fn queue_stop_voice(slot: i32, time: f64) {
    assert!(slot >= 0 && slot < VOICE_COUNT.try_into().unwrap(), "Tried to stop invalid voice handle");
    unsafe { audio_queueStopVoice(slot, time) };
}

/// Gets whether the given voice is currently playing
pub fn get_voice_state(slot: i32) -> bool {
    assert!(slot >= 0 && slot < VOICE_COUNT.try_into().unwrap(), "Tried to get state of invalid voice handle");
    unsafe { return audio_getVoiceState(slot) };
}

/// Get the current audio timer value for scheduling
pub fn get_time() -> f64 {
    unsafe { return audio_getTime(); }
}

/// Set the current reverb unit parameters
pub fn set_reverb(room_size: f32, damping: f32, width: f32, wet: f32, dry: f32) {
    unsafe { audio_setReverbParams(room_size, damping, width, wet, dry); }
}

/// Initialize the MIDI synth using the given soundfont data
pub fn init_synth(sf2_data: &[u8]) -> Result<(),()> {
    unsafe {
        let result = audio_initSynth(sf2_data.as_ptr(), sf2_data.len().try_into().unwrap());
        if result {
            return Ok(());
        } else {
            return Err(());
        }
    }
}

/// Start playing a MIDI file using the initialized MIDI synth, optionally looping the file when it reaches the end
pub fn play_midi(midi_data: &[u8], looping: bool) -> Result<(),()> {
    unsafe {
        let result = audio_playMidi(midi_data.as_ptr(), midi_data.len().try_into().unwrap(), looping);
        if result {
            return Ok(());
        } else {
            return Err(());
        }
    }
}

/// Set whether to route MIDI playback through the reverb unit
pub fn set_midi_reverb(enabled: bool) {
    unsafe { audio_setMidiReverb(enabled); }
}

/// Set the volume of MIDI playback
pub fn set_midi_volume(volume: f32) {
    unsafe { audio_setMidiVolume(volume); }
}