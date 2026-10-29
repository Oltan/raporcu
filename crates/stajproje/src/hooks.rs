//! Sonuç geldikten ve koşum bittikten sonra çalışan işler.

use crate::AppState;

/// Yeni sonuç geldi: canlı ekran güncellenir.
pub async fn after_results(_st: &AppState, _run_id: i64) {}

/// Koşum bitti: triage kuralları, Jira eşleşmesi, ...
pub async fn after_finish(_st: &AppState, _run_id: i64) {}
