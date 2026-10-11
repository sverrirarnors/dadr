//! Built-in tasks are the task guides of the `dadr` agent skill
//! (`skills/dadr/tasks/<id>.md`), embedded at build time. Each guide's YAML
//! front matter is the task manifest and its body is the task prompt.

#[derive(Debug, Clone, Copy)]
pub(crate) struct BuiltinTaskSpec {
    pub id: &'static str,
    pub document: &'static str,
}

pub(crate) fn starter_task_specs() -> &'static [BuiltinTaskSpec] {
    STARTER_TASKS
}

macro_rules! skill_task {
    ($id:literal) => {
        BuiltinTaskSpec {
            id: $id,
            document: include_str!(concat!("../../../../skills/dadr/tasks/", $id, ".md")),
        }
    };
}

const STARTER_TASKS: &[BuiltinTaskSpec] = &[
    skill_task!("custom"),
    skill_task!("context-to-record"),
    skill_task!("code-to-record"),
    skill_task!("find-conflicts"),
    skill_task!("merge-records"),
    skill_task!("supersede-record"),
    skill_task!("edit-record"),
    skill_task!("decisions-to-spec"),
];

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, fs, path::Path};

    use super::STARTER_TASKS;

    #[test]
    fn every_skill_task_guide_is_a_registered_builtin() {
        let tasks_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../skills/dadr/tasks");
        let guides = fs::read_dir(&tasks_dir)
            .expect("read skills/dadr/tasks")
            .map(|entry| entry.expect("read skill task entry").path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
            .map(|path| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .expect("UTF-8 task guide name")
                    .to_owned()
            })
            .collect::<BTreeSet<_>>();
        let registered = STARTER_TASKS
            .iter()
            .map(|spec| spec.id.to_owned())
            .collect::<BTreeSet<_>>();

        assert_eq!(
            registered.len(),
            STARTER_TASKS.len(),
            "duplicate built-in task ids"
        );
        assert_eq!(
            registered, guides,
            "built-in tasks must match the task guides in skills/dadr/tasks/"
        );
    }
}
