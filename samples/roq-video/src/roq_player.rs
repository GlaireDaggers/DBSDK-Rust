use std::{io::{Read, Error, Seek}, convert::TryInto, marker::PhantomData};
use byteorder::{ReadBytesExt, LittleEndian};
use dbsdk_rs::db::log;

const ROQ_SIGNATURE: u16 = 0x1084;

lazy_static! {
    static ref SND_SQR_LUT: [i16;256] = {
        let mut buf: [i16;256] = [0;256];
        for i in 0..128 {
            let si: i16 = i.try_into().unwrap();
            buf[i] = si * si;
            buf[i + 128] = -(si * si);
        }
        buf
    };
}

pub trait Colorspace<TColor>: where TColor : Sized + Clone + Copy {
    fn default() -> TColor;
    fn convert(y: i32, cb: i32, cr: i32, a: u8) -> TColor;
}

/// Represents a 16 bit-per-pixel 3 channel RGB color format
pub struct ColorspaceBgr565 {
}

impl Colorspace<u16> for ColorspaceBgr565 {
    fn default() -> u16 {
        return 0;
    }

    fn convert(y: i32, cb: i32, cr: i32, _a: u8) -> u16 {
        let yp = ((y - 16) as f32) * 1.164;
        let mut r = (yp + 1.596 * ((cr - 128) as f32)) / 8.0;
        let mut g = (yp - 0.813 * ((cr - 128) as f32) - 0.391 * ((cb - 128) as f32)) / 4.0;
        let mut b = (yp + 2.018 * ((cb - 128) as f32)) / 8.0;
        if r < 0.0 { r = 0.0; }
        if r > 31.0 { r = 31.0; }
        if g < 0.0 { g = 0.0; }
        if g > 63.0 { g = 63.0; }
        if b < 0.0 { b = 0.0; }
        if b > 31.0 { b = 31.0; }

        let r5 = r as u16;
        let g6 = g as u16;
        let b5 = b as u16;
        
        return (r5 << 11) | (g6 << 5) | (b5);
    }
}

/// Represents a 32 bit-per-pixel 4 channel RGBA color format
pub struct ColorspaceRgba8888 {
}

impl Colorspace<u32> for ColorspaceRgba8888 {
    fn default() -> u32 {
        return 0;
    }

    fn convert(y: i32, cb: i32, cr: i32, a: u8) -> u32 {
        let yp = ((y - 16) as f32) * 1.164;
        let mut r = yp + 1.596 * ((cr - 128) as f32);
        let mut g = yp - 0.813 * ((cr - 128) as f32) - 0.391 * ((cb - 128) as f32);
        let mut b = yp + 2.018 * ((cb - 128) as f32);
        if r < 0.0 { r = 0.0; }
        if r > 255.0 { r = 255.0; }
        if g < 0.0 { g = 0.0; }
        if g > 255.0 { g = 255.0; }
        if b < 0.0 { b = 0.0; }
        if b > 255.0 { b = 255.0; }

        let r32 = r as u32;
        let g32 = g as u32;
        let b32 = b as u32;
        let a32 = a as u32;
        
        return (r32) | (g32 << 8) | (b32 << 16) | (a32 << 24);
    }
}

#[derive(Debug)]
pub enum RoqError {
    IOError(Error),
    ParseError,
}

pub enum RoqEvent<'a, C> where C:Sized+Clone+Copy {
    InitVideo,
    Audio (i32, &'a[i16]),
    Video (&'a[C]),
    EndOfFile,
}

pub struct RoqPlayer<R,C,S>
    where R : Read + Seek, C:Sized + Clone + Copy, S : Colorspace<C> {
    pub framerate: u16,
    pub width: u16,
    pub height: u16,
    alpha: bool,
    cb2x2: [[C;4];256],
    cb4x4: [[C;16];256],
    framebuf: [Vec<C>;2],
    audiobuf: Vec<i16>,
    cur_frame: usize,
    next_frame: usize,
    roqstream: R,
    phantom: PhantomData<S>,
}

impl<R,C,S> RoqPlayer<R,C,S> where R : Read + Seek, C:Sized + Clone + Copy, S : Colorspace<C> {
    pub fn new(mut reader: R) -> Result<RoqPlayer<R,C,S>,RoqError> {
        // read ROQ header
        let chunk_id = match reader.read_u16::<LittleEndian>() {
            Ok(v) => { v },
            Err(e) => { return Err(RoqError::IOError(e)); }
        };
        let chunk_size = match reader.read_u32::<LittleEndian>() {
            Ok(v) => { v },
            Err(e) => { return Err(RoqError::IOError(e)); }
        };
        if chunk_id != ROQ_SIGNATURE && chunk_size != 0xFFFFFFFF {
            return Err(RoqError::ParseError);
        }

        let framerate = match reader.read_u16::<LittleEndian>() {
            Ok(v) => { v },
            Err(e) => { return Err(RoqError::IOError(e)); }
        };

        let roq_player = RoqPlayer {
            framerate,
            roqstream: reader,
            width: 0,
            height: 0,
            cb2x2: [[S::default();4];256],
            cb4x4: [[S::default();16];256],
            framebuf: [Vec::new(), Vec::new()],
            audiobuf: Vec::new(),
            alpha: false,
            cur_frame: 0,
            next_frame: 0,
            phantom: PhantomData,
        };
        return Ok(roq_player);
    }

    /// Retrieve the current framebuffer for display
    pub fn get_framebuffer_data(&self) -> &[C] {
        return &self.framebuf[self.cur_frame & 1].as_slice();
    }

    fn unpack_codebook<RD>(&mut self, reader: &mut RD, chunk_size: u32, chunk_arg: u16) -> Result<(),RoqError> where RD : Read {
        // chunkarg specifies how many vector cells make up the codebook
        let mut cell_cnt_2x2: usize = ((chunk_arg >> 8) & 0xFF).try_into().unwrap();
        let mut cell_cnt_4x4: usize = (chunk_arg & 0xFF).try_into().unwrap();

        if cell_cnt_2x2 == 0 {
            cell_cnt_2x2 = 256;
        }

        if cell_cnt_4x4 == 0 && (cell_cnt_2x2 * 6) < chunk_size.try_into().unwrap() {
            cell_cnt_4x4 = 256;
        }

        // unpack 2x2 vectors
        for i in 0..cell_cnt_2x2 {
            // unpack YCbCr components from byte stream
            let mut y: [u8;4] = [0;4];
            let mut a: [u8;4] = [255;4];

            if self.alpha {
                for j in 0..4 {
                    y[j] = match reader.read_u8() {
                        Ok (v) => { v },
                        Err(e) => {
                            return Err(RoqError::IOError(e));
                        }
                    };
                    a[j] = match reader.read_u8() {
                        Ok (v) => { v },
                        Err(e) => {
                            return Err(RoqError::IOError(e));
                        }
                    };
                }
            } else {
                match reader.read_exact(&mut y) {
                    Ok(_) => {},
                    Err(e) => {
                        return Err(RoqError::IOError(e));
                    }
                }
            }

            let cb = match reader.read_u8() {
                Ok(v) => { v },
                Err(e) => {
                    return Err(RoqError::IOError(e));
                }
            };

            let cr = match reader.read_u8() {
                Ok(v) => { v },
                Err(e) => {
                    return Err(RoqError::IOError(e));
                }
            };

            // convert to color space
            for j in 0..4 {
                self.cb2x2[i][j] = S::convert(y[j] as i32, cb as i32, cr as i32, a[j]);
            }
        }

        // unpack 4x4 vectors
        for i in 0..cell_cnt_4x4 {
            for j in 0..4 {
                let vi = match reader.read_u8() {
                    Ok(v) => { v },
                    Err(e) => {
                        return Err(RoqError::IOError(e));
                    }
                };

                let v4x4_offs = (j / 2) * 8 + (j % 2) * 2;

                let v2x2 = &self.cb2x2[vi as usize];
                let v4x4 = &mut self.cb4x4[i][v4x4_offs..];
                v4x4[0] = v2x2[0];
                v4x4[1] = v2x2[1];
                v4x4[4] = v2x2[2];
                v4x4[5] = v2x2[3];
            }
        }

        return Ok(());
    }

    fn get_mode<RD>(&mut self, reader: &mut RD, mode_count: &mut u16, mode_set: &mut u16) -> Result<u16,RoqError> where RD : Read {
        if *mode_count == 0 {
            let lo = match reader.read_u8() {
                Ok(v) => { v },
                Err(e) => {
                    return Err(RoqError::IOError(e));
                }
            } as u16;
            let hi = match reader.read_u8() {
                Ok(v) => { v },
                Err(e) => {
                    return Err(RoqError::IOError(e));
                }
            } as u16;
            *mode_set = (hi << 8) | lo;
            *mode_count = 16;
        }

        *mode_count -= 2;
        return Ok((*mode_set >> *mode_count) & 0x03);
    }

    // unpack VQ data into a frame for display
    fn unpack_vq<RD>(&mut self, reader: &mut RD, _chunk_size: u32, chunk_arg: u16) -> Result<(), RoqError> where RD : Read {
        self.cur_frame = self.next_frame;
        let prev_fb_idx = (self.cur_frame + 1) & 1;
        let cur_fb_idx = self.cur_frame & 1;
        self.next_frame += 1;

        // special case for frame 1, which needs to begin with frame 0's data
        if self.cur_frame == 1 {
            for i in 0..self.framebuf[0].len() {
                self.framebuf[cur_fb_idx][i] = self.framebuf[prev_fb_idx][i];
            }
        }

        let mb_width = (self.width as usize) / 16;
        let mb_height = (self.height as usize) / 16;

        let stride = self.width as i32;

        let mut mode_count: u16 = 0;
        let mut mode_set: u16 = 0;

        let mx = ((chunk_arg >> 8) as i8) as i32;
        let my = (chunk_arg as i8) as i32;

        for mb_y in 0..mb_height {
            let lineoffset = mb_y * 16 * (stride as usize);
            for mb_x in 0..mb_width {
                let mb_offset = lineoffset + mb_x * 16;
                for block in 0..4 {
                    let block_offset = mb_offset + (block / 2 * 8 * (stride as usize)) + (block % 2 * 8);
                    match self.get_mode(reader, &mut mode_count, &mut mode_set)? {
                        0 => {
                            // MOT: skip
                        },
                        1 => {
                            // FCC: motion compensation
                            let data_byte = match reader.read_u8() {
                                Ok(v) => { v },
                                Err(e) => { return Err(RoqError::IOError(e)); }
                            };

                            let motion_x = 8 - ((data_byte >> 4) as i32) - mx;
                            let motion_y = 8 - ((data_byte & 0xF) as i32) - my;
                            let mut prev_frame_idx = (block_offset as i32) +
                                (motion_y * stride) +
                                (motion_x);
                            let mut cur_frame_idx = block_offset as i32;

                            // check against frame bounds
                            let posx = ((mb_x * 16) + (block % 2 * 8)) as i32;
                            let posy = (mb_y * 16 + (block / 2 * 8)) as i32;

                            let dposx = posx + motion_x;
                            let dposy = posy + motion_y;

                            if dposx < 0 || dposx > (self.width - 8) as i32 ||
                                dposy < 0 || dposy > (self.height - 8) as i32 {
                                log(format!("Motion vector out of bounds: Pos = ({}, {}), MV = ({}, {}), Bounds = ({}, {})",
                                    posx, posy,
                                    dposx, dposy,
                                    self.width, self.height).as_str());
                            } else {
                                // copy 8x8 pixel block from previous frame
                                for _ in 0..8 {
                                    for j in 0..8 {
                                        self.framebuf[cur_fb_idx][(cur_frame_idx + j) as usize] = self.framebuf[prev_fb_idx][(prev_frame_idx + j) as usize];
                                    }

                                    // move to next line
                                    prev_frame_idx += stride;
                                    cur_frame_idx += stride;
                                }
                            }
                        },
                        2 => {
                            // SDL: upsample 4x4 vector
                            let data_byte = match reader.read_u8() {
                                Ok(v) => { v },
                                Err(e) => { return Err(RoqError::IOError(e)); }
                            } as usize;
                            for i in 0..16 {
                                let this_ptr = block_offset + (i / 4 * 2 * (stride as usize)) + (i % 4 * 2);
                                self.framebuf[cur_fb_idx][this_ptr] = self.cb4x4[data_byte][i];
                                self.framebuf[cur_fb_idx][this_ptr + 1] = self.cb4x4[data_byte][i];
                                self.framebuf[cur_fb_idx][this_ptr + (stride as usize)] = self.cb4x4[data_byte][i];
                                self.framebuf[cur_fb_idx][this_ptr + 1 + (stride as usize)] = self.cb4x4[data_byte][i];
                            }
                        },
                        3 => {
                            // CCC: subdivide into 4 subblocks
                            for subblock in 0..4 {
                                let subblock_offset = block_offset + (subblock / 2 * 4 * (stride as usize)) +
                                    (subblock % 2 * 4);

                                match self.get_mode(reader, &mut mode_count, &mut mode_set)? {
                                    0 => {
                                        // MOT: skip
                                    },
                                    1 => {
                                        // FCC: motion compensation
                                        let data_byte = match reader.read_u8() {
                                            Ok(v) => { v },
                                            Err(e) => { return Err(RoqError::IOError(e)); }
                                        } as i32;

                                        let motion_x = 8 - (data_byte >> 4) - mx;
                                        let motion_y = 8 - (data_byte & 0xF) - my;
                                        let mut prev_frame_idx = (subblock_offset as i32) +
                                            (motion_y * stride) +
                                            (motion_x);
                                        let mut cur_frame_idx = subblock_offset as i32;

                                        // check against frame bounds
                                        let posx = ((mb_x * 16) + (block % 2 * 8) + (subblock % 2 * 4)) as i32;
                                        let posy = (mb_y * 16 + (block / 2 * 8) + (subblock / 2 * 4)) as i32;

                                        let dposx = posx + motion_x;
                                        let dposy = posy + motion_y;

                                        if dposx < 0 || dposx > (self.width - 4) as i32 ||
                                            dposy < 0 || dposy > (self.height - 4) as i32 {
                                            log(format!("Motion vector out of bounds: Pos = ({}, {}), MV = ({}, {}), Bounds = ({}, {})",
                                                posx, posy,
                                                dposx, dposy,
                                                self.width, self.height).as_str());
                                        } else {
                                            // copy 4x4 pixel block from previous frame
                                            for _ in 0..4 {
                                                for j in 0..4 {
                                                    self.framebuf[cur_fb_idx][(cur_frame_idx + j) as usize] = self.framebuf[prev_fb_idx][(prev_frame_idx + j) as usize];
                                                }

                                                // move to next line
                                                prev_frame_idx += stride;
                                                cur_frame_idx += stride;
                                            }
                                        }
                                    },
                                    2 => {
                                        // SDL: use 4x4 vector from codebook
                                        let data_byte = match reader.read_u8() {
                                            Ok(v) => { v },
                                            Err(e) => { return Err(RoqError::IOError(e)); }
                                        } as usize;
                                        let mut this_ptr = subblock_offset;
                                        let mut vec_ptr = 0;
                                        for _ in 0..4 {
                                            self.framebuf[cur_fb_idx][this_ptr] = self.cb4x4[data_byte][vec_ptr];
                                            self.framebuf[cur_fb_idx][this_ptr + 1] = self.cb4x4[data_byte][vec_ptr + 1];
                                            self.framebuf[cur_fb_idx][this_ptr + 2] = self.cb4x4[data_byte][vec_ptr + 2];
                                            self.framebuf[cur_fb_idx][this_ptr + 3] = self.cb4x4[data_byte][vec_ptr + 3];
                                            vec_ptr += 4;

                                            // move to next line
                                            this_ptr += stride as usize;
                                        }
                                    },
                                    3 => {
                                        // CCC: subdivide into 4 subblocks
                                        // at this point we just read four 2x2 codebooks into this 4x4 block
                                        let mut this_ptr = subblock_offset;
                                        {
                                            let data_byte = match reader.read_u8() {
                                                Ok(v) => { v },
                                                Err(e) => { return Err(RoqError::IOError(e)); }
                                            } as usize;

                                            self.framebuf[cur_fb_idx][this_ptr] = self.cb2x2[data_byte][0];
                                            self.framebuf[cur_fb_idx][this_ptr + 1] = self.cb2x2[data_byte][1];
                                            self.framebuf[cur_fb_idx][this_ptr + (stride as usize)] = self.cb2x2[data_byte][2];
                                            self.framebuf[cur_fb_idx][this_ptr + 1 + (stride as usize)] = self.cb2x2[data_byte][3];
                                        }
                                        {
                                            let data_byte = match reader.read_u8() {
                                                Ok(v) => { v },
                                                Err(e) => { return Err(RoqError::IOError(e)); }
                                            } as usize;

                                            self.framebuf[cur_fb_idx][this_ptr + 2] = self.cb2x2[data_byte][0];
                                            self.framebuf[cur_fb_idx][this_ptr + 3] = self.cb2x2[data_byte][1];
                                            self.framebuf[cur_fb_idx][this_ptr + 2 + (stride as usize)] = self.cb2x2[data_byte][2];
                                            self.framebuf[cur_fb_idx][this_ptr + 3 + (stride as usize)] = self.cb2x2[data_byte][3];
                                        }

                                        this_ptr += (stride as usize) * 2;

                                        {
                                            let data_byte = match reader.read_u8() {
                                                Ok(v) => { v },
                                                Err(e) => { return Err(RoqError::IOError(e)); }
                                            } as usize;

                                            self.framebuf[cur_fb_idx][this_ptr] = self.cb2x2[data_byte][0];
                                            self.framebuf[cur_fb_idx][this_ptr + 1] = self.cb2x2[data_byte][1];
                                            self.framebuf[cur_fb_idx][this_ptr + (stride as usize)] = self.cb2x2[data_byte][2];
                                            self.framebuf[cur_fb_idx][this_ptr + 1 + (stride as usize)] = self.cb2x2[data_byte][3];
                                        }
                                        {
                                            let data_byte = match reader.read_u8() {
                                                Ok(v) => { v },
                                                Err(e) => { return Err(RoqError::IOError(e)); }
                                            } as usize;

                                            self.framebuf[cur_fb_idx][this_ptr + 2] = self.cb2x2[data_byte][0];
                                            self.framebuf[cur_fb_idx][this_ptr + 3] = self.cb2x2[data_byte][1];
                                            self.framebuf[cur_fb_idx][this_ptr + 2 + (stride as usize)] = self.cb2x2[data_byte][2];
                                            self.framebuf[cur_fb_idx][this_ptr + 3 + (stride as usize)] = self.cb2x2[data_byte][3];
                                        }
                                    },
                                    _ => {}
                                }
                            }
                        },
                        _ => {
                        }
                    }
                }
            }
        }

        return Ok(());
    }

    /// Read the next data chunk from the ROQ stream
    pub fn read_next(&mut self) -> Result<RoqEvent<C>,RoqError> {
        loop {
            // read chunk header
            let chunk_id = match self.roqstream.read_u16::<LittleEndian>() {
                Ok(v) => { v },
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        break;
                    } else {
                        return Err(RoqError::IOError(e));
                    }
                }
            };
            
            let chunk_size = match self.roqstream.read_u32::<LittleEndian>() {
                Ok(v) => { v },
                Err(e) => { return Err(RoqError::IOError(e)); }
            };

            let chunk_arg = match self.roqstream.read_u16::<LittleEndian>() {
                Ok(v) => { v },
                Err(e) => { return Err(RoqError::IOError(e)); }
            };

            let mut chunk_data: Vec<u8> = vec![0; chunk_size as usize];
            self.roqstream.read_exact(chunk_data.as_mut_slice()).expect("ERROR");

            let mut chunk_reader = chunk_data.as_slice();

            match chunk_id {
                0x1001 => {
                    // roq info chunk
                    let vwidth =  match chunk_reader.read_u16::<LittleEndian>() {
                        Ok(v) => { v },
                        Err(e) => { return Err(RoqError::IOError(e)); }
                    };
                    let vheight =  match chunk_reader.read_u16::<LittleEndian>() {
                        Ok(v) => { v },
                        Err(e) => { return Err(RoqError::IOError(e)); }
                    };

                    self.width = vwidth;
                    self.height = vheight;
                    self.alpha = chunk_arg != 0;

                    // ensure capacity
                    let fbsize: usize = (self.width as usize) * (self.height as usize);
                    self.framebuf = [vec![S::default();fbsize], vec![S::default();fbsize]];

                    return Ok(RoqEvent::InitVideo);
                },
                0x1002 => {
                    // roq quad codebook chunk
                    // updates codebook vectors
                    self.unpack_codebook(&mut chunk_reader, chunk_size, chunk_arg)?;
                },
                0x1011 => {
                    // roq quad vq chunk
                    // this unpacks a frame and returns a Video chunk w/ the frame data
                    self.unpack_vq(&mut chunk_reader, chunk_size, chunk_arg)?;
                    return Ok(RoqEvent::Video (self.get_framebuffer_data()));
                },
                0x1020 => {
                    // roq mono sound chunk
                    
                    // ensure audio buffer has enough room to fit
                    let sample_cnt = chunk_size as usize;
                    self.audiobuf.resize(sample_cnt, 0);

                    // decode DPCM
                    let mut snd = chunk_arg as i16;
                    for i in 0..sample_cnt {
                        snd += SND_SQR_LUT[chunk_reader[i] as usize];
                        self.audiobuf[i] = snd;
                    }

                    return Ok(RoqEvent::Audio(1, &self.audiobuf.as_slice()));
                },
                0x1021 => {
                    // roq stereo sound chunk
                    
                    // ensure audio buffer has enough room to fit
                    let sample_cnt = (chunk_size / 2) as usize;
                    self.audiobuf.resize(sample_cnt * 2, 0);

                    // decode DPCM
                    let mut snd_left = (chunk_arg & 0xFF00) as i16;
                    let mut snd_right = ((chunk_arg & 0xFF) << 8) as i16;
                    for i in 0..sample_cnt {
                        snd_left += SND_SQR_LUT[chunk_reader[i * 2] as usize];
                        snd_right += SND_SQR_LUT[chunk_reader[i * 2 + 1] as usize];
                        self.audiobuf[i * 2] = snd_left;
                        self.audiobuf[i * 2 + 1] = snd_right;
                    }

                    return Ok(RoqEvent::Audio(2, &self.audiobuf.as_slice()));
                },
                _ => {
                    // some other chunk type - ignore
                    log(format!("ROQ: unhandled chunk (id: {}, size: {}, arg: {})", chunk_id, chunk_size, chunk_arg).as_str());
                }
            }
        }

        return Ok(RoqEvent::EndOfFile);
    }
}