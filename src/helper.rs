use crate::database::{self, Entry};
use bimap::BiHashMap;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::LazyLock;

#[derive(Debug, thiserror::Error)]
pub enum AudioFileError {
    #[error("could not recognize audio source: {src}")]
    UnkownSource { src: String },
    #[error("unknown forvo speaker: {speaker}")]
    UnknownForvoSpeaker { speaker: String },
    #[error("missing entry speaker. forvo audio must contain a speaker: {forvo_speakers}")]
    MissingForvoEntrySpeaker { forvo_speakers: String },
    #[error("missing entry speaker: {file_name}")]
    MissingAudioFile { file_name: String },
    #[error("io error")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AudioSource {
    pub name: String,
    pub url: String,
}

impl AudioSource {
    pub fn create_list(entries: &[Entry]) -> Vec<AudioSource> {
        // contruct the list of audio sources
        let mut audio_sources_list: Vec<AudioSource> = Vec::with_capacity(entries.len());

        if !entries.is_empty() {
            let audio_files_res: Vec<AudioSource> = entries
                .par_iter()
                .filter_map(|e| Some(find_audio_file(e).unwrap())) // remove `None` values
                .collect();

            audio_sources_list = audio_files_res;
        }
        audio_sources_list
    }

    pub fn print_list(list: &[AudioSource]) {
        for entry in list {
            println!("    ▼ {}\n        {}", entry.name, entry.url);
        }
    }
}

pub fn find_audio_file(entry: &database::Entry) -> Result<AudioSource, AudioFileError> {
    let jp_forvo_speakers = ["strawberrybrown", "kaoring", "poyotan", "akitomo", "skent"];

    match entry.source.as_str() {
        "shinmeikai8" => {
            let shinmeikai_dir_path = "audio/shinmeikai8_files/media";
            search_dir_helper("smk", &entry.display, shinmeikai_dir_path, &entry.file)
        }
        "nhk16" => {
            let nhk16_dir_path = "audio/nhk16_files/media";
            search_dir_helper("nhk", &entry.display, nhk16_dir_path, &entry.file)
        }
        "daijisen" => {
            let daijisen_dir_path = "audio/daijisen_files/media";
            search_dir_helper("daijisen", &entry.display, daijisen_dir_path, &entry.file)
        }
        "jpod" => {
            let jpod_dir_path = "audio/jpod_files/media";
            search_dir_helper("jpod", "", jpod_dir_path, &entry.file)
        }
        // should be renamed to forvo_jp
        "forvo" => {
            let Some(entry_speaker) = entry.speaker.clone() else {
                return Err(AudioFileError::MissingForvoEntrySpeaker {
                    forvo_speakers: jp_forvo_speakers.join(" "),
                });
            };

            let Some(current_speaker) = jp_forvo_speakers
                .into_iter()
                .find(|speaker| *speaker == entry_speaker)
            else {
                return Err(AudioFileError::UnknownForvoSpeaker {
                    speaker: entry_speaker,
                });
            };

            let format_dir = format!("audio/jp_forvo_files/{}", current_speaker);

            search_dir_helper(
                entry.speaker.as_ref().unwrap(),
                "",
                &format_dir,
                &entry.file,
            )
        }
        "forvo_zh" => search_dir_helper_forvo(
            entry.speaker.as_deref().unwrap(),
            "",
            "audio/zh",
            &entry.file,
        ),

        _ => Err(AudioFileError::UnkownSource {
            src: entry.source.clone(),
        }),
    }
}

fn search_dir_helper_forvo(
    speaker: &str,
    entry_display: &str,
    main_dir: &str,
    file_name: &str,
) -> Result<AudioSource, AudioFileError> {
    // Iterate over each folder in the main directory
    for folder in std::fs::read_dir(main_dir)? {
        let folder = folder?;
        let folder_name = folder.file_name();
        let folder_name_str = folder_name.to_string_lossy();
        let folder_path = folder.path();

        // Skip folders that do not match the speaker name
        if folder_name_str != speaker {
            continue;
        }

        // Search for the file within the matched folder
        let final_audio = std::fs::read_dir(&folder_path)?.find_map(|entry| {
            let entry = entry.ok()?;
            let entry_file_name = entry.file_name();
            if entry_file_name == file_name {
                // Construct and return the audio source if file matches
                let audio_src = construct_audio_source(
                    speaker,
                    entry_display,
                    &format!("{}/{}", main_dir, speaker),
                    file_name,
                );
                return Some(audio_src);
            }
            None
        });

        if let Some(final_audio) = final_audio {
            return Ok(final_audio);
        }
    }

    Err(AudioFileError::MissingAudioFile {
        file_name: file_name.to_string(),
    })
}

fn search_dir_helper(
    dict_name: &str,
    entry_display: &str,
    main_dir: &str,
    file_name: &str,
) -> Result<AudioSource, AudioFileError> {
    let file_path = Path::new(main_dir).join(file_name);
    //println!("searching path {:#?}", file_path);
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
