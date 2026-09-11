use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{
    fmt,
    hash::{Hash, Hasher},
    path::PathBuf,
};
use strum::EnumIter;

/// A collection of paths from the platform, such as from a file drop.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct ExternalPaths(pub SmallVec<[PathBuf; 2]>);

impl ExternalPaths {
    /// Convert this collection of paths into a slice.
    pub fn paths(&self) -> &[PathBuf] {
        &self.0
    }
}

/// One of the image formats supported by the clipboard.
#[derive(Clone, Copy, Debug, Eq, PartialEq, EnumIter, Hash)]
pub enum ImageFormat {
    /// PNG.
    Png,
    /// JPEG or JPG.
    Jpeg,
    /// WebP.
    Webp,
    /// GIF.
    Gif,
    /// SVG.
    Svg,
    /// BMP.
    Bmp,
    /// TIFF.
    Tiff,
    /// ICO.
    Ico,
    /// Netpbm image formats.
    Pnm,
}

impl ImageFormat {
    /// Returns the MIME type for this image format.
    pub const fn mime_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
            Self::Gif => "image/gif",
            Self::Svg => "image/svg+xml",
            Self::Bmp => "image/bmp",
            Self::Tiff => "image/tiff",
            Self::Ico => "image/ico",
            Self::Pnm => "image/x-portable-anymap",
        }
    }

    /// Returns the file extension for this image format, without a leading dot.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
            Self::Gif => "gif",
            Self::Svg => "svg",
            Self::Bmp => "bmp",
            Self::Tiff => "tiff",
            Self::Ico => "ico",
            Self::Pnm => "pnm",
        }
    }

    /// Returns the image format for a MIME type, including known aliases.
    pub fn from_mime_type(mime_type: &str) -> Option<Self> {
        use strum::IntoEnumIterator;

        Self::iter()
            .find(|format| format.mime_type() == mime_type)
            .or_else(|| Self::from_mime_type_alias(mime_type))
    }

    fn from_mime_type_alias(mime_type: &str) -> Option<Self> {
        match mime_type {
            "image/jpg" => Some(Self::Jpeg),
            "image/tif" => Some(Self::Tiff),
            _ => None,
        }
    }
}

/// Image data carried by a backend-neutral clipboard item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipboardImage {
    /// The image format represented by the bytes.
    pub format: ImageFormat,
    /// The raw image bytes.
    pub bytes: Vec<u8>,
    /// The unique ID for the image.
    pub id: u64,
}

impl Hash for ClipboardImage {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.id);
    }
}

impl ClipboardImage {
    /// Create an image from a format and bytes.
    pub fn from_bytes(format: ImageFormat, bytes: Vec<u8>) -> Self {
        Self {
            id: seahash::hash(&bytes),
            format,
            bytes,
        }
    }

    /// Get the image's format.
    pub fn format(&self) -> ImageFormat {
        self.format
    }

    /// Get the image's raw bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// A clipboard item that should be copied to the clipboard.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipboardItem<I = ClipboardImage> {
    /// The entries in this clipboard item.
    pub entries: Vec<ClipboardEntry<I>>,
}

/// An error produced by an asynchronous clipboard read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipboardReadError {
    /// The platform clipboard is not available in this context.
    Unavailable,
    /// The platform refused access.
    Denied(String),
    /// The clipboard contents could not be converted into a clipboard item.
    UnsupportedContent,
}

impl fmt::Display for ClipboardReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("the clipboard is unavailable"),
            Self::Denied(message) => write!(formatter, "clipboard access was denied: {message}"),
            Self::UnsupportedContent => {
                formatter.write_str("the clipboard contents are unsupported")
            }
        }
    }
}

impl std::error::Error for ClipboardReadError {}

/// A single backend-neutral clipboard entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClipboardEntry<I = ClipboardImage> {
    /// A string entry.
    String(ClipboardString),
    /// An image entry.
    Image(I),
    /// A file entry.
    ExternalPaths(ExternalPaths),
}

impl<I> ClipboardItem<I> {
    /// Create a new string clipboard item with no associated metadata.
    pub fn new_string(text: String) -> Self {
        Self {
            entries: vec![ClipboardEntry::String(ClipboardString::new(text))],
        }
    }

    /// Create a new string clipboard item with associated metadata.
    pub fn new_string_with_metadata(text: String, metadata: String) -> Self {
        Self {
            entries: vec![ClipboardEntry::String(ClipboardString {
                text,
                metadata: Some(metadata),
            })],
        }
    }

    /// Create a new string clipboard item with JSON metadata.
    pub fn new_string_with_json_metadata<T: Serialize>(text: String, metadata: T) -> Self {
        Self {
            entries: vec![ClipboardEntry::String(
                ClipboardString::new(text).with_json_metadata(metadata),
            )],
        }
    }

    /// Create a new image clipboard item.
    pub fn new_image(image: &I) -> Self
    where
        I: Clone,
    {
        Self {
            entries: vec![ClipboardEntry::Image(image.clone())],
        }
    }

    /// Concatenate all string entries in the item, falling back to file paths.
    pub fn text(&self) -> Option<String> {
        let mut answer = String::new();

        for entry in &self.entries {
            if let ClipboardEntry::String(ClipboardString { text, metadata: _ }) = entry {
                answer.push_str(text);
            }
        }

        if answer.is_empty() {
            for entry in &self.entries {
                if let ClipboardEntry::ExternalPaths(paths) = entry {
                    for path in &paths.0 {
                        use std::fmt::Write as _;
                        _ = write!(answer, "{}", path.display());
                    }
                }
            }
        }

        (!answer.is_empty()).then_some(answer)
    }

    /// If this item is one string entry, return its metadata.
    pub fn metadata(&self) -> Option<&String> {
        match self.entries.first() {
            Some(ClipboardEntry::String(clipboard_string)) if self.entries.len() == 1 => {
                clipboard_string.metadata.as_ref()
            }
            _ => None,
        }
    }

    /// Get the item's entries.
    pub fn entries(&self) -> &[ClipboardEntry<I>] {
        &self.entries
    }

    /// Get owned versions of the item's entries.
    pub fn into_entries(self) -> impl Iterator<Item = ClipboardEntry<I>> {
        self.entries.into_iter()
    }
}

impl<I> From<ClipboardString> for ClipboardEntry<I> {
    fn from(value: ClipboardString) -> Self {
        Self::String(value)
    }
}

impl From<ClipboardImage> for ClipboardEntry {
    fn from(value: ClipboardImage) -> Self {
        Self::Image(value)
    }
}

impl<I> From<String> for ClipboardEntry<I> {
    fn from(value: String) -> Self {
        Self::from(ClipboardString::from(value))
    }
}

impl<I> From<ClipboardEntry<I>> for ClipboardItem<I> {
    fn from(value: ClipboardEntry<I>) -> Self {
        Self {
            entries: vec![value],
        }
    }
}

impl<I> From<String> for ClipboardItem<I> {
    fn from(value: String) -> Self {
        Self::from(ClipboardEntry::from(value))
    }
}

impl From<ClipboardImage> for ClipboardItem {
    fn from(value: ClipboardImage) -> Self {
        Self::from(ClipboardEntry::from(value))
    }
}

/// Clipboard operations supplied by a platform implementation.
pub trait PlatformClipboardSpi {
    /// The concrete image type used by the platform's clipboard item.
    type Image: Clone;

    /// Read data from the platform clipboard.
    fn read_from_clipboard(&self) -> Option<ClipboardItem<Self::Image>>;

    /// Write data to the platform clipboard.
    fn write_to_clipboard(&self, item: ClipboardItem<Self::Image>);

    /// Clear the platform clipboard.
    fn clear_clipboard(&self);
}

/// A string entry in a clipboard item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipboardString {
    /// The text content.
    pub text: String,
    /// Optional metadata associated with this clipboard string.
    pub metadata: Option<String>,
}

impl ClipboardString {
    /// Create a new clipboard string.
    pub fn new(text: String) -> Self {
        Self {
            text,
            metadata: None,
        }
    }

    /// Replace the metadata with its JSON representation.
    pub fn with_json_metadata<T: Serialize>(mut self, metadata: T) -> Self {
        self.metadata = Some(serde_json::to_string(&metadata).unwrap());
        self
    }

    /// Get the text content.
    pub fn text(&self) -> &String {
        &self.text
    }

    /// Get the owned text content.
    pub fn into_text(self) -> String {
        self.text
    }

    /// Parse the metadata as JSON.
    pub fn metadata_json<T>(&self) -> Option<T>
    where
        T: for<'a> Deserialize<'a>,
    {
        self.metadata
            .as_ref()
            .and_then(|metadata| serde_json::from_str(metadata).ok())
    }

    /// Compute a hash of text for clipboard change detection.
    pub fn text_hash(text: &str) -> u64 {
        let mut hasher = seahash::SeaHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }
}

impl From<String> for ClipboardString {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}
