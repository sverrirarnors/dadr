use std::{fs, path::PathBuf, process, time::Duration};

use camino::Utf8PathBuf;
use dadrock::{
    project::{discover_from, init_project},
    records::create_candidate,
    tasks::{create_task_run, discover_tasks, write_task_result},
    RecordId,
};
use rmcp::{
    model::{CallToolRequestParams, JsonObject},
    transport::TokioChildProcess,
    ServiceExt,
};
use serde_json::{json, Value};
use tokio::process::Command;

#[tokio::test(flavor = "current_thread")]
async fn dadrock_mcp_stdio_lists_and_calls_tools() {
    let root = temp_project("dadrock-mcp-stdio");
    init_project(&root).expect("initialize project");
    let project = discover_from(&root).expect("discover project");
    create_candidate(&project, "Use an MCP integration test", &[]).expect("create candidate");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 project root");
    let user_tasks_dir =
        Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks directory");
    let tasks = discover_tasks(&cwd, Some(&user_tasks_dir));
    let task = tasks
        .tasks
        .iter()
        .find(|task| task.manifest.id == "find-conflicts")
        .expect("built-in find-conflicts task");
    let run = create_task_run(&cwd, task, json!({"scope": "live"})).expect("create task run");
    let task_result = json!({
        "kind": "conflict_set.v1",
        "summary": "No conflicts found.",
        "findings": []
    });
    write_task_result(&run.run_dir, task_result.clone()).expect("write task result");

    let mut command = Command::new(env!("CARGO_BIN_EXE_dadrock"));
    command.arg("mcp").arg("serve").arg(&root);
    let transport = TokioChildProcess::new(command).expect("spawn dadrock mcp serve");
    let mut client = ().serve(transport).await.expect("initialize MCP client");

    let tools = client.peer().list_all_tools().await.expect("list tools");
    assert!(tools.iter().any(|tool| tool.name == "dadrock.list_records"));
    assert!(tools
        .iter()
        .any(|tool| tool.name == "dadrock.create_candidate"));
    assert!(tools.iter().any(|tool| tool.name == "dadrock.list_tasks"));
    assert!(tools.iter().any(|tool| tool.name == "dadrock.get_task"));
    assert!(tools
        .iter()
        .any(|tool| tool.name == "dadrock.list_task_runs"));
    assert!(tools.iter().any(|tool| tool.name == "dadrock.get_task_run"));
    assert!(tools
        .iter()
        .any(|tool| tool.name == "dadrock.submit_task_result"));

    let result = client
        .peer()
        .call_tool(
            CallToolRequestParams::new("dadrock.list_records").with_arguments(JsonObject::new()),
        )
        .await
        .expect("call list records");
    let records = result
        .structured_content
        .expect("structured records")
        .get("data")
        .expect("records field")
        .as_array()
        .expect("records array")
        .clone();
    assert!(records
        .iter()
        .any(|record| record["title"] == "Use an MCP integration test"));

    let task_listing = client
        .peer()
        .call_tool(
            CallToolRequestParams::new("dadrock.list_tasks").with_arguments(JsonObject::new()),
        )
        .await
        .expect("call list tasks");
    let task_listing = task_listing.structured_content.expect("task list envelope");
    assert_eq!(task_listing["command"], "task list");
    assert_eq!(task_listing["ok"], true);
    assert!(task_listing["data"]["tasks"]
        .as_array()
        .expect("tasks array")
        .iter()
        .any(|task| task["manifest"]["id"] == "find-conflicts"));

    let task_details = client
        .peer()
        .call_tool(
            CallToolRequestParams::new("dadrock.get_task")
                .with_arguments(json_object(json!({ "task_id": "find-conflicts" }))),
        )
        .await
        .expect("call get task");
    let task_details = task_details.structured_content.expect("task show envelope");
    assert_eq!(task_details["command"], "task show");
    assert_eq!(task_details["ok"], true);
    assert_eq!(task_details["data"]["manifest"]["id"], "find-conflicts");
    assert!(task_details["data"]["prompt"]
        .as_str()
        .expect("task prompt")
        .contains("conflict"));

    let task_runs = client
        .peer()
        .call_tool(
            CallToolRequestParams::new("dadrock.list_task_runs").with_arguments(JsonObject::new()),
        )
        .await
        .expect("call list task runs");
    let task_runs = task_runs.structured_content.expect("task runs envelope");
    assert_eq!(task_runs["command"], "task runs");
    assert_eq!(task_runs["ok"], true);
    assert!(task_runs["data"]
        .as_array()
        .expect("task runs array")
        .iter()
        .any(|task_run| task_run["id"] == run.id.to_string()));

    let task_run_result = client
        .peer()
        .call_tool(
            CallToolRequestParams::new("dadrock.get_task_run")
                .with_arguments(json_object(json!({ "run_id": run.id.to_string() }))),
        )
        .await
        .expect("call get task run");
    let task_run_result = task_run_result
        .structured_content
        .expect("task result envelope");
    assert_eq!(task_run_result["command"], "task result");
    assert_eq!(task_run_result["ok"], true);
    assert_eq!(task_run_result["data"], task_result);

    let submitted_run = create_task_run(&cwd, task, json!({"scope": "live"}))
        .expect("create task run for submitted result");
    let submitted_result = json!({
        "kind": "conflict_set.v1",
        "summary": "Submitted through MCP.",
        "findings": []
    });
    let submit_result = client
        .peer()
        .call_tool(
            CallToolRequestParams::new("dadrock.submit_task_result").with_arguments(json_object(
                json!({
                    "run_id": submitted_run.id.to_string(),
                    "result": submitted_result
                }),
            )),
        )
        .await
        .expect("submit task result through MCP");
    let submit_result = submit_result
        .structured_content
        .expect("submit result envelope");
    assert_eq!(submit_result["command"], "task submit-result");
    assert_eq!(submit_result["ok"], true);
    assert_eq!(submit_result["data"]["status"], "succeeded");

    let submitted_task_run_result = client
        .peer()
        .call_tool(
            CallToolRequestParams::new("dadrock.get_task_run").with_arguments(json_object(
                json!({ "run_id": submitted_run.id.to_string() }),
            )),
        )
        .await
        .expect("read submitted task result through MCP");
    let submitted_task_run_result = submitted_task_run_result
        .structured_content
        .expect("submitted task result envelope");
    assert_eq!(submitted_task_run_result["data"], submitted_result);

    let create = client
        .peer()
        .call_tool(
            CallToolRequestParams::new("dadrock.create_candidate").with_arguments(json_object(
                json!({
                    "title": "Create a candidate through MCP",
                    "tags": ["kind/quality", "writer/agent"],
                    "body_markdown": "## Context\n\nThe MCP integration test needs a candidate with complete sections.\n\n## Decision\n\nCreate a candidate through the Dadrock MCP command surface.\n\n## Consequences\n\nThis exercises the same command facade as the CLI.\n\n## Status\n\nCandidate for test verification.\n"
                }),
            )),
        )
        .await
        .expect("create candidate through MCP");
    let create = create.structured_content.expect("create envelope");
    assert_eq!(create["command"], "new");
    assert_eq!(create["ok"], true);
    assert_eq!(
        create["changed_paths"]
            .as_array()
            .expect("changed paths")
            .len(),
        1
    );

    let id = create["data"]["id"]
        .as_str()
        .expect("created candidate id")
        .to_owned();
    let reject = client
        .peer()
        .call_tool(
            CallToolRequestParams::new("dadrock.reject_record")
                .with_arguments(json_object(json!({ "query": id }))),
        )
        .await
        .expect("reject candidate through MCP");
    let reject = reject.structured_content.expect("reject envelope");
    assert_eq!(reject["command"], "status reject");
    assert_eq!(reject["data"]["status"], "rejected");
    assert!(reject["data"]["to"]
        .as_str()
        .expect("rejected path")
        .contains("/dadr/archive/"));
    assert_eq!(
        reject["changed_paths"]
            .as_array()
            .expect("changed paths")
            .len(),
        2
    );

    let _ = tokio::time::timeout(Duration::from_secs(2), client.close()).await;
    let _ = fs::remove_dir_all(root);
}

fn json_object(value: Value) -> JsonObject {
    match value {
        Value::Object(object) => object,
        _ => JsonObject::new(),
    }
}

fn temp_project(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{label}-{}-{}", process::id(), RecordId::new_v7()))
}
