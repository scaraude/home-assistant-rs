//! Ada Brain — Codebook Inference Engine
//!
//! bgz17/PCDVQ compressed weights for local LLM inference.
//! Runs on ARM64 (Pi 4, NEON) and x86 (AVX2/AVX-512/AMX).
//!
//! Not matrix multiplication — codebook lookup. O(1) per token.
//!
//! SIMD Tiers:
//!   AMX (Sapphire):  380,000 tok/s
//!   AVX-512:          10,000-50,000 tok/s
//!   AVX2:              3,000-10,000 tok/s
//!   NEON (Pi 4):         500-5,000 tok/s
//!   Scalar:               50-500 tok/s

pub mod codebook;
pub mod inference;

pub mod codebook {
    //! Codebook loader + mmap
    //! TODO: Load qkvgud codebooks from /opt/pi-audio/codebook/
}

pub mod inference {
    //! Token generation via codebook lookup
    //! TODO: SIMD-dispatched lookup kernel

    #[cfg(target_arch = "aarch64")]
    pub mod neon;

    #[cfg(target_arch = "x86_64")]
    pub mod x86;

    pub mod scalar;
}
