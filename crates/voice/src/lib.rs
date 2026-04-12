//! Voice I/O — Offline-First
//!
//! STT: Vosk (offline) → ElevenLabs Scribe (fallback up)
//! TTS: Piper Thorsten (offline) → ElevenLabs Ramona (fallback up)
//!
//! SPECIAL mode: blocked when ElevenLabs unavailable
//! (no [seductively] with Thorsten's voice)

pub mod stt;
pub mod tts;

pub mod stt {
    //! Speech-to-Text
    //! Vosk offline → ElevenLabs API fallback
    pub fn transcribe(_audio: &[u8]) -> Option<String> { todo!() }
}

pub mod tts {
    //! Text-to-Speech
    //! Piper offline → ElevenLabs API fallback
    pub fn speak(_text: &str) -> Option<Vec<u8>> { todo!() }
    pub fn has_elevenlabs() -> bool { false } // Check API key
}
