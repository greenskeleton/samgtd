use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Identity {
    pub node: String,
    pub dataset: String,
}
#[derive(Serialize, Deserialize)]
pub struct Provision {
    pub dataset: String,
    pub root: Vec<u8>,
}
#[derive(Serialize, Deserialize, Default)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub notes: Option<String>,
    pub status: Option<String>,
}
#[derive(Serialize, Deserialize)]
pub struct Envelope {
    pub version: u8,
    pub dataset: String,
    pub kind: String,
    pub id: String,
    pub message: Vec<u8>,
}
