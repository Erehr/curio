//! The numbers the engine works with, in one place.

/// How much of a text file its thumbnail reads.
pub const TEXT_THUMB_BYTES: u64 = 24576;
/// One frame when a video doesn't say its rate.
pub const PLAYER_FRAME_SECONDS: f64 = 0.03333333;
/// The longest side of every thumbnail.
pub const THUMBNAIL_SIZE: u32 = 512;
/// Preview size for big images.
pub const BIG_IMAGE_PREVIEW_SIZE: u32 = 1024;
pub const THUMBNAIL_QUALITY: u8 = 82;
pub const THUMBNAIL_TIMEOUT_MS: u64 = 90_000;

pub const PALETTE_COLORS: usize = 10;
/// Colours covering less than this percentage are noise.
pub const PALETTE_MIN_RATIO: f32 = 0.3;
/// How far (CIEDE2000) a colour may be from the one asked for.
pub const COLOR_SIMILAR_DE2000: f64 = 16.0;
pub const COLOR_ACCURATE_DE2000: f64 = 8.0;
/// The distance at accuracy 0 and how much each point of accuracy (to 100) takes off it.
pub const COLOR_LOOSEST_DE2000: f64 = 28.0;
pub const COLOR_DE2000_PER_ACCURACY: f64 = 0.24;
/// How much of a picture a colour must cover to count as a near match.
pub const COLOR_SWATCH_MIN_RATIO: f32 = 4.0;
/// A picture is grayscale when none of its visible colours has more chroma than this.
pub const GRAYSCALE_MAX_CHROMA: f64 = 6.0;
pub const GRAYSCALE_MIN_RATIO: f32 = 0.5;

pub const PANORAMIC_RATIO: f64 = 2.35;

/// How far from an exact ratio still counts for the 4:3, 16:9 and custom shape filters.
pub const RATIO_TOLERANCE: f64 = 0.015;

/// Website thumbnails: size of the hidden page capture.
pub const PAGE_CAPTURE_WIDTH: u32 = 1280;
pub const PAGE_CAPTURE_HEIGHT: u32 = 800;
pub const PAGE_CAPTURE_TIMEOUT_SECS: u64 = 25;
/// How long a page capture waits for the page to settle before it takes the page as it stands.
pub const PAGE_CAPTURE_SETTLED_SECS: u64 = 15;

pub const MAX_CONDITIONS: usize = 40;
pub const MAX_RULES_PER_CONDITION: usize = 40;

/// Similar pictures (prints.rs): the thumbnail reduced to this square of greys, of which the lowest frequencies count.
pub const PHASH_IMAGE_SIZE: usize = 32;
pub const PHASH_LOW_FREQUENCIES: usize = 8;
/// The duplicate finder's similarity unless set.
pub const SIMILAR_THRESHOLD: f64 = 0.9;
/// Pictures whose shapes differ by more than this (the logarithm of their aspect ratios' quotient) are other pictures.
pub const DUPLICATE_ASPECT_TOLERANCE: f64 = 0.08;
/// How many bits more the gradient hash may differ by than the DCT hash.
pub const DUPLICATE_GRADIENT_SLACK: u32 = 4;
/// Pictures whose quarters' colours are further apart than this, on average (CIE76), are other pictures.
pub const DUPLICATE_COLOUR_GAP: f64 = 12.0;
/// Frames read at even steps through each video.
pub const DUPLICATE_VIDEO_FRAMES: usize = 6;
/// How much of each sound is compared, from where it starts.
pub const DUPLICATE_SOUND_SECONDS: u32 = 60;

/// The watcher gathers file events for this long before acting on them.
pub const WATCH_BATCH_MS: u64 = 300;

/// Hashing: bytes read from each end of a file for the quick identity hash.
pub const QUICK_HASH_CHUNK: usize = 64 * 1024;
/// Rows per write transaction during scans.
pub const SCAN_BATCH: usize = 500;
/// Items per writer job when purging.
pub const PURGE_CHUNK: usize = 1000;
/// Watcher: more events than this in one batch → full reconcile of the root.
pub const WATCH_OVERFLOW: usize = 2000;

/// Auto-import folder poll interval (seconds); a file is taken once its size is stable over a poll.
pub const AUTO_IMPORT_POLL_SECS: u64 = 5;
