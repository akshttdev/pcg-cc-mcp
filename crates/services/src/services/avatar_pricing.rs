//! Cost pricing for the avatar engine pipelines.
//!
//! Two estimates we surface:
//!   - **profile pipeline**: 16 × `gpt-image-1` images/edits + 1 Claude vision
//!     pass to build the character bible. Each pipeline therefore costs an
//!     approximately fixed amount of upstream USD.
//!   - **motion clip**: 1 ElevenLabs TTS render + 1 Fal Omnihuman job. The
//!     ElevenLabs cost scales with script length (audio seconds), the
//!     Omnihuman cost is per-clip.
//!
//! The user-facing VIBE charge is **2× the upstream USD cost**, converted via
//! `VibePricingService::usd_to_vibe`. Hardcode prices here for v1; lift to
//! `SystemSettings` when upstream prices change for the first time.

use crate::services::vibe_pricing::VibePricingService;

// ─── Upstream unit prices (USD) ───────────────────────────────────────────────
//
// Sourced from the vendor's published pricing as of 2026-05-16. Update these
// when vendors change prices; commit + bump the planning doc.

/// `gpt-image-1` `images/edits` — per image at our default size.
const GPT_IMAGE_1_PER_IMAGE_USD: f64 = 0.04;

/// Claude 3.5 Sonnet vision pass for the bible — average input + output tokens.
const CLAUDE_BIBLE_USD: f64 = 0.01;

/// Fal Omnihuman flat per-clip price (averages out across short prompts).
const FAL_OMNIHUMAN_PER_CLIP_USD: f64 = 0.50;

/// ElevenLabs TTS per audio second (mid-tier voice).
const ELEVENLABS_PER_AUDIO_SECOND_USD: f64 = 0.005;

// ─── Constants ────────────────────────────────────────────────────────────────

/// Number of shots in the canonical SHOT_SLOTS taxonomy.
pub const PROFILE_SHOT_COUNT: usize = 16;

/// User-facing markup applied on top of upstream USD costs.
pub const VIBE_MARKUP_MULTIPLIER: f64 = 2.0;

// ─── Public API ───────────────────────────────────────────────────────────────

/// Upstream USD cost of running one `generate-profile` pipeline.
pub fn profile_pipeline_upstream_usd() -> f64 {
    (PROFILE_SHOT_COUNT as f64) * GPT_IMAGE_1_PER_IMAGE_USD + CLAUDE_BIBLE_USD
}

/// Upstream USD cost of rendering one motion clip for a given audio length.
pub fn motion_clip_upstream_usd(audio_seconds: f64) -> f64 {
    FAL_OMNIHUMAN_PER_CLIP_USD + audio_seconds.max(0.0) * ELEVENLABS_PER_AUDIO_SECOND_USD
}

/// VIBE cost (2× upstream USD, converted) for one `generate-profile` run.
pub fn profile_pipeline_vibe_cost() -> i64 {
    VibePricingService::usd_to_vibe(profile_pipeline_upstream_usd() * VIBE_MARKUP_MULTIPLIER)
}

/// VIBE cost for one motion clip. Audio length is an upper-bound estimate so
/// the pre-debit covers the worst case; settle refunds the delta.
pub fn motion_clip_vibe_cost(audio_seconds_estimate: f64) -> i64 {
    VibePricingService::usd_to_vibe(
        motion_clip_upstream_usd(audio_seconds_estimate) * VIBE_MARKUP_MULTIPLIER,
    )
}

/// Conservative audio-seconds estimate from a script. ElevenLabs TTS averages
/// ~150 wpm = 2.5 wps. Add a 20% safety margin so the pre-debit doesn't fall
/// short for slow-speaking voice models.
pub fn estimate_audio_seconds(script_text: &str) -> f64 {
    let word_count = script_text.split_whitespace().count();
    let base_seconds = word_count as f64 / 2.5;
    base_seconds * 1.2
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_upstream_is_sixteen_shots_plus_bible() {
        let upstream = profile_pipeline_upstream_usd();
        let expected = 16.0 * GPT_IMAGE_1_PER_IMAGE_USD + CLAUDE_BIBLE_USD;
        assert!((upstream - expected).abs() < 1e-9);
    }

    #[test]
    fn motion_clip_upstream_scales_with_audio_seconds() {
        let zero = motion_clip_upstream_usd(0.0);
        let ten = motion_clip_upstream_usd(10.0);
        assert!(ten > zero);
        assert!((ten - zero - 10.0 * ELEVENLABS_PER_AUDIO_SECOND_USD).abs() < 1e-9);
    }

    #[test]
    fn negative_audio_seconds_treated_as_zero() {
        assert_eq!(
            motion_clip_upstream_usd(-5.0),
            motion_clip_upstream_usd(0.0)
        );
    }

    #[test]
    fn vibe_cost_is_double_upstream_in_vibe_units() {
        let upstream_vibe = VibePricingService::usd_to_vibe(profile_pipeline_upstream_usd());
        let marked_up_vibe = profile_pipeline_vibe_cost();
        // 2x markup on USD then convert — at minimum should be ~2x the
        // single-markup value (within 1 unit due to ceil()).
        assert!(marked_up_vibe >= upstream_vibe * 2 - 1);
        assert!(marked_up_vibe <= upstream_vibe * 2 + 1);
    }

    #[test]
    fn audio_estimate_grows_with_word_count() {
        let short = estimate_audio_seconds("hello world");
        let long = estimate_audio_seconds("one two three four five six seven eight nine ten");
        assert!(long > short);
    }
}
