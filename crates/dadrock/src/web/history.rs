//! Read-only history and diff endpoints. Both return the command envelope of
//! `dadrock history` and `dadrock diff`.
//!
//! - `GET /api/records/{id}/history[?patch=1]`
//! - `GET /api/records/{id}/diff?against={other}`: from `other` to `{id}`.

use camino::Utf8PathBuf;
use dadrock::commands as core_commands;

use super::{
    context_for,
    http::{json_response, ApiFailure, HttpRequest, HttpResponse},
};

pub(super) fn record_history(
    request: &HttpRequest,
    cwd: &Utf8PathBuf,
    query: &str,
) -> Result<HttpResponse, ApiFailure> {
    let patch = query_flag(&request.path, "patch");
    let envelope = core_commands::history(&context_for(cwd)?, &percent_decode(query), patch)?;
    json_response(200, &envelope)
}

pub(super) fn record_diff(
    request: &HttpRequest,
    cwd: &Utf8PathBuf,
    query: &str,
) -> Result<HttpResponse, ApiFailure> {
    let against = query_param(&request.path, "against")
        .filter(|against| !against.trim().is_empty())
        .ok_or_else(|| {
            ApiFailure::bad_request("diff requires ?against=<record> (the record to compare from)")
        })?;
    let envelope = core_commands::diff(&context_for(cwd)?, &against, &percent_decode(query))?;
    json_response(200, &envelope)
}

/// The decoded value of the first `name` parameter in the request target's
/// query string.
fn query_param(target: &str, name: &str) -> Option<String> {
    let (_, query) = target.split_once('?')?;
    query
        .split('&')
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (percent_decode(key) == name).then(|| percent_decode(value))
        })
        .next()
}

/// A flag parameter is on when present, unless its value is `0`, `false`, or
/// `no`.
fn query_flag(target: &str, name: &str) -> bool {
    query_param(target, name).is_some_and(|value| {
        !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "no"
        )
    })
}

/// Decode `%XX` escapes and `+` as a space. Invalid escapes stay as written.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => decoded.push(b' '),
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3])
                    .ok()
                    .and_then(|hex| u8::from_str_radix(hex, 16).ok());
                match hex {
                    Some(byte) => {
                        decoded.push(byte);
                        index += 2;
                    }
                    None => decoded.push(b'%'),
                }
            }
            byte => decoded.push(byte),
        }
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use camino::Utf8PathBuf;
    use dadrock::{project::init_project, RecordId};

    use super::{percent_decode, query_flag, query_param};
    use crate::web::{
        http::{HttpHeader, HttpRequest},
        routing::route_request,
    };

    const OLD: &str = "01938c20-a3b4-7000-8000-000000000001";
    const NEW: &str = "0193b2a1-c0f4-7000-8000-000000000002";

    fn project_with_successor() -> Utf8PathBuf {
        let root = Utf8PathBuf::from_path_buf(
            std::env::temp_dir().join(format!("dadrock-web-history-{}", RecordId::new_v7())),
        )
        .unwrap();
        fs::create_dir_all(&root).unwrap();
        init_project(&root).unwrap();
        fs::write(
            root.join("dadr/archive/listwise-deletion.md"),
            format!(
                "---\ntitle: Listwise deletion\nstatus: accepted\nid: {OLD}\nsuccessors:\n  - {NEW}\ntags:\n  - topic/attrition\n---\n\n## Context\n\nMissing data.\n\n## Decision\n\nUse listwise deletion.\n\n## Alternatives\n\nNone.\n\n## Consequences\n\nSmaller sample.\n"
            ),
        )
        .unwrap();
        fs::write(
            root.join("dadr/ledger/multiple-imputation.md"),
            format!(
                "---\ntitle: Multiple imputation\nstatus: accepted\nid: {NEW}\ntags:\n  - topic/missing-data\n---\n\n## Supersedes\n\n[[{OLD}|Listwise deletion]]\n\nComplete cases lose a third of the sample.\n\n## Context\n\nMissing data.\n\n## Decision\n\nUse multiple imputation.\n\n## Alternatives\n\nNone.\n\n## Consequences\n\nSmaller sample.\n"
            ),
        )
        .unwrap();
        root
    }

    fn get_json(path: &str, root: &Utf8PathBuf) -> (u16, serde_json::Value) {
        let request = HttpRequest {
            method: "GET".to_owned(),
            path: path.to_owned(),
            headers: vec![HttpHeader {
                name: "Host".to_owned(),
                value: "localhost".to_owned(),
            }],
            body: Vec::new(),
        };
        match route_request(&request, root) {
            Ok(response) => (
                response.status,
                serde_json::from_slice(&response.body).unwrap(),
            ),
            Err(failure) => (
                failure.status,
                serde_json::json!({"error": failure.message}),
            ),
        }
    }

    #[test]
    fn history_endpoint_returns_the_command_envelope() {
        let root = project_with_successor();

        let (status, value) = get_json(&format!("/api/records/{OLD}/history"), &root);
        assert_eq!(status, 200);
        assert_eq!(value["command"], "history");
        assert_eq!(value["ok"], true);
        let data = &value["data"];
        assert_eq!(data["record"], OLD);
        let versions: Vec<_> = data["versions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|version| version["id"].as_str().unwrap())
            .collect();
        assert_eq!(versions, [NEW, OLD]);
        assert_eq!(data["versions"][0]["current"], true);
        assert!(!data["versions"][0]["body"]
            .as_str()
            .unwrap()
            .contains("## Supersedes"));
        assert_eq!(data["links"][0]["predecessor"], OLD);
        assert_eq!(data["links"][0]["successor"], NEW);
        assert_eq!(
            data["links"][0]["reason"],
            "Complete cases lose a third of the sample."
        );
        assert!(data["links"][0].get("diff").is_none());
        assert_eq!(data["neighbors"], serde_json::json!([]));

        let (status, value) = get_json(&format!("/api/records/{NEW}/history?patch=1"), &root);
        assert_eq!(status, 200);
        let diff = &value["data"]["links"][0]["diff"];
        assert_eq!(diff["changed"], true);
        assert_eq!(diff["title"]["old"], "Listwise deletion");
        assert_eq!(
            diff["tags_added"],
            serde_json::json!(["topic/missing-data"])
        );
        assert_eq!(diff["tags_removed"], serde_json::json!(["topic/attrition"]));
        assert!(diff["lines"]
            .as_array()
            .unwrap()
            .iter()
            .any(|line| line["kind"] == "added" && line["text"] == "Use multiple imputation."));

        let (status, _) = get_json("/api/records/0193ffff/history", &root);
        assert_eq!(status, 400);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn diff_endpoint_compares_against_another_record() {
        let root = project_with_successor();

        let (status, value) = get_json(
            &format!("/api/records/{NEW}/diff?against=01938c20a3b4"),
            &root,
        );
        assert_eq!(status, 200);
        assert_eq!(value["command"], "diff");
        assert_eq!(value["data"]["old"]["id"], OLD);
        assert_eq!(value["data"]["new"]["id"], NEW);
        assert_eq!(
            value["data"]["diff"]["title"],
            serde_json::json!({"old": "Listwise deletion", "new": "Multiple imputation"})
        );
        assert!(value["data"]["diff"]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .all(|line| line["text"] != "## Supersedes"));

        let (status, value) = get_json(
            &format!("/api/records/{NEW}/diff?against=Listwise%20deletion"),
            &root,
        );
        assert_eq!(status, 200);
        assert_eq!(value["data"]["old"]["id"], OLD);

        let (status, value) = get_json(&format!("/api/records/{NEW}/diff"), &root);
        assert_eq!(status, 400);
        assert!(value["error"].as_str().unwrap().contains("against"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn query_parameters_are_decoded() {
        assert_eq!(
            query_param("/api/records/x/diff?against=a%20b&patch", "against").as_deref(),
            Some("a b")
        );
        assert_eq!(query_param("/api/records/x/diff", "against"), None);
        assert!(query_flag("/api/records/x/history?patch", "patch"));
        assert!(query_flag("/api/records/x/history?patch=true", "patch"));
        assert!(!query_flag("/api/records/x/history?patch=0", "patch"));
        assert!(!query_flag("/api/records/x/history", "patch"));
        assert_eq!(percent_decode("a+b%2Fc%zz%4"), "a b/c%zz%4");
    }
}
