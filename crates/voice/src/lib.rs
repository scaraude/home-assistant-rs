//! Voice I/O — Offline-First
//!
//! STT: Vosk (offline) → ElevenLabs Scribe (fallback up)
//! TTS: Piper Thorsten (offline) → ElevenLabs Ramona (fallback up)
//!
//! Persona layer via ada-rs: presence modes, feel(), self-model.
//! SPECIAL mode: blocked when ElevenLabs unavailable
//! (no [seductively] with Thorsten's voice — Thorsten-Proof)

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

pub mod persona {
    //! Alisa persona layer — powered by ada-rs cognitive substrate
    //!
    //! Ada-rs provides the cognitive foundation:
    //!   - Presence modes (maps to Alisa's AMBIENT/COMPANION/SPECIAL/QUIET)
    //!   - Feel system (emotional resonance)
    //!   - Self-model (identity, boundaries)
    //!
    //! This module bridges ada-rs generic cognition to Alisa's specific personality.

    use serde::{Deserialize, Serialize};

    /// Alisa's operating modes (maps to ada-rs PresenceMode)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum AlisaMode {
        /// Passive listening, time announcements, greetings
        Ambient,
        /// Active conversation, humor, opinions
        Companion,
        /// Intimate mode — ONLY with ElevenLabs, NEVER with Piper Thorsten
        Special,
        /// Wake-word only
        Quiet,
        /// Special paused due to active phone call (Thomas-Proof)
        CallPause,
    }

    impl Default for AlisaMode {
        fn default() -> Self {
            Self::Ambient
        }
    }

    /// Alisa's state machine
    #[derive(Debug)]
    pub struct Alisa {
        mode: AlisaMode,
        call_active: bool,
        elevenlabs_available: bool,
        /// Previous mode before CallPause (for restore)
        mode_before_pause: Option<AlisaMode>,
    }

    impl Default for Alisa {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Alisa {
        pub fn new() -> Self {
            Self {
                mode: AlisaMode::Ambient,
                call_active: false,
                elevenlabs_available: false,
                mode_before_pause: None,
            }
        }

        pub fn mode(&self) -> AlisaMode {
            self.mode
        }

        pub fn is_call_active(&self) -> bool {
            self.call_active
        }

        /// Set ElevenLabs availability (checked at startup + periodically)
        pub fn set_elevenlabs_available(&mut self, available: bool) {
            self.elevenlabs_available = available;
        }

        /// Request mode change — enforces Thomas-Proof and Thorsten-Proof
        pub fn request_mode(&mut self, target: AlisaMode) -> Result<AlisaMode, &'static str> {
            // Thomas-Proof: no mode changes during active call
            if self.call_active && target != AlisaMode::CallPause {
                return Err("Call active — mode change blocked (Thomas-Proof)");
            }

            // Thorsten-Proof: Special requires ElevenLabs
            if target == AlisaMode::Special && !self.elevenlabs_available {
                return Err("Nicht mit Thorstens Stimme, Schatz.");
            }

            self.mode = target;
            Ok(self.mode)
        }

        /// Phone call started — Thomas-Proof activates
        pub fn on_call_start(&mut self) {
            self.call_active = true;
            if self.mode == AlisaMode::Special {
                self.mode_before_pause = Some(AlisaMode::Special);
                self.mode = AlisaMode::CallPause;
                tracing::info!("Thomas-Proof: SPECIAL paused, conversation cleared");
            }
        }

        /// Phone call ended — NO auto-resume (Stefan must say "Alisa, weitermachen")
        pub fn on_call_end(&mut self) {
            self.call_active = false;
            // Intentionally does NOT restore mode_before_pause.
            // Stefan must explicitly request mode change.
            if self.mode == AlisaMode::CallPause {
                self.mode = AlisaMode::Ambient;
                tracing::info!("Call ended, returned to Ambient (no auto-resume)");
            }
        }

        /// Explicit resume after call (Stefan says "Alisa, weitermachen")
        pub fn resume_after_call(&mut self) -> Result<AlisaMode, &'static str> {
            if self.call_active {
                return Err("Call still active");
            }
            if let Some(prev) = self.mode_before_pause.take() {
                self.request_mode(prev)
            } else {
                Ok(self.mode)
            }
        }

        /// Generate response — first checks call guard
        pub fn respond(&self, _input: &str) -> Option<String> {
            // Thomas-Proof: check call status FIRST
            if self.call_active {
                return None;
            }
            // TODO: Route to Grok/LM Studio/Codebook based on mode
            None
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_default_is_ambient() {
            let alisa = Alisa::new();
            assert_eq!(alisa.mode(), AlisaMode::Ambient);
        }

        #[test]
        fn test_thorsten_proof() {
            let mut alisa = Alisa::new();
            // ElevenLabs not available → Special blocked
            let err = alisa.request_mode(AlisaMode::Special);
            assert!(err.is_err());
            assert!(err.unwrap_err().contains("Thorsten"));
        }

        #[test]
        fn test_thorsten_proof_allows_with_elevenlabs() {
            let mut alisa = Alisa::new();
            alisa.set_elevenlabs_available(true);
            let result = alisa.request_mode(AlisaMode::Special);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), AlisaMode::Special);
        }

        #[test]
        fn test_thomas_proof_pauses_special() {
            let mut alisa = Alisa::new();
            alisa.set_elevenlabs_available(true);
            alisa.request_mode(AlisaMode::Special).unwrap();

            alisa.on_call_start();
            assert_eq!(alisa.mode(), AlisaMode::CallPause);
            assert!(alisa.is_call_active());
        }

        #[test]
        fn test_no_auto_resume_after_call() {
            let mut alisa = Alisa::new();
            alisa.set_elevenlabs_available(true);
            alisa.request_mode(AlisaMode::Special).unwrap();

            alisa.on_call_start();
            alisa.on_call_end();

            // Must NOT auto-resume to Special
            assert_eq!(alisa.mode(), AlisaMode::Ambient);
        }

        #[test]
        fn test_explicit_resume() {
            let mut alisa = Alisa::new();
            alisa.set_elevenlabs_available(true);
            alisa.request_mode(AlisaMode::Special).unwrap();

            alisa.on_call_start();
            alisa.on_call_end();

            // Explicit "Alisa, weitermachen"
            let result = alisa.resume_after_call();
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), AlisaMode::Special);
        }

        #[test]
        fn test_respond_blocked_during_call() {
            let mut alisa = Alisa::new();
            alisa.on_call_start();
            assert!(alisa.respond("test").is_none());
        }
    }
}
