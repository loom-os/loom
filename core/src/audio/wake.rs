use crate::{
    audio::utils::{gen_id, now_ms},
    event::EventBus,
    proto::Event,
    QoSLevel, Result,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// Configuration for wake word detection driven by transcripts
#[derive(Clone, Debug)]
pub struct WakeWordConfig {
    /// Topic to listen for final transcripts
    pub transcript_topic: String,
    /// Topic to publish wake events
    pub wake_topic: String,
    /// Topic to publish user queries once armed
    pub query_topic: String,
    /// Wake phrases to match against
    pub phrases: Vec<String>,
    /// Maximum Levenshtein distance allowed for fuzzy match
    pub max_distance: usize,
    /// Enable matching phrases anywhere in the sentence (sliding window)
    pub match_anywhere: bool,
    /// Jaro-Winkler similarity threshold (0.0-1.0) as an additional fuzzy gate
    pub jaro_winkler_threshold: f64,
    /// Optional: minimum characters in query after removing wake phrase to consider same-utterance query
    pub min_query_chars: usize,
}

impl Default for WakeWordConfig {
    fn default() -> Self {
        let phrases_env = std::env::var("WAKE_PHRASES").unwrap_or_else(|_| "hey loom,loom".into());
        let phrases = phrases_env
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        Self {
            transcript_topic: std::env::var("STT_TRANSCRIPT_TOPIC")
                .unwrap_or_else(|_| "transcript".into()),
            wake_topic: std::env::var("WAKE_TOPIC").unwrap_or_else(|_| "wake".into()),
            query_topic: std::env::var("QUERY_TOPIC").unwrap_or_else(|_| "query".into()),
            phrases,
            max_distance: std::env::var("WAKE_FUZZY_DISTANCE")
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(1),
            match_anywhere: std::env::var("WAKE_MATCH_ANYWHERE")
                .ok()
                .map(|s| matches!(s.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
                .unwrap_or(true),
            jaro_winkler_threshold: std::env::var("WAKE_JW_THRESHOLD")
                .ok()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.90),
            min_query_chars: std::env::var("WAKE_MIN_QUERY_CHARS")
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(4),
        }
    }
}

pub struct WakeWordDetector {
    bus: Arc<EventBus>,
    cfg: WakeWordConfig,
    // When Some(session_id), the next transcript will be treated as user query
    armed_session: Arc<Mutex<Option<String>>>,
}

impl WakeWordDetector {
    pub fn new(bus: Arc<EventBus>, cfg: WakeWordConfig) -> Self {
        Self {
            bus,
            cfg,
            armed_session: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start(self) -> Result<JoinHandle<()>> {
        let bus = Arc::clone(&self.bus);
        let cfg = self.cfg.clone();
        let armed = Arc::clone(&self.armed_session);

        // Subscribe to transcripts
        let (_id, mut rx) = bus
            .subscribe(
                cfg.transcript_topic.clone(),
                vec!["transcript.final".into()],
                QoSLevel::QosRealtime,
            )
            .await?;

        let handle = tokio::spawn(async move {
            while let Some(ev) = rx.recv().await {
                // Extract transcript text
                let text = ev
                    .metadata
                    .get("text")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| String::from_utf8_lossy(&ev.payload).to_string());

                if text.trim().is_empty() {
                    continue;
                }

                let text_norm = normalize(&text);

                // Check if we're already armed for a session
                // Take the session id and drop the lock immediately to avoid deadlocks
                let armed_session: Option<String> = {
                    let mut guard = armed.lock().await;
                    guard.take()
                };
                if let Some(session_id) = armed_session {
                    // Treat this transcript as the user's query
                    let mut md = HashMap::new();
                    md.insert("session_id".into(), session_id.clone());
                    md.insert("text".into(), text.clone());

                    let query_event = Event {
                        id: gen_id(),
                        r#type: "user.query".into(),
                        timestamp_ms: now_ms(),
                        source: "wake".into(),
                        metadata: md,
                        payload: text.into_bytes(),
                        confidence: 1.0,
                        tags: vec![],
                        priority: 60,
                    };

                    if let Err(e) = bus.publish(&cfg.query_topic, query_event).await {
                        warn!("Failed to publish user.query: {}", e);
                    } else {
                        info!(target: "wake", "📨 Published user.query for armed session");
                    }

                    // Continue to next event
                    continue;
                }

                // Not armed: check for wake phrase
                if let Some((matched, remainder)) = detect_wake(&cfg, &text_norm) {
                    let session_id = format!("sess_{}", gen_id());

                    // Publish wake_word_detected immediately
                    let mut md = HashMap::new();
                    md.insert("phrase".into(), matched.clone());
                    md.insert("text".into(), text.clone());
                    md.insert("session_id".into(), session_id.clone());

                    let wake_event = Event {
                        id: gen_id(),
                        r#type: "wake_word_detected".into(),
                        timestamp_ms: now_ms(),
                        source: "wake".into(),
                        metadata: md,
                        payload: vec![],
                        confidence: 1.0,
                        tags: vec![],
                        priority: 65,
                    };

                    if let Err(e) = bus.publish(&cfg.wake_topic, wake_event).await {
                        warn!("Failed to publish wake_word_detected: {}", e);
                    } else {
                        info!(target: "wake", "🔔 Wake word detected: {}", matched);
                    }

                    // If there's meaningful remainder, treat it as immediate query; otherwise arm
                    if remainder.chars().filter(|c| !c.is_whitespace()).count()
                        >= cfg.min_query_chars
                    {
                        let mut qmd = HashMap::new();
                        qmd.insert("session_id".into(), session_id.clone());
                        qmd.insert("text".into(), remainder.clone());

                        let query_event = Event {
                            id: gen_id(),
                            r#type: "user.query".into(),
                            timestamp_ms: now_ms(),
                            source: "wake".into(),
                            metadata: qmd,
                            payload: remainder.into_bytes(),
                            confidence: 1.0,
                            tags: vec![],
                            priority: 60,
                        };

                        if let Err(e) = bus.publish(&cfg.query_topic, query_event).await {
                            warn!("Failed to publish user.query: {}", e);
                        } else {
                            info!(target: "wake", "📨 Published immediate user.query");
                        }
                    } else {
                        // Arm for the next utterance (lock only while setting)
                        {
                            let mut guard = armed.lock().await;
                            *guard = Some(session_id);
                        }
                        debug!(target: "wake", "Armed for next utterance as query");
                    }
                }
            }
        });

        Ok(handle)
    }
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Detect wake phrase and return (matched_phrase, remainder_after_phrase)
fn detect_wake(cfg: &WakeWordConfig, text_norm: &str) -> Option<(String, String)> {
    use strsim::{jaro_winkler, levenshtein};

    let text_tokens: Vec<&str> = text_norm.split_whitespace().collect();
    if text_tokens.is_empty() {
        return None;
    }

    // Helper: compute phrase match score at a given position
    let matches_at = |p_tokens: &[&str], start: usize| -> bool {
        if start + p_tokens.len() > text_tokens.len() {
            return false;
        }
        let mut total_dist = 0usize;
        let mut jw_sum = 0.0f64;
        for (i, p_tok) in p_tokens.iter().enumerate() {
            let t_tok = text_tokens[start + i];
            total_dist += levenshtein(t_tok, p_tok);
            jw_sum += jaro_winkler(t_tok, p_tok);
        }
        let avg_jw = jw_sum / (p_tokens.len() as f64);
        let per_token_ok = p_tokens
            .iter()
            .enumerate()
            .all(|(i, p_tok)| levenshtein(text_tokens[start + i], p_tok) <= cfg.max_distance);
        per_token_ok
            || total_dist <= cfg.max_distance * p_tokens.len()
            || avg_jw >= cfg.jaro_winkler_threshold
    };

    for phrase in &cfg.phrases {
        let p = normalize(phrase);
        if p.is_empty() {
            continue;
        }
        let p_tokens: Vec<&str> = p.split_whitespace().collect();

        // 1) Prefer a start-of-utterance match (robust to small distance)
        if matches_at(&p_tokens, 0) {
            let remainder_tokens = if p_tokens.len() < text_tokens.len() {
                &text_tokens[p_tokens.len()..]
            } else {
                &[][..]
            };
            let remainder = remainder_tokens.join(" ");
            return Some((phrase.clone(), remainder));
        }

        // 2) Optionally allow match anywhere (sliding window)
        if cfg.match_anywhere {
            // For single-word phrases, find the closest word anywhere
            if p_tokens.len() == 1 {
                let p_tok = p_tokens[0];
                let mut best: Option<(usize, usize, f64)> = None; // (pos, dist, jw)
                for (i, t_tok) in text_tokens.iter().enumerate() {
                    let d = levenshtein(t_tok, p_tok);
                    let jw = jaro_winkler(t_tok, p_tok);
                    if d <= cfg.max_distance || jw >= cfg.jaro_winkler_threshold {
                        let better = match best {
                            None => true,
                            Some((_, bd, bjw)) => d < bd || (d == bd && jw > bjw),
                        };
                        if better {
                            best = Some((i, d, jw));
                        }
                    }
                }
                if let Some((pos, _, _)) = best {
                    let remainder = if pos + 1 < text_tokens.len() {
                        text_tokens[pos + 1..].join(" ")
                    } else {
                        String::new()
                    };
                    return Some((phrase.clone(), remainder));
                }
            } else {
                for start in 1..=text_tokens.len().saturating_sub(p_tokens.len()) {
                    if matches_at(&p_tokens, start) {
                        let remainder = if start + p_tokens.len() < text_tokens.len() {
                            text_tokens[start + p_tokens.len()..].join(" ")
                        } else {
                            String::new()
                        };
                        return Some((phrase.clone(), remainder));
                    }
                }
            }
        }
    }

    None
}
