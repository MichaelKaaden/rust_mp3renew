use std::fmt;
use std::fmt::Formatter;
use std::fs;

use walkdir::WalkDir;

use crate::config::Config;
use crate::music_file::MusicFile;
use crate::ordinary_file::OrdinaryFile;
use crate::util;

pub struct DirContents {
    pub dir_entry: walkdir::DirEntry,
    pub music_files: Vec<MusicFile>,       // contained music files
    pub ordinary_files: Vec<OrdinaryFile>, // contained other files (potentially being deleted)
}

impl DirContents {
    pub fn new(config: &Config) -> Vec<DirContents> {
        let all_files_and_directories = get_list_of_dirs(&config);
        let music_directories = get_dirs_with_music(all_files_and_directories);
        music_directories
    }

    // Which album name does the whole directory have for all music files?
    pub fn same_album_title(&self) -> Option<&String> {
        let albums: Vec<&String> = self
            .music_files
            .iter()
            .filter_map(|m| m.music_metadata.as_ref())
            .map(|m| &m.album)
            .collect();

        if albums.len() > 0 {
            let first_album = albums[0];
            for album in albums {
                if album != first_album {
                    return None;
                }
            }
            return Some(first_album);
        }

        None
    }

    /// Has the whole directory the same artist for every music file?
    pub fn same_artists(&self) -> bool {
        let artists: Vec<&String> = self
            .music_files
            .iter()
            .filter_map(|m| m.music_metadata.as_ref())
            .map(|m| &m.artist)
            .collect();

        if artists.len() > 0 {
            let first_artist = artists[0];
            for artist in artists {
                if artist != first_artist {
                    return false;
                }
            }
            return true;
        }

        // an error for missing tags has already been reported in MusicMetadata::new()
        false
    }
}

impl fmt::Display for DirContents {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Directory Name: {}",
            self.dir_entry.path().to_string_lossy()
        )?;
        writeln!(f, "music files:    {} entries", self.music_files.len())?;
        writeln!(f, "ordinary files: {} entries", self.ordinary_files.len())?;
        for o in &self.ordinary_files {
            writeln!(
                f,
                "ordinary file:  {}",
                o.dir_entry.path().to_string_lossy()
            )?;
        }
        for m in &self.music_files {
            writeln!(f, "{}", m)?;
        }

        fmt::Result::Ok(())
    }
}

/// Returns the list of directories.
fn get_list_of_dirs(config: &Config) -> Vec<walkdir::DirEntry> {
    WalkDir::new(&config.start_dir)
        .contents_first(true)
        .into_iter()
        .filter_entry(|e| e.file_type().is_dir())
        // filter out errors (cannot print warnings!)
        //.filter_map(Result::ok)
        // filter *and* report errors
        .filter(|e| match e {
            Ok(_) => true,
            Err(err) => {
                eprintln!("Error traversing directories: {}", err);
                false
            }
        })
        // convert to DirEntry
        .map(|e| e.unwrap())
        .collect()
}

/// Returns directories containing music files
fn get_dirs_with_music(files_and_directories: Vec<walkdir::DirEntry>) -> Vec<DirContents> {
    let mut dir_contents = vec![];

    for dir in files_and_directories {
        if dir.file_type().is_dir() {
            let readdir = fs::read_dir(dir.path());
            if readdir.is_ok() {
                let (music, others): (Vec<fs::DirEntry>, Vec<fs::DirEntry>) = readdir
                    .unwrap()
                    .filter(|dir_entry| dir_entry.as_ref().unwrap().path().is_file())
                    .map(|dir_entry| dir_entry.unwrap())
                    .partition(|dir_entry| util::is_music_file(dir_entry));

                // only return directories containing music files
                if music.len() > 0 {
                    let mut music_files: Vec<MusicFile> = music
                        .into_iter()
                        .map(|dir_entry| MusicFile::new(dir_entry))
                        .filter(|music_file| music_file.music_metadata.is_some())
                        .collect();
                    music_files.sort_by(|left, right| MusicFile::sort_func(left, right));

                    let ordinary_files: Vec<OrdinaryFile> =
                        others.into_iter().map(|o| OrdinaryFile::new(o)).collect();

                    dir_contents.push(DirContents {
                        dir_entry: dir,
                        music_files,
                        ordinary_files,
                    });
                }
            }
        }
    }

    dir_contents
}
