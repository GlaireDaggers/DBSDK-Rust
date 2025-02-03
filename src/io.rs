use std::{ffi::{CString, CStr}, convert::TryInto};

use crate::{db_internal::{fs_close, fs_open, fs_read, fs_write, fs_seek, fs_tell, fs_eof, fs_deviceExists, fs_deviceEject, fs_fileExists, fs_closeDir, fs_openDir, fs_readDir, clock_timestampToDatetime, fs_rewindDir, fs_allocMemoryCard}, clock::DateTime};

const ESUCCESS: i32 = 0;
const EACCESS: i32 = 2;
const EEXIST: i32 = 20;
const EFBIG: i32 = 22;
const ENFILE: i32 = 41;
const ENODEV: i32 = 43;
const ENOENT: i32 = 44;
const ENOSPC: i32 = 51;
const EROFS: i32 = 69;
const ESPIPE: i32 = 70;

#[repr(C)]
#[derive(Clone, Copy)]
pub enum FileMode {
    Read,
    Write
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum SeekOrigin {
    Begin,
    Current,
    End,
}

#[derive(Debug)]
pub enum IOError {
    TooManyFilesOpen,
    ReadOnlyFileSystem,
    FileNotFound,
    DirectoryNotFound,
    NoSuchDevice,
    NotSupported,
    InvalidSeek,
    FileTooBig,
    FileAlreadyExists,
    NoSpaceOnDevice,
    ReachedEndOfFile
}

pub struct FileStream {
    handle: i32,
}

impl std::io::Read for FileStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        unsafe {
            let result = fs_read(self.handle, buf.as_mut_ptr().cast(), buf.len().try_into().unwrap());

            match crate::db_internal::ERRNO {
                ESUCCESS => {
                }
                EACCESS => {
                    return Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
                }
                _ => {
                    panic!("Unhandled errno");
                }
            }

            return Ok(result.try_into().unwrap());
        }
    }
}

impl std::io::Write for FileStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        unsafe {
            let result = fs_write(self.handle, buf.as_ptr().cast(), buf.len().try_into().unwrap());

            match crate::db_internal::ERRNO {
                ESUCCESS => {
                }
                EACCESS => {
                    return Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
                }
                EFBIG => {
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, "File size limit reached"));
                }
                _ => {
                    panic!("Unhandled errno");
                }
            }

            return Ok(result.try_into().unwrap());
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        unsafe {
            crate::db_internal::fs_flush(self.handle);

            match crate::db_internal::ERRNO {
                ESUCCESS => {
                    return Ok(());
                },
                EACCESS => {
                    return Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
                },
                _ => {
                    panic!("Unhandled errno");
                }
            }
        }
    }
}

impl std::io::Seek for FileStream {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        unsafe {
            let result = match pos {
                std::io::SeekFrom::Start(position) => {
                    fs_seek(self.handle, position.try_into().unwrap(), SeekOrigin::Begin)
                },
                std::io::SeekFrom::Current(position) => {
                    fs_seek(self.handle, position.try_into().unwrap(), SeekOrigin::Current)
                },
                std::io::SeekFrom::End(position) => {
                    fs_seek(self.handle, position.try_into().unwrap(), SeekOrigin::End)
                }
            };

            match crate::db_internal::ERRNO {
                ESUCCESS => {
                }
                ESPIPE => {
                    return Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
                }
                _ => {
                    panic!("Unhandled errno");
                }
            }

            return Ok(result.try_into().unwrap());
        }
    }
}

impl FileStream {
    /// Open a file from the filesystem (paths are given in the form of "/\[device\]/path/to/file") <br/>
    /// Valid devices are "cd", "ma", and "mb"
    pub fn open(path: &str, mode: FileMode) -> Result<FileStream, IOError> {
        unsafe {
            let path_cstr = CString::new(path).expect("Failed creating C string");
            let handle = fs_open(path_cstr.as_ptr(), mode);

            if handle == 0 {
                match crate::db_internal::ERRNO {
                    ENFILE => {
                        return Err(IOError::TooManyFilesOpen);
                    }
                    ENOENT => {
                        return Err(IOError::FileNotFound);
                    }
                    EROFS => {
                        return Err(IOError::ReadOnlyFileSystem);
                    }
                    ENODEV => {
                        return Err(IOError::NoSuchDevice);
                    }
                    _ => {
                        panic!("Unhandled errno");
                    }
                }
            }

            return Ok(FileStream {
                handle: handle
            });
        }
    }

    /// Allocate a new file on the memory card device given in the path string of the given size in 512-byte blocks for writing
    pub fn allocate_memory_card(path: &str, icondata: &[u8;128], iconpalette: &[u16;16], blocks: i32) -> Result<FileStream, IOError> {
        unsafe {
            let path_cstr = CString::new(path).expect("Failed creating C string");
            let handle = fs_allocMemoryCard(path_cstr.as_ptr(), icondata.as_ptr(), iconpalette.as_ptr(), blocks);

            if handle == 0 {
                match crate::db_internal::ERRNO {
                    EEXIST => {
                        return Err(IOError::FileAlreadyExists);
                    }
                    ENOSPC => {
                        return Err(IOError::NoSpaceOnDevice);
                    }
                    ENODEV => {
                        return Err(IOError::NoSuchDevice);
                    }
                    _ => {
                        panic!("Unhandled errno");
                    }
                }
            }

            return Ok(FileStream {
                handle: handle
            });
        }
    }

    /// Get the position within the stream
    pub fn position(&self) -> i32 {
        unsafe {
            return fs_tell(self.handle);
        }
    }

    /// Gets whether the stream has reached its end
    pub fn end_of_file(&self) -> bool {
        unsafe {
            return fs_eof(self.handle);
        }
    }
}

impl Drop for FileStream {
    fn drop(&mut self) {
        unsafe { fs_close(self.handle); }
    }
}

pub struct DirectoryEntry {
    pub name: String,
    pub is_directory: bool,
    pub size: i32,
    pub created: DateTime,
    pub modified: DateTime,
}

pub struct DirectoryInfo {
    handle: i32,
}

impl DirectoryInfo {
    /// Open the given directory
    pub fn open(path: &str) -> Result<DirectoryInfo, IOError> {
        unsafe {
            let path_cstr = CString::new(path).expect("Failed creating C string");
            let result = fs_openDir(path_cstr.as_ptr());

            match crate::db_internal::ERRNO {
                ESUCCESS => {
                }
                ENOENT => {
                    return Err(IOError::DirectoryNotFound);
                }
                ENODEV => {
                    return Err(IOError::NoSuchDevice);
                }
                _ => {
                    panic!("Unhandled errno");
                }
            }

            return Ok(DirectoryInfo {
                handle: result
            });
        }
    }

    /// Read the next entry from the directory list
    pub fn read(self) -> Option<DirectoryEntry> {
        unsafe {
            let dir_info_ptr = fs_readDir(self.handle);
            
            if dir_info_ptr.is_null() {
                return None;
            }
            
            let name_cstr = CStr::from_ptr((*dir_info_ptr).name.as_ptr());
            let name_str = name_cstr.to_str().unwrap();

            let mut created_dt = DateTime {
                year: 0,
                month: 0,
                day: 0,
                hour: 0,
                minute: 0,
                second: 0,
            };
            clock_timestampToDatetime((*dir_info_ptr).created, &mut created_dt);

            let mut modified_dt = DateTime {
                year: 0,
                month: 0,
                day: 0,
                hour: 0,
                minute: 0,
                second: 0,
            };
            clock_timestampToDatetime((*dir_info_ptr).modified, &mut modified_dt);

            return Some(DirectoryEntry {
                name: name_str.to_string(),
                is_directory: (*dir_info_ptr).is_directory != 0,
                size: (*dir_info_ptr).size,
                created: created_dt,
                modified: modified_dt,
            });
        }
    }

    /// Rewind to the beginning of the directory list
    pub fn rewind(self) {
        unsafe {
            fs_rewindDir(self.handle);
        }
    }
}

impl Drop for DirectoryInfo {
    fn drop(&mut self) {
        unsafe { fs_closeDir(self.handle); }
    }
}

/// Check if the given device exists <br/>
/// Valid devices are "cd", "ma", and "mb"
pub fn device_exists(device: &str) -> bool {
    unsafe {
        let path_cstr = CString::new(device).expect("Failed creating C string");
        return fs_deviceExists(path_cstr.as_ptr());
    }
}

/// Eject the given device, if it supports being ejected
pub fn device_eject(device: &str) {
    unsafe {
        let path_cstr = CString::new(device).expect("Failed creating C string");
        fs_deviceEject(path_cstr.as_ptr());
    }
}

/// Check if the given file exists
pub fn file_exists(path: &str) -> bool {
    unsafe {
        let path_cstr = CString::new(path).expect("Failed creating C string");
        return fs_fileExists(path_cstr.as_ptr());
    }
}