use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use std::collections::hash_map::HashMap;
use std::fs::File;
use std::io::BufReader;

#[derive(Default, Deserialize, Serialize, Debug)]
pub struct Entry {
    pub expression: String,
    pub reading: Option<String>,
    pub source: String,
    pub speaker: Option<String>,
    pub display: String,
    pub file: String,
}
