use dadrock::{
    agent::{acp::AcpPermissionPolicy, providers::LocalAcpAgentCommand},
    commands::{draft_with_acp_agent, AgentDraftCommandRequest, CommandContext},
    project::init_project,
};

#[test]
fn legacy_draft_uses_task_and_only_writes_candidate_when_requested() {
    let root = std::env::temp_dir().join(format!("dadrock-simple-draft-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&root).unwrap();
    init_project(&root).unwrap();
    let pack = root.join("draft-guidance.md");
    std::fs::write(&pack, "Preserve sample exclusions carefully.").unwrap();
    let script = root.join("agent.py");
    std::fs::write(&script, r#"
import sys,json
for line in sys.stdin:
    q=json.loads(line)
    method=q.get('method')
    if method=='initialize':
        result={'protocolVersion':1,'agentCapabilities':{},'authMethods':[]}
    elif method=='session/new':
        assert '--read-only' in json.dumps(q), q
        result={'sessionId':'test','configOptions':[]}
    elif method=='session/prompt':
        prompt=json.dumps(q)
        assert 'skill preflight' not in prompt, 'Builtins must work without an installed skill'
        assert 'context-to-record' in prompt
        assert 'Remember sample exclusions' in prompt
        assert 'Preserve sample exclusions carefully.' in prompt
        output={'kind':'candidate_draft.v1','title':'Exclude missing income','tags':['writer/agent'], 'body_markdown':'## Context\nRationale not documented.\n\n## Decision\nExclude missing income.\n\n## Alternatives\nNot documented.\n\n## Consequences\nSmaller sample.', 'source_references':['analysis.R:2'], 'warnings':['Rationale not documented'], 'confidence':'high', 'provenance':{'model':'fake-task'}}
        print(json.dumps({'jsonrpc':'2.0','method':'session/update','params':{'sessionId':'test','update':{'sessionUpdate':'agent_message_chunk','content':{'type':'text','text':json.dumps(output)}}}}),flush=True)
        result={'stopReason':'end_turn'}
    else:
        continue
    print(json.dumps({'jsonrpc':'2.0','id':q['id'],'result':result}),flush=True)
"#).unwrap();
    let context = CommandContext::from_path(&root).unwrap();
    for write in [false, true] {
        let result = draft_with_acp_agent(
            &context,
            AgentDraftCommandRequest {
                prompt: "Remember sample exclusions".to_owned(),
                local_agent: LocalAcpAgentCommand::new(
                    "python3",
                    vec![script.to_string_lossy().into_owned()],
                ),
                permission_policy: AcpPermissionPolicy::Deny,
                prompt_packs: vec![pack.to_string_lossy().into_owned()],
                enable_dadrock_mcp: true,
                write,
            },
        )
        .unwrap();
        assert_eq!(result.command, "agent draft");
        assert!(result.data.response.provenance.prompt_pack_sha256.is_some());
        assert_eq!(result.data.candidate.is_some(), write);
        assert_eq!(result.changed_paths.len(), usize::from(write));
        assert_eq!(
            result.data.response.provenance.model.as_deref(),
            Some("fake-task")
        );
        assert_eq!(
            result.data.response.source_references[0].excerpt.as_deref(),
            Some("analysis.R:2")
        );
        let (_, index) = context.initialized_index().unwrap();
        assert_eq!(index.list_records(None).len(), usize::from(write));
    }
    std::fs::remove_dir_all(root).unwrap();
}
