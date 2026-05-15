pub mod acp;
mod json;
pub mod prompts;
pub mod providers;
pub mod response;
pub mod skills;
pub mod types;

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::{
        env, fs,
        path::{Path, PathBuf},
        process,
        sync::mpsc,
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use camino::Utf8PathBuf;

    use agent_client_protocol::schema::{SessionNotification, SessionUpdate};

    use super::{
        acp::{
            run_acp_agent, run_acp_agent_json, AcpConversation, AcpPermissionOptionKind,
            AcpPermissionPolicy, AcpPermissionRequest, AcpPromptOptions, AcpSessionEvent,
        },
        providers::{
            detect_local_acp_agent_in_path, known_acp_providers, parse_acp_agent_args,
            resolve_local_acp_agent_command_in_path, LocalAcpAgentCommand, ACP_AGENT_ENV,
        },
        response::{
            candidate_metadata_from_response, parse_agent_response_text, validate_draft_response,
        },
        types::{
            AgentIntent, AgentProjectSnapshot, AgentProvenance, AgentRequestEnvelope,
            AgentResponseEnvelope, Confidence,
        },
    };
    use crate::{
        error::DomainErrorKind,
        project::{DiscoverySource, ProjectContext},
    };

    #[test]
    fn acp_usage_update_session_notification_deserializes() {
        let notification: SessionNotification = serde_json::from_value(serde_json::json!({
            "sessionId": "019dfbb8-6904-7153-9318-f20ced4ba48a",
            "update": {
                "sessionUpdate": "usage_update",
                "size": 258400,
                "used": 17465
            }
        }))
        .expect("usage_update should be a supported ACP session update");

        match notification.update {
            SessionUpdate::UsageUpdate(update) => {
                assert_eq!(update.size, 258400);
                assert_eq!(update.used, 17465);
            }
            other => panic!("expected usage_update, got {other:?}"),
        }
    }

    #[test]
    fn known_provider_registry_includes_command_specific_args() {
        let providers = known_acp_providers();

        assert!(providers
            .iter()
            .any(|provider| provider.id == "codex-acp" && provider.command_line() == "codex-acp"));
        assert!(providers.iter().any(|provider| {
            provider.id == "gemini-acp" && provider.command_line() == "gemini --acp"
        }));
        assert!(providers.iter().any(|provider| {
            provider.id == "cursor-acp" && provider.command_line() == "cursor-agent acp"
        }));
    }

    #[test]
    fn detects_first_available_known_provider_from_path() {
        let root = env::temp_dir().join(format!(
            "dadrock-agent-detect-{}-{}",
            process::id(),
            "known-provider"
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp provider directory");
        let program = root.join("codex-acp");
        fs::write(&program, "#!/bin/sh\nexit 0\n").expect("write fake provider");
        make_executable(&program);
        let path = env::join_paths([root.as_path()]).expect("join test PATH");

        let detected =
            detect_local_acp_agent_in_path(Some(path.as_os_str())).expect("detect provider");

        assert_eq!(detected.provider.id, "codex-acp");
        assert_eq!(detected.command.program, program.to_string_lossy().as_ref());
        assert!(detected.command.args.is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn detection_uses_provider_args() {
        let root = env::temp_dir().join(format!(
            "dadrock-agent-detect-{}-{}",
            process::id(),
            "provider-args"
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp provider directory");
        let program = root.join("gemini");
        fs::write(&program, "#!/bin/sh\nexit 0\n").expect("write fake provider");
        make_executable(&program);
        let path = env::join_paths([root.as_path()]).expect("join test PATH");

        let detected =
            detect_local_acp_agent_in_path(Some(path.as_os_str())).expect("detect provider");

        assert_eq!(detected.provider.id, "gemini-acp");
        assert_eq!(detected.command.program, program.to_string_lossy().as_ref());
        assert_eq!(detected.command.args, vec!["--acp".to_owned()]);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn explicit_agent_command_resolves_executable_from_path() {
        let root = env::temp_dir().join(format!(
            "dadrock-agent-detect-{}-{}",
            process::id(),
            "explicit-command"
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp provider directory");
        let program = root.join("custom-acp");
        fs::write(&program, "#!/bin/sh\nexit 0\n").expect("write fake provider");
        make_executable(&program);
        let path = env::join_paths([root.as_path()]).expect("join test PATH");

        let command = resolve_local_acp_agent_command_in_path(
            "custom-acp".to_owned(),
            vec!["--stdio".to_owned()],
            "--agent",
            Some(path.as_os_str()),
        )
        .expect("resolve explicit provider");

        assert_eq!(command.program, program.to_string_lossy().as_ref());
        assert_eq!(command.args, vec!["--stdio".to_owned()]);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn explicit_agent_command_reports_missing_executable() {
        let error = resolve_local_acp_agent_command_in_path(
            "missing-acp".to_owned(),
            Vec::new(),
            ACP_AGENT_ENV,
            None,
        )
        .expect_err("missing provider should be rejected");
        let message = error.to_string();

        assert!(message.contains("DADROCK_ACP_AGENT points to `missing-acp`"));
        assert!(message.contains("not executable or not on PATH"));
        assert!(message.contains("unset DADROCK_ACP_AGENT to allow auto-detection"));
        assert!(message.contains("Known provider commands"));
    }

    #[test]
    fn explicit_agent_command_reports_inline_arguments() {
        let error = resolve_local_acp_agent_command_in_path(
            "gemini --acp".to_owned(),
            Vec::new(),
            ACP_AGENT_ENV,
            None,
        )
        .expect_err("inline args should be rejected");
        let message = error.to_string();

        assert!(message.contains("must contain only the ACP executable"));
        assert!(message.contains("DADROCK_ACP_AGENT_ARGS"));
        assert!(message.contains("DADROCK_ACP_AGENT=gemini"));
    }

    #[test]
    fn acp_agent_args_parse_shell_words() {
        let args = parse_acp_agent_args(r#"--model "gpt 5" --flag 'quoted value'"#)
            .expect("parse shell words");

        assert_eq!(
            args,
            vec![
                "--model".to_owned(),
                "gpt 5".to_owned(),
                "--flag".to_owned(),
                "quoted value".to_owned()
            ]
        );
    }

    #[test]
    fn acp_agent_args_report_unclosed_quotes() {
        let error =
            parse_acp_agent_args(r#"--model "gpt 5"#).expect_err("unclosed quote should fail");

        let message = error
            .domain_message(DomainErrorKind::Acp)
            .expect("expected ACP error");
        assert!(message.contains("DADROCK_ACP_AGENT_ARGS"));
        assert!(message.contains("missing closing quote"));
    }

    #[test]
    fn parses_final_agent_json_after_prelude_objects() {
        let response = parse_agent_response_text(
            r###"
Here is a small example {"intent":"draft_candidate","body_markdown":"not the answer"}.

Final answer:
{"intent":"draft_candidate","confidence":"medium","summary":"Drafted.","frontmatter":{"title":"Use indexes"},"body_markdown":"## Context\nRecords are scanned repeatedly.\n\n## Decision\nUse an index.\n\n## Alternatives\nKeep rescanning.\n\n## Consequences\nLink checks are faster.","source_references":[],"activity_observations":[],"warnings":[],"provenance":{"model":"local-test","tool_runs":[]}}
"###,
        )
        .expect("final JSON object should parse");

        assert_eq!(response.intent, AgentIntent::DraftCandidate);
        assert_eq!(response.summary, "Drafted.");
    }

    #[test]
    fn parses_later_fenced_agent_json() {
        let response = parse_agent_response_text(
            r###"
```json
{"tool":"example"}
```

```json
{"intent":"draft_candidate","confidence":"high","summary":"Drafted.","frontmatter":{"title":"Use fenced JSON"},"body_markdown":"## Context\nAgents may emit multiple fences.\n\n## Decision\nParse the answer fence.\n\n## Alternatives\nParse the first fence.\n\n## Consequences\nResponses are more robust.","source_references":[],"activity_observations":[],"warnings":[],"provenance":{"model":"local-test","tool_runs":[]}}
```
"###,
        )
        .expect("later answer fence should parse");

        assert_eq!(response.intent, AgentIntent::DraftCandidate);
        assert_eq!(response.confidence, Confidence::High);
    }

    #[test]
    fn parses_agent_json_with_partial_source_reference() {
        let response = parse_agent_response_text(
            r###"
{"intent":"draft_candidate","confidence":"medium","summary":"Drafted.","frontmatter":{"title":"Use partial source refs"},"body_markdown":"## Context\nContext.\n\n## Decision\nDecision.\n\n## Alternatives\nAlternatives.\n\n## Consequences\nConsequences.","source_references":[{"excerpt":"source excerpt without a path"}],"activity_observations":[],"warnings":[],"provenance":{"model":"local-test","tool_runs":[]}}
"###,
        )
        .expect("partial source references should not reject the draft");

        assert_eq!(response.source_references.len(), 1);
        assert_eq!(response.source_references[0].path, None);
        assert_eq!(
            response.source_references[0].excerpt.as_deref(),
            Some("source excerpt without a path")
        );
    }

    #[test]
    fn parses_agent_json_with_string_activity_observation() {
        let response = parse_agent_response_text(
            r###"
{"intent":"draft_candidate","confidence":"medium","summary":"Drafted.","frontmatter":{"title":"Use string activity observation"},"body_markdown":"## Context\nContext.\n\n## Decision\nDecision.\n\n## Alternatives\nAlternatives.\n\n## Consequences\nConsequences.","source_references":[],"activity_observations":["analysis/penguins_ols.py already reads from the Rdatasets URL (line 11, 15)"],"warnings":[],"provenance":{"model":"local-test","tool_runs":[]}}
"###,
        )
        .expect("string activity observations should not reject the draft");

        assert_eq!(response.activity_observations.len(), 1);
        assert_eq!(response.activity_observations[0].activity, "observation");
        assert_eq!(
            response.activity_observations[0].summary,
            "analysis/penguins_ols.py already reads from the Rdatasets URL (line 11, 15)"
        );
        assert_eq!(response.activity_observations[0].path, None);
    }

    #[test]
    fn parses_agent_json_with_string_tool_run() {
        let response = parse_agent_response_text(
            r###"
{"intent":"draft_candidate","confidence":"medium","summary":"Drafted.","frontmatter":{"title":"Use string tool run"},"body_markdown":"## Context\nContext.\n\n## Decision\nDecision.\n\n## Alternatives\nAlternatives.\n\n## Consequences\nConsequences.","source_references":[],"activity_observations":[],"warnings":[],"provenance":{"model":"local-test","tool_runs":["dadrock_dadrock_project_snapshot"]}}
"###,
        )
        .expect("string tool runs should not reject the draft");

        assert_eq!(response.provenance.tool_runs.len(), 1);
        assert_eq!(
            response.provenance.tool_runs[0].name,
            "dadrock_dadrock_project_snapshot"
        );
        assert_eq!(response.provenance.tool_runs[0].summary, None);
    }

    #[test]
    fn rejects_truncated_agent_json_with_brace_inside_string() {
        let error = parse_agent_response_text(
            r#"{"intent":"draft_candidate","body_markdown":"unterminated }"#,
        )
        .expect_err("truncated JSON should not be extracted");

        let message = error
            .domain_message(DomainErrorKind::Acp)
            .expect("expected ACP error");
        assert!(message.contains("JSON object"));
    }

    #[test]
    fn response_tags_are_deduplicated_by_normalized_tag() {
        let mut response = valid_agent_response();
        response.frontmatter = serde_json::json!({
            "tags": [
                "origin/agent",
                "#origin/agent",
                "kind/format",
                "Kind/Format",
                "origin/agent/local-test"
            ]
        });
        response.provenance.model = Some("local test".to_owned());

        let tags = candidate_metadata_from_response(&mut response)
            .expect("candidate metadata")
            .tags;

        assert_eq!(tags.iter().filter(|tag| *tag == "origin/agent").count(), 1);
        assert_eq!(
            tags.iter()
                .filter(|tag| tag.eq_ignore_ascii_case("kind/format"))
                .count(),
            1
        );
        assert_eq!(
            tags.iter()
                .filter(|tag| *tag == "origin/agent/local-test")
                .count(),
            1
        );
    }

    #[test]
    fn response_title_summary_fallback_records_warning() {
        let mut response = valid_agent_response();
        response.frontmatter = serde_json::json!({});
        response.body_markdown = "## Context\nContext.\n\n## Decision\nDecision.\n\n## Alternatives\nAlternatives.\n\n## Consequences\nConsequences.\n".to_owned();

        let metadata = candidate_metadata_from_response(&mut response).expect("metadata");

        assert_eq!(metadata.title, "Drafted.");
        assert!(response
            .warnings
            .iter()
            .any(|warning| warning.contains("using summary as candidate title")));
    }

    #[test]
    fn candidate_response_validation_requires_dadr_sections() {
        let mut response = valid_agent_response();
        response.body_markdown = "## Context\nOnly context.\n".to_owned();

        let error = validate_draft_response(&response).unwrap_err();

        let message = error
            .domain_message(DomainErrorKind::Acp)
            .expect("expected ACP error");
        assert!(
            message.contains("missing required section(s): Decision, Alternatives, Consequences")
        );
    }

    #[test]
    fn candidate_response_validation_rejects_dadrock_owned_frontmatter() {
        let mut response = valid_agent_response();
        response.frontmatter = serde_json::json!({
            "title": "Use test response",
            "id": "01938b8e-4d70-7abc-8def-0123456789ab"
        });

        let error = validate_draft_response(&response).unwrap_err();

        let message = error
            .domain_message(DomainErrorKind::Acp)
            .expect("expected ACP error");
        assert!(message.contains("must not include `id`"));
    }

    #[test]
    fn candidate_response_validation_rejects_non_candidate_status() {
        let mut response = valid_agent_response();
        response.frontmatter = serde_json::json!({
            "title": "Use test response",
            "status": "accepted"
        });

        let error = validate_draft_response(&response).unwrap_err();

        let message = error
            .domain_message(DomainErrorKind::Acp)
            .expect("expected ACP error");
        assert!(message.contains("status must be `candidate`"));
    }

    #[cfg(unix)]
    #[test]
    fn fake_acp_conversation_streams_chat_response() {
        let fixture = FakeAcpFixture::new("chat");
        let (events, event_receiver) = mpsc::channel();
        let conversation = AcpConversation::start_with_events(
            fixture.command(),
            fixture.root.clone(),
            Some(events),
        )
        .expect("start fake ACP conversation");
        wait_for_ready_event(&event_receiver);

        let response = conversation
            .handle()
            .expect("conversation handle")
            .prompt_text("hello".to_owned())
            .expect("fake chat response");

        assert_eq!(response, "fake chat answer");
    }

    #[cfg(unix)]
    #[test]
    fn fake_acp_conversation_round_trips_permission_selection() {
        let fixture = FakeAcpFixture::new("permission");
        let decision_log = fixture.decision_log();
        let (events, event_receiver) = mpsc::channel();
        let conversation = AcpConversation::start_with_events(
            fixture.command(),
            fixture.root.clone(),
            Some(events),
        )
        .expect("start fake ACP conversation");
        wait_for_ready_event(&event_receiver);
        let handle = conversation.handle().expect("conversation handle");
        let prompt_handle = handle.clone();
        let prompt_thread =
            thread::spawn(move || prompt_handle.prompt_text("needs tool".to_owned()));

        let request = wait_for_permission_request(&event_receiver);
        assert_eq!(request.message, "ACP agent requests permission for shell. Options offered: Allow once, Deny once, Always deny.");
        assert!(request
            .options
            .iter()
            .any(|option| option.id == "allow-once"
                && option.kind == AcpPermissionOptionKind::AllowOnce));

        handle
            .select_permission_option(request.request_id, "allow-once")
            .expect("select permission option");
        let response = prompt_thread
            .join()
            .expect("prompt thread")
            .expect("fake permission response");

        assert_eq!(response, "permission granted");
        let decision = fs::read_to_string(decision_log).expect("read fake ACP decision");
        assert!(decision.contains("\"optionId\":\"allow-once\""));
    }

    #[cfg(unix)]
    #[test]
    fn fake_acp_prompt_applies_noninteractive_permission_policy() {
        let fixture = FakeAcpFixture::new("draft-permission");
        let request = minimal_agent_request(&fixture.root);

        let response = run_acp_agent(
            &fixture.command(),
            &request,
            AcpPromptOptions {
                permission_policy: AcpPermissionPolicy::AllowOnce,
                ..AcpPromptOptions::default()
            },
        )
        .expect("run fake ACP agent");

        assert_eq!(response.summary, "Permission draft.");
        let decision = fs::read_to_string(fixture.decision_log()).expect("read fake decision");
        assert!(decision.contains("\"optionId\":\"allow-once\""));
    }

    #[cfg(unix)]
    #[test]
    fn fake_acp_prompt_enforces_output_limit() {
        let fixture = FakeAcpFixture::new("chat");
        let request = minimal_agent_request(&fixture.root);

        let error = run_acp_agent(
            &fixture.command(),
            &request,
            AcpPromptOptions {
                output_limit_bytes: 4,
                ..AcpPromptOptions::default()
            },
        )
        .expect_err("small output limit should fail");

        let message = error
            .domain_message(DomainErrorKind::Acp)
            .expect("expected ACP error");
        assert!(message.contains("output exceeded"));
    }

    #[cfg(unix)]
    #[test]
    fn fake_acp_prompt_reads_draft_from_tool_raw_output() {
        let fixture = FakeAcpFixture::new("draft-raw-output");
        let request = minimal_agent_request(&fixture.root);

        let response = run_acp_agent(&fixture.command(), &request, AcpPromptOptions::default())
            .expect("run fake ACP agent with structured raw output");

        assert_eq!(response.summary, "Raw output draft.");
        assert_eq!(
            response
                .frontmatter
                .get("title")
                .and_then(serde_json::Value::as_str),
            Some("Use ACP rawOutput")
        );
    }

    #[cfg(unix)]
    #[test]
    fn fake_acp_json_prompt_reads_kinded_tool_raw_output() {
        let fixture = FakeAcpFixture::new("draft-raw-output");
        let request = minimal_agent_request(&fixture.root);

        let response =
            run_acp_agent_json(&fixture.command(), &request, AcpPromptOptions::default())
                .expect("run fake ACP JSON prompt with structured raw output");

        assert_eq!(
            response.get("kind").and_then(serde_json::Value::as_str),
            Some("dadrock.draft_candidate.v1")
        );
        assert_eq!(
            response.get("summary").and_then(serde_json::Value::as_str),
            Some("Raw output draft.")
        );
    }

    #[cfg(unix)]
    struct FakeAcpFixture {
        root: PathBuf,
        script: PathBuf,
        mode: &'static str,
    }

    #[cfg(unix)]
    fn minimal_agent_request(root: &Path) -> AgentRequestEnvelope {
        let cwd = Utf8PathBuf::from_path_buf(root.to_path_buf()).expect("utf8 temp path");
        let project = ProjectContext {
            cwd: cwd.clone(),
            decisions_dir: cwd.join("dadr/accepted"),
            discovery_source: DiscoverySource::Fallback,
            initialized: true,
        };
        AgentRequestEnvelope::draft_candidate(
            "Draft a small candidate.".to_owned(),
            AgentProjectSnapshot::new(&project, Vec::new()),
        )
        .expect("agent request")
    }

    #[cfg(unix)]
    impl FakeAcpFixture {
        fn new(mode: &'static str) -> Self {
            let root = unique_temp_dir(format!("dadrock-fake-acp-{mode}"));
            fs::create_dir_all(&root).expect("create fake ACP temp dir");
            let script = root.join("fake-acp");
            fs::write(&script, fake_acp_agent_script()).expect("write fake ACP script");
            make_executable(&script);
            Self { root, script, mode }
        }

        fn command(&self) -> LocalAcpAgentCommand {
            LocalAcpAgentCommand::new(
                self.script.to_string_lossy().into_owned(),
                vec![
                    self.mode.to_owned(),
                    self.root
                        .join("fake-acp-log")
                        .to_string_lossy()
                        .into_owned(),
                ],
            )
        }

        fn decision_log(&self) -> PathBuf {
            self.root.join("fake-acp-log.decision")
        }
    }

    #[cfg(unix)]
    impl Drop for FakeAcpFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[cfg(unix)]
    fn wait_for_ready_event(receiver: &mpsc::Receiver<AcpSessionEvent>) {
        let deadline = SystemTime::now() + Duration::from_secs(5);
        loop {
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(AcpSessionEvent::Ready { .. }) => return,
                Ok(AcpSessionEvent::Failed { message }) => panic!("fake ACP failed: {message}"),
                Ok(_) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    panic!("fake ACP event channel disconnected")
                }
            }
            assert!(
                SystemTime::now() < deadline,
                "timed out waiting for ready event"
            );
        }
    }

    #[cfg(unix)]
    fn wait_for_permission_request(
        receiver: &mpsc::Receiver<AcpSessionEvent>,
    ) -> AcpPermissionRequest {
        let deadline = SystemTime::now() + Duration::from_secs(5);
        loop {
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(AcpSessionEvent::PermissionRequested(request)) => return request,
                Ok(AcpSessionEvent::Failed { message }) => panic!("fake ACP failed: {message}"),
                Ok(_) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    panic!("fake ACP event channel disconnected")
                }
            }
            assert!(
                SystemTime::now() < deadline,
                "timed out waiting for permission request"
            );
        }
    }

    fn valid_agent_response() -> AgentResponseEnvelope {
        AgentResponseEnvelope {
            intent: AgentIntent::DraftCandidate,
            confidence: Confidence::Medium,
            summary: "Drafted.".to_owned(),
            frontmatter: serde_json::json!({"title": "Use test response"}),
            body_markdown: "## Context\nContext.\n\n## Decision\nDecision.\n\n## Alternatives\nAlternatives.\n\n## Consequences\nConsequences.\n".to_owned(),
            source_references: Vec::new(),
            activity_observations: Vec::new(),
            warnings: Vec::new(),
            provenance: AgentProvenance {
                prompt_pack_sha256: None,
                model: Some("local-test".to_owned()),
                tool_runs: Vec::new(),
            },
        }
    }

    #[cfg(unix)]
    fn fake_acp_agent_script() -> &'static str {
        r###"#!/bin/sh
mode="$1"
log="$2"

send() {
  printf '%s\n' "$1"
}

message_id() {
  printf '%s' "$1" | sed -n 's/.*"id":\([^,}]*\).*/\1/p'
}

while IFS= read -r line; do
  printf '%s\n' "$line" >> "$log.in"
  id="$(message_id "$line")"
  case "$line" in
    *'"method":"initialize"'*)
      send "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"agentCapabilities\":{},\"authMethods\":[]}}"
      ;;
    *'"method":"session/new"'*)
      send "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"fake-session\",\"configOptions\":[]}}"
      ;;
    *'"method":"session/prompt"'*)
      if [ "$mode" = "permission" ] || [ "$mode" = "draft-permission" ]; then
        send "{\"jsonrpc\":\"2.0\",\"id\":9001,\"method\":\"session/request_permission\",\"params\":{\"sessionId\":\"fake-session\",\"toolCall\":{\"toolCallId\":\"tool-1\",\"title\":\"shell\"},\"options\":[{\"optionId\":\"allow-once\",\"name\":\"Allow once\",\"kind\":\"allow_once\"},{\"optionId\":\"deny-once\",\"name\":\"Deny once\",\"kind\":\"reject_once\"},{\"optionId\":\"deny-always\",\"name\":\"Always deny\",\"kind\":\"reject_always\"}]}}"
        if IFS= read -r decision; then
          printf '%s\n' "$decision" >> "$log.decision"
        fi
        if [ "$mode" = "draft-permission" ]; then
          send "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionId\":\"fake-session\",\"update\":{\"sessionUpdate\":\"agent_message_chunk\",\"content\":{\"type\":\"text\",\"text\":\"{\\\"intent\\\":\\\"draft_candidate\\\",\\\"confidence\\\":\\\"medium\\\",\\\"summary\\\":\\\"Permission draft.\\\",\\\"frontmatter\\\":{\\\"title\\\":\\\"Use permission policy\\\"},\\\"body_markdown\\\":\\\"## Context\\\\nContext.\\\\n\\\\n## Decision\\\\nDecision.\\\\n\\\\n## Alternatives\\\\nAlternatives.\\\\n\\\\n## Consequences\\\\nConsequences.\\\",\\\"source_references\\\":[],\\\"activity_observations\\\":[],\\\"warnings\\\":[],\\\"provenance\\\":{\\\"model\\\":\\\"fake-acp\\\",\\\"tool_runs\\\":[]}}\"}}}}"
        else
          send "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionId\":\"fake-session\",\"update\":{\"sessionUpdate\":\"agent_message_chunk\",\"content\":{\"type\":\"text\",\"text\":\"permission granted\"}}}}"
        fi
      elif [ "$mode" = "draft-raw-output" ]; then
        send "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionId\":\"fake-session\",\"update\":{\"sessionUpdate\":\"tool_call_update\",\"toolCallId\":\"emit-draft\",\"title\":\"Emit Dadrock draft\",\"kind\":\"other\",\"status\":\"completed\",\"rawOutput\":{\"kind\":\"dadrock.draft_candidate.v1\",\"intent\":\"draft_candidate\",\"confidence\":\"high\",\"summary\":\"Raw output draft.\",\"frontmatter\":{\"title\":\"Use ACP rawOutput\"},\"body_markdown\":\"## Context\\nText chunks are for narration.\\n\\n## Decision\\nRead Dadrock draft envelopes from ACP tool rawOutput when present.\\n\\n## Alternatives\\nContinue scraping final agent text.\\n\\n## Consequences\\nStructured ACP output can create candidates without relying on prose parsing.\",\"source_references\":[],\"activity_observations\":[],\"warnings\":[],\"provenance\":{\"model\":\"fake-acp\",\"tool_runs\":[{\"name\":\"emit-draft\"}]}}}}}"
      else
        send "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionId\":\"fake-session\",\"update\":{\"sessionUpdate\":\"agent_message_chunk\",\"content\":{\"type\":\"text\",\"text\":\"fake chat answer\"}}}}"
      fi
      send "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"stopReason\":\"end_turn\"}}"
      ;;
  esac
done
"###
    }

    fn unique_temp_dir(label: impl AsRef<str>) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        env::temp_dir().join(format!("{}-{}-{nanos}", label.as_ref(), process::id()))
    }

    fn make_executable(path: &Path) {
        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(path)
                .expect("read fake provider metadata")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).expect("mark fake provider executable");
        }
    }
}
