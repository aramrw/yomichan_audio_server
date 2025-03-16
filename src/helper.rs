use crate::database::DatabaseEntry;
use bimap::BiHashMap;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(thiserror::Error)]
pub enum AudioFileError {
    // #[error("could not recognize audio source: {src}")]
    // UnkownSource { src: String },
    // #[error("unknown forvo speaker: {speaker}")]
    // UnknownForvoSpeaker { speaker: String },
    // #[error("missing entry speaker. forvo audio must contain a speaker: {forvo_speakers}")]
    // MissingForvoEntrySpeaker { forvo_speakers: String },
    #[error("{dir} does not contain audio file for: {entry:#?}")]
    MissingAudioFile { entry: DatabaseEntry, dir: String },
    #[error("io error: {}", .0)]
    Io(#[from] std::io::Error),
}

impl std::fmt::Debug for AudioFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AudioResult {
    pub name: String,
    pub url: String,
}

impl AudioResult {
    pub fn create_list(entries: &[DatabaseEntry]) -> Vec<AudioResult> {
        let mut audio_sources_list: Vec<AudioResult> = Vec::with_capacity(entries.len());
        if !entries.is_empty() {
            let audio_files_res: Vec<AudioResult> = entries
                .par_iter()
                .filter_map(|e| e.to_audio_result().ok())
                .collect();
            audio_sources_list = audio_files_res;
        }
        audio_sources_list
    }
    pub fn print_list(list: &[AudioResult]) {
        for entry in list {
            println!("    ▼ {}\n        {}", entry.name, entry.url);
        }
    }
}

#[allow(dead_code)]
pub static AUDIO_FILE_STEMS: LazyLock<std::collections::HashSet<&'static str>> =
    LazyLock::new(|| std::collections::HashSet::from_iter(["mp4", "mp3", "wav", "ogg", "flac"]));

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
