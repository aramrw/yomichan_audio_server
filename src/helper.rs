use crate::database;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioSource {
    name: String,
    url: String,
}

pub fn map_entry_object(word_object: &serde_json::Value) -> Option<database::Entry> {
    let mut word_entry = database::Entry::default();

    match word_object {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                if key == "expression" {
                    word_entry.expression = value.as_str().unwrap_or("").to_string();
                } else if key == "reading" {
                    word_entry.reading = Some(value.as_str().unwrap_or("").to_string());
                } else if key == "source" {
                    word_entry.source = value.as_str().unwrap_or("").to_string();
                } else if key == "speaker" {
                    word_entry.speaker = Some(value.as_str().unwrap_or("").to_string());
                } else if key == "display" {
                    word_entry.display = value.as_str().unwrap_or("").to_string();
                } else if key == "file" {
                    word_entry.file = value.as_str().unwrap_or("").to_string();
                }
            }
        }
        _ => eprintln!("Not an object"),
    }

    //println!("{:?}", serde_json::to_string_pretty(&word_entry).unwrap());

    Some(word_entry)
}

pub fn find_audio_file(entry: &database::Entry) -> Option<AudioSource> {
    if entry.source == "shinmeikai8" {
        let shinmeikai_dir_path = "audio/shinmeikai8_files/media";
        let shinmeikai_dir = std::fs::read_dir(shinmeikai_dir_path).unwrap();

        for file in shinmeikai_dir {
            let file = file.unwrap();
            if file.file_name() == *entry.file {
                //println!("Found file: {:?}", file.file_name());
                let audio_source =
                    construct_audio_source("smk", &entry.display, shinmeikai_dir_path, &entry.file);
                return Some(audio_source);
            }
        }
    }

    if entry.source == "nhk16" {
        let nhk16_dir_path = "audio/nhk16_files/media";
        let nhk16_dir = std::fs::read_dir(nhk16_dir_path).unwrap();

        for file in nhk16_dir {
            let file = file.unwrap();
            if file.file_name() == *entry.file {
                //println!("Found file: {:?}", file.file_name());
                let audio_source =
                    construct_audio_source("nhk", &entry.display, nhk16_dir_path, &entry.file);
                return Some(audio_source);
            }
        }
    }

    if entry.source == "jpod" {
        let jpod_dir_path = "audio/jpod_files/audio";
        let jpod_dir = std::fs::read_dir(jpod_dir_path).unwrap();

        for file in jpod_dir {
            let file = file.unwrap();
            if file.file_name() == *entry.file {
                //println!("Found file: {:?}", file.file_name());
                let audio_source = construct_audio_source("jpod", "", jpod_dir_path, &entry.file);
                return Some(audio_source);
            }
        }
    }

    if entry.source == "forvo" {
        let forvo_dir_paths = [
            "audio/forvo_files/strawberrybrown",
            "audio/forvo_files/kaoring",
            "audio/forvo_files/poyotan",
            "audio/forvo_files/akimoto",
            "audio/forvo_files/skent",
        ];

        for dir in forvo_dir_paths.iter() {
            let forvo_dir = std::fs::read_dir(dir).unwrap();

            for file in forvo_dir {
                let file = file.unwrap();
                if file.file_name() == *entry.file {
                    //println!("Found file: {:?}", file.file_name());
                    let audio_source = construct_audio_source(entry.speaker.as_ref().unwrap(), "", dir, &entry.file);
                    return Some(audio_source);
                }
            }
        }
    }

    None
}

fn construct_audio_source(
    dict_name: &str,
    entry_display: &str,
    main_dir: &str,
    file_name: &str,
) -> AudioSource {
    if entry_display.is_empty() {
        return AudioSource {
            name: dict_name.to_string(),
            url: format!("http://localhost:8080/{}/{}", main_dir, file_name),
        };
    }

    let display = format!("{} - {}", dict_name, entry_display);
    AudioSource {
        name: display,
        url: format!("http://localhost:8080/{}/{}", main_dir, file_name),
    }
}

