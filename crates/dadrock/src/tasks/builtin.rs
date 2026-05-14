#[derive(Debug, Clone, Copy)]
pub(crate) struct BuiltinTaskSpec {
    pub id: &'static str,
    pub manifest: &'static str,
    pub prompt: &'static str,
}

pub(crate) fn starter_task_specs() -> &'static [BuiltinTaskSpec] {
    STARTER_TASKS
}

const STARTER_TASKS: &[BuiltinTaskSpec] = &[
    BuiltinTaskSpec {
        id: "custom",
        manifest: include_str!("builtin/custom/task.toml"),
        prompt: include_str!("builtin/custom/prompt.md"),
    },
    BuiltinTaskSpec {
        id: "context-to-record",
        manifest: include_str!("builtin/context-to-record/task.toml"),
        prompt: include_str!("builtin/context-to-record/prompt.md"),
    },
    BuiltinTaskSpec {
        id: "code-to-record",
        manifest: include_str!("builtin/code-to-record/task.toml"),
        prompt: include_str!("builtin/code-to-record/prompt.md"),
    },
    BuiltinTaskSpec {
        id: "find-conflicts",
        manifest: include_str!("builtin/find-conflicts/task.toml"),
        prompt: include_str!("builtin/find-conflicts/prompt.md"),
    },
    BuiltinTaskSpec {
        id: "merge-records",
        manifest: include_str!("builtin/merge-records/task.toml"),
        prompt: include_str!("builtin/merge-records/prompt.md"),
    },
    BuiltinTaskSpec {
        id: "supersede-record",
        manifest: include_str!("builtin/supersede-record/task.toml"),
        prompt: include_str!("builtin/supersede-record/prompt.md"),
    },
    BuiltinTaskSpec {
        id: "edit-record",
        manifest: include_str!("builtin/edit-record/task.toml"),
        prompt: include_str!("builtin/edit-record/prompt.md"),
    },
];
