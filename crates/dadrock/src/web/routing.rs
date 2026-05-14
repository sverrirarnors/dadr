use camino::Utf8PathBuf;
use dadrock::commands as core_commands;

use super::{
    http::{json_response, validate_host_header, ApiError, ApiFailure, HttpRequest, HttpResponse},
    static_assets::web_response,
};

pub(super) fn route_request(
    request: &HttpRequest,
    cwd: &Utf8PathBuf,
) -> Result<HttpResponse, ApiFailure> {
    validate_host_header(request)?;
    let path = request.path.split('?').next().unwrap_or(&request.path);
    match (request.method.as_str(), path) {
        ("GET", "/api/project") => json_response(200, &super::build_snapshot(cwd)?),
        ("GET", "/api/commands") => json_response(200, &core_commands::command_catalog()),
        ("GET", "/api/tasks") => super::list_tasks(cwd),
        ("GET", "/api/task-runs") => super::list_task_runs(cwd),
        ("GET", "/api/templates/candidate") => super::candidate_template(cwd),
        ("GET", record_path) => {
            if let Some(task_id) = task_query(record_path, "/api/tasks/") {
                return super::show_task(cwd, task_id);
            }
            if let Some(run_id) = task_log_query(record_path) {
                return super::show_task_log(cwd, run_id);
            }
            if let Some(run_id) = task_query(record_path, "/api/task-runs/") {
                return super::show_task_result(cwd, run_id);
            }
            if let Some(query) = record_lifecycle_query(record_path) {
                return super::record_lifecycle(cwd, query);
            }
            if let Some(query) = record_source_query(record_path) {
                return super::record_source(cwd, query);
            }
            if record_path.starts_with("/api/") {
                json_response(404, &ApiError::not_found())
            } else {
                web_response(record_path)
                    .map(Ok)
                    .unwrap_or_else(|| json_response(404, &ApiError::not_found()))
            }
        }
        ("POST", "/api/init") => super::initialize_project(cwd),
        ("POST", "/api/records") => super::create_record(request, cwd),
        ("POST", "/api/records/supersede") => super::supersede_records(request, cwd),
        ("POST", "/api/validate/project") => super::validate_project(cwd),
        ("POST", "/api/validate/records") => super::validate_records(cwd),
        ("POST", "/api/validate/links") => super::validate_links(cwd),
        ("POST", "/api/validate/tags") => super::validate_tags(cwd),
        ("POST", "/api/agent/draft") => super::agent_draft(request, cwd),
        ("POST", "/api/tasks/run") => super::run_task(request, cwd),
        ("POST", "/api/task-runs/clear") => super::clear_task_history(cwd),
        ("POST", action_path) => {
            if let Some(run_id) = task_cancel_query(action_path) {
                return super::cancel_task_run(cwd, run_id);
            }
            if let Some(query) = record_revision_query(action_path) {
                return super::apply_record_revision(request, cwd, query);
            }
            if let Some(query) = record_source_query(action_path) {
                return super::save_record_source(request, cwd, query);
            }
            if let Some(query) = candidate_action_query(action_path, "/accept") {
                return super::accept_candidate(cwd, query);
            }
            if let Some(query) = candidate_action_query(action_path, "/reject") {
                return super::reject_candidate(cwd, query);
            }
            json_response(404, &ApiError::not_found())
        }
        _ => json_response(404, &ApiError::not_found()),
    }
}

pub(super) fn candidate_action_query<'a>(path: &'a str, suffix: &str) -> Option<&'a str> {
    dynamic_segment_query(path, "/api/candidates/", suffix)
}

pub(super) fn record_lifecycle_query(path: &str) -> Option<&str> {
    dynamic_segment_query(path, "/api/records/", "/status")
}

pub(super) fn record_source_query(path: &str) -> Option<&str> {
    dynamic_segment_query(path, "/api/records/", "/source")
}

pub(super) fn record_revision_query(path: &str) -> Option<&str> {
    dynamic_segment_query(path, "/api/records/", "/revision")
}

pub(super) fn task_query<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    dynamic_segment_query(path, prefix, "")
}

pub(super) fn task_log_query(path: &str) -> Option<&str> {
    dynamic_segment_query(path, "/api/task-runs/", "/log")
}

pub(super) fn task_cancel_query(path: &str) -> Option<&str> {
    dynamic_segment_query(path, "/api/task-runs/", "/cancel")
}

fn dynamic_segment_query<'a>(path: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    let rest = path.strip_prefix(prefix)?;
    let query = rest.strip_suffix(suffix)?;
    (!query.is_empty() && !query.contains('/')).then_some(query)
}
