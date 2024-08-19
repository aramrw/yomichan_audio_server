use crate::database;
//use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AudioSource {
    name: String,
    url: String,
}

pub fn find_audio_file(entry: &database::Entry) -> Option<AudioSource> {
    if entry.source == "shinmeikai8" {
        let shinmeikai_dir_path = "audio/shinmeikai8_files/media";

        let audio_source =
            search_dir_helper("smk", &entry.display, shinmeikai_dir_path, &entry.file).unwrap();

        return Some(audio_source);
    }

    if entry.source == "nhk16" {
        let nhk16_dir_path = "audio/nhk16_files/media";

        let audio_source =
            search_dir_helper("nhk", &entry.display, nhk16_dir_path, &entry.file).unwrap();
        return Some(audio_source);
    }

    if entry.source == "daijisen" {
        let daijisen_dir_path = "audio/daijisen_files/audio";

        let audio_source =
            search_dir_helper("daijisen", &entry.display, daijisen_dir_path, &entry.file).unwrap();
        return Some(audio_source);
    }

    if entry.source == "jpod" {
        let jpod_dir_path = "audio/jpod_files/audio";

        let audio_source = search_dir_helper("jpod", "", jpod_dir_path, &entry.file).unwrap();
        return Some(audio_source);
    }

    if entry.source == "forvo" {
        let forvo_speakers = ["strawberrybrown", "kaoring", "poyotan", "akitomo", "skent"];

        for speaker in forvo_speakers.iter() {
            if speaker != entry.speaker.as_ref().unwrap() {
                continue;
            }

            let format_dir = format!("audio/forvo_files/{}", speaker);

            let audio_source = search_dir_helper(
                entry.speaker.as_ref().unwrap(),
                "",
                &format_dir,
                &entry.file,
            )
            .unwrap();
            return Some(audio_source);
        }
    }

    None
}

// fn search_dir_helper(
//     dict_name: &str,
//     entry_display: &str,
//     main_dir: &str,
//     file_name: &str,
// ) -> Result<Option<AudioSource>, std::io::Error> {
//     let read_dir = std::fs::read_dir(main_dir)?;
//
//     let result: Option<AudioSource> = read_dir
//         .par_bridge()
//         .filter_map(|file| {
//             let file = file.unwrap();
//             if file.file_name() == file_name {
//                 let audio_source =
//                     construct_audio_source(dict_name, entry_display, main_dir, file_name);
//                 return Some(audio_source);
//             }
//
//             None
//         })
//         .find_any(|_| true);
//
//     Ok(result)
// }

fn search_dir_helper(
    dict_name: &str,
    entry_display: &str,
    main_dir: &str,
    file_name: &str,
) -> Result<AudioSource, std::io::Error> {
    let file_path = Path::new(main_dir).join(file_name);
    std::fs::File::open(file_path)?;

    Ok(construct_audio_source(
        dict_name,
        entry_display,
        main_dir,
        file_name,
    ))
}

fn construct_audio_source(
    dict_name: &str,
    entry_display: &str,
    main_dir: &str,
    file_name: &str,
) -> AudioSource {
    // if is forvo file
    if entry_display.is_empty() {
        return AudioSource {
            name: dict_name.to_string(),
            url: format!("http://localhost:8080/{}/{}", main_dir, file_name),
        };
    }
    
    // dict files
    let display = format!("{} {}", dict_name, entry_display);
    AudioSource {
        name: display,
        url: format!("http://localhost:8080/{}/{}", main_dir, file_name),
    }
}

pub fn convert_kana(term: &str) -> String {
    let chars = term.chars();

    chars
        .map(|c| {
            let mut tmp = [0u8];
            let str = c.encode_utf8(&mut tmp);
            match KANA_MAP.get_by_left(str) {
                Some(hg) => *hg,
                None => *KANA_MAP.get_by_right(str).unwrap(),
            }
        })
        .collect::<Vec<&str>>()
        .concat()
}

