use crate::database;
//use rayon::prelude::*;
use bimap::BiHashMap;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::LazyLock;

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

            //println!("returning {:?}", audio_source);

            return Some(audio_source);
        }
    }

    if entry.source == "forvo_zh" {
        let audio_source =
            search_dir_helper_forvo(entry.speaker.as_ref().unwrap(), "", "audio/zh", &entry.file)
                .unwrap();

        println!("returning {:?}", audio_source);
        return audio_source;
    }

    None
}

fn search_dir_helper_forvo(
    speaker: &str,
    entry_display: &str,
    main_dir: &str,
    file_name: &str,
) -> Result<Option<AudioSource>, std::io::Error> {
    // Iterate over each folder in the main directory
    for folder in std::fs::read_dir(main_dir)? {
        let folder = folder?;
        let folder_name = folder.file_name();
        let folder_name_str = folder_name.to_string_lossy(); // Convert to &str for comparison
        let folder_path = folder.path();

        // Skip folders that do not match the speaker name
        if folder_name_str != speaker {
            continue; // Continue to the next folder if names do not match
        }

        // Search for the file within the matched folder
        let result = std::fs::read_dir(&folder_path)?.find_map(|entry| {
            match entry {
                Ok(entry) => {
                    let entry_file_name = entry.file_name();
                    if entry_file_name == file_name {
                        // Construct and return the audio source if file matches
                        Some(construct_audio_source(
                            speaker,
                            entry_display,
                            &format!("{}/{}", main_dir, speaker),
                            file_name,
                        ))
                    } else {
                        None
                    }
                }
                Err(e) => {
                    // Log any errors reading directory entries
                    eprintln!("Error reading directory entry: {}", e);
                    None
                }
            }
        });

        // Return the result if found
        if result.is_some() {
            return Ok(result);
        }
    }

    // Return None if no file was found
    Ok(None)
}

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

#[rustfmt::skip]
pub static KANA_MAP: LazyLock<BiHashMap<&'static str, &'static str>> = LazyLock::new(|| {
    BiHashMap::from_iter([
        ("ア", "あ"), ("イ", "い"), ("ウ", "う"), ("エ", "え"), ("オ", "お"),
        ("カ", "か"), ("キ", "き"), ("ク", "く"), ("ケ", "け"), ("コ", "こ"),
        ("サ", "さ"), ("シ", "し"), ("ス", "す"), ("セ", "せ"), ("ソ", "そ"),
        ("タ", "た"), ("チ", "ち"), ("ツ", "つ"), ("テ", "て"), ("ト", "と"),
        ("ナ", "な"), ("ニ", "に"), ("ヌ", "ぬ"), ("ネ", "ね"), ("ノ", "の"),
        ("ハ", "は"), ("ヒ", "ひ"), ("フ", "ふ"), ("ヘ", "へ"), ("ホ", "ほ"),
        ("マ", "ま"), ("ミ", "み"), ("ム", "む"), ("メ", "め"), ("モ", "も"),
        ("ヤ", "や"), ("ユ", "ゆ"), ("ヨ", "よ"), ("ラ", "ら"), ("リ", "り"),
        ("ル", "る"), ("レ", "れ"), ("ロ", "ろ"), ("ワ", "わ"), ("ヲ", "を"),
        ("ン", "ん"), ("ガ", "が"), ("ギ", "ぎ"), ("グ", "ぐ"), ("ゲ", "げ"),
        ("ゴ", "ご"), ("ザ", "ざ"), ("ジ", "じ"), ("ズ", "ず"), ("ゼ", "ぜ"),
        ("ゾ", "ぞ"), ("ダ", "だ"), ("ヂ", "ぢ"), ("ヅ", "づ"), ("デ", "で"),
        ("ド", "ど"), ("バ", "ば"), ("ビ", "び"), ("ブ", "ぶ"), ("ベ", "べ"),
        ("ボ", "ぼ"), ("パ", "ぱ"), ("ピ", "ぴ"), ("プ", "ぷ"), ("ペ", "ぺ"),
        ("ポ", "ぽ"), ("キャ", "きゃ"), ("キュ", "きゅ"), ("キョ", "きょ"),
        ("シャ", "しゃ"), ("シュ", "しゅ"), ("ショ", "しょ"), ("チャ", "ちゃ"),
        ("チュ", "ちゅ"), ("チョ", "ちょ"), ("ニャ", "にゃ"), ("ニュ", "にゅ"),
        ("ニョ", "にょ"), ("ヒャ", "ひゃ"), ("ヒュ", "ひゅ"), ("ヒョ", "ひょ"),
        ("ミャ", "みゃ"), ("ミュ", "みゅ"), ("ミョ", "みょ"), ("リャ", "りゃ"),
        ("リュ", "りゅ"), ("リョ", "りょ"), ("ギャ", "ぎゃ"), ("ギュ", "ぎゅ"),
        ("ギョ", "ぎょ"), ("ジャ", "じゃ"), ("ジュ", "じゅ"), ("ジョ", "じょ"),
        ("ビャ", "びゃ"), ("ビュ", "びゅ"), ("ビョ", "びょ"), ("ピャ", "ぴゃ"),
        ("ピュ", "ぴゅ"), ("ピョ", "ぴょ"),
    ])
});
