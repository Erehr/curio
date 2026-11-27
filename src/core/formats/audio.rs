//! Sound. Covers come from the attached picture; waveforms are measured once and cached.
use super::Family;
use crate::core::model::AssetKind;

pub const FAMILY: Family = Family {
    id: "audio",
    name: "Audio",
    description: "Covers, waveforms, playback and audio metadata.",
    icon: "music",
    kind: AssetKind::Audio,
    exts: &[
        "mp3", "wav", "flac", "ogg", "oga", "opus", "aac", "m4a", "aiff", "aif", "wma",
    ],
};
