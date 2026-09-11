use std::ops::Range;

/// A change in the state of the focused text input.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum TextInputStateChange {
    /// The window changed from having no active text input to having one.
    FocusGained,
    /// The window no longer has an active text input.
    FocusLost,
    /// The selection or caret moved.
    SelectionChanged,
    /// The document content changed outside of platform-initiated edits.
    ContentChanged,
}

/// A selection in a text buffer, in UTF-16 characters.
#[derive(Debug)]
pub struct UTF16Selection {
    /// The range of text in the document this selection corresponds to.
    pub range: Range<usize>,
    /// Whether the head of this selection is at the start of the range.
    pub reversed: bool,
}

/// Platform text-input preferences for the focused text region.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextInputConfiguration {
    /// Whether the platform may automatically correct entered text.
    pub autocorrect: bool,
    /// How software keyboards automatically capitalize entered text.
    pub autocapitalize: Autocapitalize,
    /// Whether software keyboards may offer word suggestions and spellcheck.
    pub suggestions: bool,
    /// The action advertised on a software keyboard's confirm ("enter") key.
    pub input_action: TextInputAction,
}

/// Automatic capitalization applied by software keyboards.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Autocapitalize {
    /// No automatic capitalization.
    #[default]
    None,
    /// Capitalize the first letter of each word.
    Words,
    /// Capitalize the first letter of each sentence.
    Sentences,
    /// Capitalize every letter.
    Characters,
}

/// The action a software keyboard advertises on its confirm ("enter") key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextInputAction {
    /// Let the platform choose its default presentation.
    #[default]
    Unspecified,
    /// Inserting a line break.
    Enter,
    /// Committing the field's value.
    Done,
    /// Navigating to the typed target.
    Go,
    /// Moving to the next field.
    Next,
    /// Moving to the previous field.
    Previous,
    /// Executing a search.
    Search,
    /// Sending a message.
    Send,
}

/// Text-input operations supplied by a platform window.
pub trait PlatformTextInputSpi {
    /// Applies the focused text region's platform text-input preferences.
    fn set_text_input_configuration(&mut self, configuration: TextInputConfiguration);

    /// Informs the operating system that the text input state changed.
    fn text_input_state_changed(&self, change: TextInputStateChange);
}
