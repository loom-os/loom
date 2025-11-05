# Personalized Wake Word (Enrollment-based)

This plan proposes a robust, privacy-first wake mechanism that combines
(1) pronunciation similarity for the specific wake phrases ("hey loom" / "loom")
with (2) speaker verification of the enrolled user. It improves both UX and security
compared to transcript-only or generic keyword spotting.

## Goals

- Fast, reliable wake on the user’s own voice for phrases: "hey loom", "loom".
- Minimize false accepts from background speech or other people’s voices.
- Keep all data local; no network required.
- Fall back gracefully to transcript-based wake if models are missing.

## High-level approach

Two-stage decision (both should pass):

1. Personalized Keyword Match (KWS-personalized)

   - Compare incoming audio against the user’s enrolled samples for each phrase.
   - Use frame-level acoustic features (MFCC/log-Mel) + DTW (Dynamic Time Warping) or a tiny ONNX keyword model.
   - Output a similarity score per phrase.

2. Speaker Verification (SV)
   - Compute a speaker embedding (e.g., ECAPA-TDNN or x-vector via ONNX).
   - Compare incoming utterance embedding vs. enrolled embedding(s) with cosine similarity.
   - Output a similarity score per speaker.

Wake triggers when: KWS score >= KWS threshold AND SV score >= SV threshold.

Fallback: If models/embeddings not available, revert to the existing transcript-based wake logic.

## UX – Enrollment

- User records several short utterances for each phrase:
  - "hey loom" x N (e.g., 5–10)
  - "loom" x N (e.g., 5–10)
- We apply VAD to trim leading/trailing silence, normalize gain, and store:
  - The raw WAV (PCM16 mono @ 16 kHz),
  - Extracted features for KWS (Mel/MFCC sequences),
  - Speaker embedding(s) for each utterance, plus an averaged embedding.
- Provide immediate feedback if a sample is too noisy or too short.
- Allow re-enrollment at any time.

CLI UX (to be implemented):

```bash
# List devices and pick one
MIC_LOG_DEVICES=1 cargo run -p loom-core --example wake_enroll --features mic,vad

# Enroll default phrases to default profile
cargo run -p loom-core --example wake_enroll --features mic,vad \
  -- --phrases "hey loom,loom" --utterances 6 --profile default

# Verify enrollment by a quick test
cargo run -p loom-core --example wake_verify --features mic,vad \
  -- --profile default
```

## Runtime detection pipeline

- Always-on VAD gates audio and forms candidate segments (~300–1200 ms typical).
- For each candidate segment:
  1. Compute Mel/MFCC; score vs. enrolled templates (DTW distance or KWS model score).
  2. Compute speaker embedding; score vs. enrolled average embedding.
  3. If both scores exceed thresholds, emit `wake.verified` event with session.
  4. If only one score passes or both are marginal, emit `wake.candidate` with scores (for debugging/telemetry) but do not wake.
- On success, continue with current pipeline (start a session, route next utterance as `user.query`).

## Data & models

- Features: Mel filterbanks or MFCC (13–40 dims) @ 25 ms frame / 10 ms hop.
- KWS:
  - P0: DTW over MFCC sequences against the user’s enrolled phrase templates (fast, no NN model).
  - P1: Tiny ONNX keyword model (optional) fine-tuned on the enrolled phrases for better robustness.
- SV model: ONNX ECAPA-TDNN (or x-vector) to get a 192–256D embedding; cosine similarity for scoring.
- Inference runtime: `onnxruntime` crate (or `tract-onnx`) with CPU backend by default.

## Storage layout

- Base dir (XDG compliant):
  - Linux: `~/.local/share/loom/wake/`
  - macOS: `~/Library/Application Support/loom/wake/`
  - Windows: `%APPDATA%/loom/wake/`
- Per-profile folder: `<base>/<profile_id>/`
  - `profile.json` (metadata: phrases, thresholds, created_at, device hints)
  - `samples/` raw WAV files (e.g., `hey-loom_001.wav`)
  - `features/` serialized MFCC sequences (e.g., `.npy` or CBOR)
  - `embeddings/` per-utterance speaker vectors + `avg.vec`
- Optional encryption at rest for `features/` and `embeddings/` (see Security).

## Matching & thresholds

- KWS score:
  - DTW distance normalized by path length; lower is better.
  - Convert to similarity in [0,1] by `sim = exp(-alpha * dist_norm)`.
  - Threshold default: `WAKE_KWS_THRESHOLD = 0.75` (tune per environment).
- SV score:
  - Cosine similarity in [-1,1]; map to [0,1] by `(cos+1)/2`.
  - Threshold default: `WAKE_SPKR_THRESHOLD = 0.80` (tune per environment).
- Multi-sample aggregation:
  - Use min(DTW) across templates or top-k average.
  - Use average embedding across utterances for SV.
- Optional safety net: require k-of-n consecutive positives or short temporal smoothing to reduce bursts of false accepts.

## Events (contract)

- `wake.enroll.start` – user begins enrolling (metadata: profile_id, phrases)
- `wake.enroll.sample` – one sample captured (metadata: phrase, quality_metrics)
- `wake.enroll.complete` – enrollment finished (metadata: counts, quality)
- `wake.candidate` – runtime candidate with scores (metadata: kws_score, sv_score)
- `wake.verified` – wake accepted (metadata: session_id, phrase, kws_score, sv_score)
- `wake.rejected` – candidate rejected (metadata: reason/scores)

## Configuration (ENV)

Enrollment & profile:

- `WAKE_ENROLL_DIR` – base dir for profiles (default: XDG path above)
- `WAKE_ENROLL_PHRASES` – phrases to enroll (default: "hey loom,loom")
- `WAKE_MIN_ENROLL_UTTERANCES` – per phrase min utterances (default: 5)
- `WAKE_PROFILE_ID` – active profile (default: `default`)

Models & thresholds:

- `WAKE_SPKR_MODEL_PATH` – ONNX path for speaker embedding model (optional in P0)
- `WAKE_KWS_MODEL_PATH` – ONNX path for keyword model (optional; P1)
- `WAKE_SPKR_THRESHOLD` – default 0.80
- `WAKE_KWS_THRESHOLD` – default 0.75

Audio & preprocessing:

- `WAKE_SR_HZ` – sample rate for enrollment and runtime (default: 16000)
- `WAKE_PREEMPHASIS` / `WAKE_MEL_BINS` / `WAKE_MFCC_DIM` – advanced tuning (optional)

Security & privacy:

- `WAKE_ENCRYPTION_KEY_PATH` – if set, encrypt `features/` and `embeddings/` at rest using AES-GCM
- `WAKE_PRIVACY_MODE` – if `strict`, delete raw WAV after feature/embedding extraction

Interop & fallbacks:

- `WAKE_USE_TRANSCRIPT_FALLBACK` – if `true`, fall back to transcript-based wake when models or profile missing (default: true)

## Security & privacy

- All enrollment data stays on-device.
- Offer encryption for features & embeddings; WAVs can be deleted in strict mode.
- Expose an easy UI to reset/delete profile.

## Implementation plan

P0 – Minimal personalized wake (DTW + averaged speaker embedding)

- Add enrollment example `wake_enroll` that records and saves N utterances per phrase.
- Extract MFCCs and store; compute speaker embeddings (if model available) else skip SV.
- Runtime: integrate a `WakePersonalized` module:
  - Listen to VAD-gated chunks, form candidates, compute DTW+SV, emit `wake.verified` on pass.
- Thresholds exposed via ENV; telemetry events for tuning.

P1 – ONNX models & better scoring

- Integrate ONNX ECAPA-TDNN for speaker embeddings; cache avg embedding.
- Optional small ONNX KWS model; backoff to DTW if missing.
- Add k-of-n smoothing and SNR-based quality gating.

P2 – UX polish & GUI

- GUI panel for enrollment with mic device picker, signal meter, and quality feedback.
- One-click re-enrollment; profile management; encryption toggle.
- Wizard flow for first-time setup.

## Risks & mitigations

- False accepts in noisy environments → increase thresholds; require k-of-n; add SNR gate.
- Model size/latency on low-end CPUs → keep models tiny; DTW as fallback.
- Enrollment quality variability → real-time guidance (min duration, avoid clipping); allow retakes.

## Notes

- This plan coexists with current transcript-based wake. We can run both and require
  either both-pass (strict) or KWS-only when SV model unavailable (degraded).
- We’ll log scores (anonymized) locally to help pick sane defaults.
