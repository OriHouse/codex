//! orihouse fork: `compact_model` / `compact_reasoning_effort` change only the compaction request.

use anyhow::Result;
use codex_core::TurnInputRequest;
use codex_login::CodexAuth;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use core_test_support::responses;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::TestCodexHarness;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use test_case::test_case;

const SUMMARY: &str = "ORIHOUSE_COMPACT_SUMMARY";

fn model_and_effort(body: &Value) -> Value {
    json!([body["model"], body["reasoning"]["effort"]])
}

async fn harness(compact_effort: ReasoningEffort) -> Result<TestCodexHarness> {
    TestCodexHarness::with_auto_env_builder(
        test_codex()
            .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
            .with_model("gpt-6-astra")
            .with_config(move |config| {
                config.compact_model = Some("gpt-6.1-terra".into());
                config.compact_reasoning_effort = Some(compact_effort);
                config.model_reasoning_effort = Some(ReasoningEffort::High);
                config.model_auto_compact_token_limit = Some(200_000);
            }),
    )
    .await
}

async fn submit(harness: &TestCodexHarness, text: &str) -> Result<()> {
    let codex = &harness.test().codex;
    codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: text.into(),
            text_elements: Vec::new(),
        }]))
        .await?;
    wait_for_event(codex, |e| matches!(e, EventMsg::TurnComplete(_))).await;
    Ok(())
}

#[test_case("manual"; "manual")]
#[test_case("auto"; "auto")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_model_is_request_scoped(trigger: &str) -> Result<()> {
    skip_if_no_network!(Ok(()));
    let automatic = trigger == "auto";
    let harness = harness(ReasoningEffort::Low).await?;
    let requests = responses::mount_sse_sequence(
        harness.server(),
        vec![
            responses::sse(vec![
                responses::ev_assistant_message("m1", "BEFORE_COMPACT"),
                responses::ev_completed_with_tokens("r1", if automatic { 300_000 } else { 50 }),
            ]),
            responses::sse(vec![
                json!({"type": "response.output_item.done", "item": {"type": "compaction", "encrypted_content": SUMMARY}}),
                responses::ev_completed("rc"),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("m2", "AFTER_COMPACT"),
                responses::ev_completed("r2"),
            ]),
        ],
    )
    .await;
    submit(&harness, "Remember the task and reply briefly").await?;
    if !automatic {
        let codex = &harness.test().codex;
        codex.submit(Op::Compact).await?;
        wait_for_event(codex, |e| matches!(e, EventMsg::TurnComplete(_))).await;
    }
    submit(&harness, "Continue after the compact").await?;

    let bodies = requests
        .requests()
        .iter()
        .map(|request| request.body_json())
        .collect::<Vec<_>>();
    assert_eq!(bodies.len(), 3);
    assert_eq!(
        model_and_effort(&bodies[1]),
        json!(["gpt-6.1-terra", "low"])
    );
    for body in [&bodies[0], &bodies[2]] {
        assert_eq!(model_and_effort(body), json!(["gpt-6-astra", "high"]));
    }
    assert!(bodies[2].to_string().contains(SUMMARY));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_compact_effort_compacts_with_thread_settings() -> Result<()> {
    skip_if_no_network!(Ok(()));
    let harness = harness(ReasoningEffort::Custom("invalid-effort".into())).await?;
    let requests = responses::mount_sse_sequence(
        harness.server(),
        vec![
            responses::sse(vec![
                responses::ev_assistant_message("m1", "HISTORY_MUST_SURVIVE"),
                responses::ev_completed("r1"),
            ]),
            responses::sse(vec![
                json!({"type": "response.output_item.done", "item": {"type": "compaction", "encrypted_content": SUMMARY}}),
                responses::ev_completed("rc"),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("m2", "CONTINUED"),
                responses::ev_completed("r2"),
            ]),
        ],
    )
    .await;
    submit(&harness, "Remember this task").await?;
    let codex = &harness.test().codex;
    codex.submit(Op::Compact).await?;
    wait_for_event(codex, |e| matches!(e, EventMsg::TurnComplete(_))).await;
    submit(&harness, "Continue the original task").await?;

    let bodies = requests
        .requests()
        .iter()
        .map(|request| request.body_json())
        .collect::<Vec<_>>();
    assert_eq!(bodies.len(), 3);
    for body in &bodies {
        assert_eq!(model_and_effort(body), json!(["gpt-6-astra", "high"]));
    }
    assert!(bodies[2].to_string().contains(SUMMARY));
    Ok(())
}
