//! orihouse fork: compaction-only model and reasoning effort (see ORIHOUSE.md, commit #1).
//!
//! `compact_model` / `compact_reasoning_effort` change only the step that sends the compaction
//! request. The thread keeps its own model for the replacement window and the following turns, and
//! upstream's fallback retries a failed compaction request with that thread model.

use std::sync::Arc;

use arc_swap::ArcSwap;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing::warn;

use super::session::Session;
use super::step_context::StepContext;
use super::step_settings::ResolvedStepSettings;

/// Returns the step that should send the compaction request: the given step unless compaction
/// settings are configured (and `skip` is false), in which case a speculative step for them.
pub(crate) async fn compaction_step_context(
    sess: &Arc<Session>,
    step_context: &Arc<StepContext>,
    skip: bool,
) -> Arc<StepContext> {
    let config = &step_context.turn.config;
    if skip || (config.compact_model.is_none() && config.compact_reasoning_effort.is_none()) {
        return Arc::clone(step_context);
    }
    let current = step_context.settings.model_info.slug.clone();
    let model = config
        .compact_model
        .as_deref()
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .unwrap_or(current.as_str())
        .to_string();
    let mut turn = step_context
        .turn
        .with_model(model.clone(), &sess.services.models_manager)
        .await;
    if let Some(effort) = config.compact_reasoning_effort.clone() {
        let supported = turn
            .model_info()
            .supported_reasoning_levels
            .iter()
            .any(|preset| preset.effort == effort);
        if !supported {
            warn!(
                model,
                ?effort,
                "compact_reasoning_effort is not supported; compacting with the thread settings"
            );
            return Arc::clone(step_context);
        }
        let mut selected = turn.initial_settings.selected().clone();
        selected.collaboration_mode = selected.collaboration_mode.with_updates(
            /*model*/ None,
            Some(Some(effort.clone())),
            /*developer_instructions*/ None,
        );
        let settings = Arc::new(ResolvedStepSettings::new(
            Arc::new(selected),
            Arc::clone(&turn.initial_settings.model_info),
            &turn.config.features,
        ));
        turn.session_telemetry = settings.telemetry(&step_context.turn.session_telemetry);
        turn.initial_settings = Arc::clone(&settings);
        turn.next_step_settings = ArcSwap::from(settings);
        Arc::make_mut(&mut turn.config).model_reasoning_effort = Some(effort);
    }
    // Compaction requests carry no tool calls, so MCP startup never needs to be cancelled here.
    match sess
        .capture_speculative_step_context(Arc::new(turn), &CancellationToken::new())
        .await
    {
        Ok(compaction) => {
            info!(
                model = %compaction.settings.model_info.slug,
                effort = ?compaction.settings.reasoning_effort(),
                thread_model = %current,
                "resolved compaction-only model settings"
            );
            compaction
        }
        Err(error) => {
            warn!(model, %error, "compaction step capture failed; compacting with the thread settings");
            Arc::clone(step_context)
        }
    }
}
