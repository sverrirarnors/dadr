mod builtin;
pub mod discovery;
mod ids;
pub mod manifest;
pub mod output;
mod runner;
pub mod runs;

pub use discovery::{
    default_user_tasks_dir, discover_tasks, TaskDiagnostic, TaskDiagnosticCode, TaskListing,
    TaskPackage, TaskSource, TaskSourceKind,
};
pub use manifest::{
    parse_task_manifest, TaskExecutionMode, TaskInput, TaskInputType, TaskManifest,
    TaskNoteRequirement, TaskOutput, TaskPathRequirement, TaskPrompt, TaskRecordRequirement,
    TaskRecordStatusRequirement, TaskRequirements, TaskScopeRequirement, TaskTool, TaskToolMode,
};
pub use output::{
    validate_task_output, CandidateBatchOutput, CandidateDraftOutput, ConflictSetOutput,
    DiagnosisReportOutput, MarkdownReportOutput, RecordRevisionOutput, SupersessionProposalOutput,
    TaskOutputKind, TaskOutputValue,
};
pub use runner::{
    create_task_run_for_acp, create_task_run_for_task, request_task_run_cancellation,
    run_created_local_task, run_created_task_with_acp, run_task_with_acp, AcpTaskRunOptions,
    TaskContext, TaskRunCommandRequest,
};
pub use runs::{
    append_or_extend_task_run_log, append_task_run_log, clear_task_runs, create_task_run,
    list_task_runs, mark_task_run_cancelled, mark_task_run_running, read_task_result,
    read_task_run, read_task_run_log, resolve_task_run_dir, task_runs_dir, write_task_failure,
    write_task_result, PersistedTaskRun, TaskRunLogEntry, TaskRunStatus,
};
