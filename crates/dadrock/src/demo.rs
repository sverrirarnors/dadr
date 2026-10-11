use std::{fs, path::PathBuf};

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    error::DadrError,
    file_store,
    ids::RecordId,
    model::RecordStatus,
    project::{init_project, utf8_to_path},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemoProjectResult {
    pub example: DemoExample,
    pub project_root: Utf8PathBuf,
    pub decisions_dir: Utf8PathBuf,
    pub records: Vec<DemoRecordSummary>,
    pub support_paths: Vec<Utf8PathBuf>,
    pub created_paths: Vec<Utf8PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DemoExample {
    Lifecycle,
    CandidateReview,
    Penguins,
    Broken,
    Conflicts,
    TaskHistory,
}

impl DemoExample {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lifecycle => "lifecycle",
            Self::CandidateReview => "candidate-review",
            Self::Penguins => "penguins",
            Self::Broken => "broken",
            Self::Conflicts => "conflicts",
            Self::TaskHistory => "task-history",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemoRecordSummary {
    pub id: RecordId,
    pub title: String,
    pub status: RecordStatus,
    pub path: Utf8PathBuf,
}

struct DemoRecordSeed {
    relative_path: &'static str,
    title: &'static str,
    status: RecordStatus,
    id: &'static str,
    successors: &'static [&'static str],
    tags: &'static [&'static str],
    /// Raw YAML lines for front-matter keys outside DADR.
    extra_front_matter: &'static str,
    body: &'static str,
}

const ID_COMPLETE_CASE: &str = "01938b8e-4d70-7abc-8def-000000000001";
const ID_OUTCOME_SCALE: &str = "01938b8f-9d71-7456-8000-000000000002";
const ID_ASSIGNMENT_LOG: &str = "01938b91-bcd0-7e00-8b00-000000000003";
const ID_COVARIATE_IMPUTATION: &str = "01938b92-bcd0-7e00-8b00-000000000004";
const ID_SHARED_CLEANING: &str = "01938b93-bcd0-7e00-8b00-000000000005";
const ID_CLUSTER_SE: &str = "01938b94-bcd0-7e00-8b00-000000000006";
const ID_SPEC_CURVE: &str = "01938b95-bcd0-7e00-8b00-000000000007";
const ID_RENAME_OUTCOME: &str = "01938b96-bcd0-7e00-8b00-000000000008";
const ID_NEGATIVE_CONTROL: &str = "01938b97-bcd0-7e00-8b00-000000000009";
const ID_ACCEPTED_CANDIDATE: &str = "01938b98-bcd0-7e00-8b00-00000000000a";
const ID_REJECTED_SITE: &str = "01938b99-bcd0-7e00-8b00-00000000000b";
const ID_REJECTED_MERGE: &str = "01938b9a-bcd0-7e00-8b00-00000000000c";

const DEMO_RECORDS: &[DemoRecordSeed] = &[
    DemoRecordSeed {
        relative_path: "01938b8e4d70-complete-case-analysis-sample.md",
        title: "Complete-case analysis sample",
        status: RecordStatus::Accepted,
        id: ID_COMPLETE_CASE,
        successors: &[ID_ASSIGNMENT_LOG, ID_COVARIATE_IMPUTATION],
        tags: &["kind/sample", "confidence/high"],
        extra_front_matter: "bibliography: bibliography.bib\n",
        body: concat!(
            "## Context\n\n",
            "The first analysis used a complete-case rule for every required field. ",
            "This record is now superseded by a split into assignment-log handling ",
            "and covariate handling.\n\n",
            "## Decision\n\n",
            "Use complete cases for the original main analysis sample.\n\n",
            "## Alternatives\n\n",
            "- Keep partially observed rows and model missingness.\n",
            "- Report a descriptive sample only.\n\n",
            "## Consequences\n\n",
            "This broad rule was split into [[01938b91bcd0|assignment-log handling]] ",
            "and [[01938b92bcd0|covariate imputation]].\n\n",
            "## Plan deviation\n\n",
            "> The plan expected a narrower exclusion rule, so this record carries an ",
            "explicit deviation section.\n",
        ),
    },
    DemoRecordSeed {
        relative_path: "01938b8f9d71-outcome-scale-v1.md",
        title: "Outcome scale v1",
        status: RecordStatus::Accepted,
        id: ID_OUTCOME_SCALE,
        successors: &[ID_SHARED_CLEANING],
        tags: &["kind/measurement", "confidence/medium"],
        extra_front_matter: "",
        body: concat!(
            "## Context\n\n",
            "The outcome was initially scored as a simple row mean.\n\n",
            "## Decision\n\n",
            "Average the available survey items into a single primary outcome.\n\n",
            "## Alternatives\n\n",
            "Use a sum score or a factor score. The mean was easier to audit.\n\n",
            "## Consequences\n\n",
            "The scoring rule was later consolidated into [[01938b93bcd0]].\n",
        ),
    },
    DemoRecordSeed {
        relative_path: "01938b91bcd0-audit-assignment-log.md",
        title: "Audit assignment log",
        status: RecordStatus::Accepted,
        id: ID_ASSIGNMENT_LOG,
        successors: &[],
        tags: &["kind/input", "topic/assignment"],
        extra_front_matter: "",
        body: concat!(
            "## Context\n\n",
            "Treatment assignment is defined by the audit log rather than by downstream ",
            "survey status.\n\n",
            "## Decision\n\n",
            "Use the audited assignment log as the authoritative treatment source.\n\n",
            "## Alternatives\n\n",
            "Use survey-reported assignment. This was rejected because respondents can ",
            "misremember their assignment status.\n\n",
            "## Consequences\n\n",
            "Rows without audited assignment are excluded from treatment-effect models. ",
            "See [[01938b8e4d70]] for the superseded broad rule.\n",
        ),
    },
    DemoRecordSeed {
        relative_path: "01938b92bcd0-impute-missing-covariates.md",
        title: "Impute missing covariates",
        status: RecordStatus::Accepted,
        id: ID_COVARIATE_IMPUTATION,
        successors: &[ID_SHARED_CLEANING],
        tags: &[
            "kind/transformation",
            "topic/covariates",
            "confidence/medium",
        ],
        extra_front_matter: "",
        body: concat!(
            "## Context\n\n",
            "Several baseline covariates are missing at low rates.\n\n",
            "## Decision\n\n",
            "Use multiple imputation for baseline covariates in the main model.\n\n",
            "## Alternatives\n\n",
            "Drop rows with any missing covariate. This would lose precision.\n\n",
            "## Consequences\n\n",
            "The imputation rule was later folded into [[01938b93bcd0]] so preprocessing ",
            "has one visible decision.\n\n",
            "## Robustness\n\n",
            "Run a complete-case robustness check on the main estimand.\n",
        ),
    },
    DemoRecordSeed {
        relative_path: "01938b93bcd0-shared-cleaning-rules.md",
        title: "Shared cleaning rules",
        status: RecordStatus::Accepted,
        id: ID_SHARED_CLEANING,
        successors: &[],
        tags: &["kind/transformation", "topic/preprocessing"],
        extra_front_matter: "",
        body: concat!(
            "## Context\n\n",
            "The outcome scale and covariate imputation rules converged into a common ",
            "preprocessing contract.\n\n",
            "## Decision\n\n",
            "Use one shared cleaning step for outcomes and covariates before model fitting.\n\n",
            "## Alternatives\n\n",
            "Keep separate scripts for each model family. This made reporting harder.\n\n",
            "## Consequences\n\n",
            "This decision consolidates [[01938b8f9d71]] and [[01938b92bcd0]].\n",
        ),
    },
    DemoRecordSeed {
        relative_path: "01938b94bcd0-use-cluster-robust-standard-errors.md",
        title: "Use cluster-robust standard errors",
        status: RecordStatus::Accepted,
        id: ID_CLUSTER_SE,
        successors: &[],
        tags: &["kind/inference", "multiverse/se-type/cluster-robust"],
        extra_front_matter: "",
        body: concat!(
            "## Context\n\n",
            "Observations are nested within sites. The demo code tag example is in ",
            "[examples/code-tags.py](examples/code-tags.py).\n\n",
            "## Decision\n\n",
            "Use cluster-robust standard errors clustered by site.\n\n",
            "## Alternatives\n\n",
            "Use heteroskedasticity-robust standard errors. This ignores site clustering.\n\n",
            "## Consequences\n\n",
            "Inference is slightly more conservative. Code comments may carry tags like ",
            "#dadr/kind/inference and #dadr/01938b94bcd0.\n",
        ),
    },
    DemoRecordSeed {
        relative_path: "01938b95bcd0-report-specification-curve.md",
        title: "Report specification curve",
        status: RecordStatus::Accepted,
        id: ID_SPEC_CURVE,
        successors: &[],
        tags: &["kind/robustness", "confidence/high"],
        extra_front_matter: "bibliography:\n  - bibliography.bib\n  - robustness.bib\n",
        body: concat!(
            "## Context\n\n",
            "The paper needs to show that the result is not an artifact of one model ",
            "choice [@simonsohn2020].\n\n",
            "## Decision\n\n",
            "Report a specification curve over the core robustness axes.\n\n",
            "## Alternatives\n\n",
            "Report only the preferred model. This would hide the sensitivity analysis.\n\n",
            "## Consequences\n\n",
            "The manuscript includes a figure and a table keyed to the DADR records.\n\n",
            "## Notes\n\n",
            "This record demonstrates a front-matter key outside DADR: Pandoc's ",
            "`bibliography` array passes through unchanged.\n",
        ),
    },
    DemoRecordSeed {
        relative_path: "01938b96-bcd0-7e00-8b00-000000000008-rename-primary-outcome.md",
        title: "Rename primary outcome",
        status: RecordStatus::Accepted,
        id: ID_RENAME_OUTCOME,
        successors: &[],
        tags: &["kind/communication", "confidence/high"],
        extra_front_matter: "",
        body: concat!(
            "## Context\n\n",
            "The manuscript label for the primary outcome was too close to a secondary ",
            "construct.\n\n",
            "## Decision\n\n",
            "Use the label primary engagement index throughout the manuscript.\n\n",
            "## Alternatives\n\n",
            "Keep the older label. This risked confusing readers.\n\n",
            "## Consequences\n\n",
            "Tables, captions, and narrative text use one label. See [[01938b95bcd0]] ",
            "for the reporting decision that consumes this label.\n",
        ),
    },
    DemoRecordSeed {
        relative_path: "candidates/01938b97bcd0-add-negative-control-outcome.md",
        title: "Add negative-control outcome",
        status: RecordStatus::Candidate,
        id: ID_NEGATIVE_CONTROL,
        successors: &[],
        tags: &["kind/robustness", "writer/agent", "workflow/needs-review"],
        extra_front_matter: "",
        body: concat!(
            "## Context\n\n",
            "A reviewer asked whether the treatment predicts an outcome it should not ",
            "affect.\n\n",
            "## Decision\n\n",
            "Candidate: add one negative-control outcome to the supplement.\n\n",
            "## Alternatives\n\n",
            "Leave the request unanswered until revision.\n\n",
            "## Consequences\n\n",
            "If accepted, this candidate will add a supplemental model.\n\n",
            "## Open questions\n\n",
            "> The team still needs to choose the exact negative-control variable.\n",
        ),
    },
    DemoRecordSeed {
        relative_path: "rejected/01938b98bcd0-early-complete-case-candidate.md",
        title: "Early complete-case candidate",
        status: RecordStatus::Rejected,
        id: ID_ACCEPTED_CANDIDATE,
        successors: &[],
        tags: &[
            "kind/sample",
            "writer/analyst",
            "rejected/merged",
            "workflow/closed",
        ],
        extra_front_matter: "",
        body: concat!(
            "## Context\n\n",
            "This rejected candidate proposed the same complete-case sample rule ",
            "that was later recorded in the accepted decision.\n\n",
            "## Decision\n\n",
            "Reject this draft as merged into the accepted complete-case decision.\n\n",
            "## Alternatives\n\n",
            "Wait for more missingness diagnostics. The team chose to record the rule ",
            "immediately and supersede it later if needed.\n\n",
            "## Consequences\n\n",
            "> #rejected/merged\n",
            "> The accepted record [[01938b8e4d70]] is the durable decision.\n",
        ),
    },
    DemoRecordSeed {
        relative_path: "rejected/01938b99bcd0-drop-site-seven-candidate.md",
        title: "Drop site seven candidate",
        status: RecordStatus::Rejected,
        id: ID_REJECTED_SITE,
        successors: &[],
        tags: &[
            "kind/sample",
            "rejected/no-longer-relevant",
            "workflow/closed",
        ],
        extra_front_matter: "",
        body: concat!(
            "## Context\n\n",
            "A draft suggested dropping one site after a preliminary balance check.\n\n",
            "## Decision\n\n",
            "Reject the candidate.\n\n",
            "## Alternatives\n\n",
            "Keep the site and use site fixed effects.\n\n",
            "## Consequences\n\n",
            "> #rejected/no-longer-relevant\n",
            "> The final audited assignment data removed the motivating concern.\n",
        ),
    },
    DemoRecordSeed {
        relative_path: "rejected/01938b9abcd0-merge-outcome-candidate.md",
        title: "Merge outcome candidate",
        status: RecordStatus::Rejected,
        id: ID_REJECTED_MERGE,
        successors: &[],
        tags: &["kind/measurement", "rejected/merged", "workflow/closed"],
        extra_front_matter: "",
        body: concat!(
            "## Context\n\n",
            "Two outcome-record drafts described the same measurement choice.\n\n",
            "## Decision\n\n",
            "Reject this candidate as merged into another record.\n\n",
            "## Alternatives\n\n",
            "Accept both records. That would duplicate the trace.\n\n",
            "## Consequences\n\n",
            "> #rejected/merged\n",
            "> No accepted decision was created from this candidate.\n",
        ),
    },
];

struct PreparedDemoProject {
    project_root: Utf8PathBuf,
    dadr_dir: Utf8PathBuf,
    decisions_dir: Utf8PathBuf,
    support_paths: Vec<Utf8PathBuf>,
    created_paths: Vec<Utf8PathBuf>,
}

pub fn seed_demo_project(target: &Utf8Path) -> Result<DemoProjectResult, DadrError> {
    seed_demo_project_with_example(target, DemoExample::Lifecycle)
}

pub fn seed_demo_project_with_example(
    target: &Utf8Path,
    example: DemoExample,
) -> Result<DemoProjectResult, DadrError> {
    match example {
        DemoExample::Lifecycle => seed_lifecycle_demo_project(target),
        DemoExample::CandidateReview => seed_candidate_review_demo_project(target),
        DemoExample::Penguins => seed_penguins_demo_project(target),
        DemoExample::Broken => seed_broken_demo_project(target),
        DemoExample::Conflicts => seed_conflicts_demo_project(target),
        DemoExample::TaskHistory => seed_task_history_demo_project(target),
    }
}

fn prepare_demo_project(target: &Utf8Path) -> Result<PreparedDemoProject, DadrError> {
    let target = absolutize(target)?;
    let project_root = target.clone();
    let decisions_dir = decisions_dir_for_target(&project_root);
    let dadr_dir = decisions_dir
        .parent()
        .map(Utf8Path::to_path_buf)
        .unwrap_or_else(|| decisions_dir.clone());
    ensure_new_or_empty_dir(&project_root)?;

    let init = init_project(project_root.as_std_path())?;
    Ok(PreparedDemoProject {
        project_root,
        dadr_dir,
        decisions_dir: init.decisions_dir,
        support_paths: Vec::new(),
        created_paths: init.created,
    })
}

fn seed_lifecycle_demo_project(target: &Utf8Path) -> Result<DemoProjectResult, DadrError> {
    let PreparedDemoProject {
        project_root,
        dadr_dir,
        decisions_dir,
        mut support_paths,
        mut created_paths,
    } = prepare_demo_project(target)?;

    write_support_file(
        &decisions_dir,
        "bibliography.bib",
        concat!(
            "@book{rubin1987,\n",
            "  title = {Multiple Imputation for Nonresponse in Surveys},\n",
            "  author = {Rubin, Donald B.},\n",
            "  year = {1987}\n",
            "}\n",
        ),
        &mut support_paths,
        &mut created_paths,
    )?;
    write_support_file(
        &decisions_dir,
        "robustness.bib",
        concat!(
            "@article{simonsohn2020,\n",
            "  title = {Specification Curve Analysis},\n",
            "  author = {Simonsohn, Uri and Simmons, Joseph P. and Nelson, Leif D.},\n",
            "  year = {2020}\n",
            "}\n",
        ),
        &mut support_paths,
        &mut created_paths,
    )?;
    write_support_file(
        &dadr_dir,
        "templates/candidate.md",
        concat!(
            "## Context\n\n",
            "Describe why this candidate exists.\n\n",
            "## Decision\n\n",
            "State the proposed decision.\n\n",
            "## Alternatives\n\n",
            "List alternatives or say Unknown.\n\n",
            "## Consequences\n\n",
            "Describe expected effects.\n\n",
            "## Review notes\n\n",
        ),
        &mut support_paths,
        &mut created_paths,
    )?;
    write_support_file(
        &decisions_dir,
        "examples/code-tags.py",
        concat!(
            "# DADR code-tag examples for the demo project.\n",
            "# #dadr/01938b94bcd0 #dadr/kind/inference #dadr/topic/standard-errors\n",
            "model = \"cluster-robust standard errors\"\n\n",
            "# #dadr/multiverse/se-type/cluster-robust #dadr/confidence/high\n",
            "standard_errors = \"clustered by site\"\n\n",
            "# #dadr/ignore\n",
            "scratch_note = \"ignored by candidate generation\"\n",
        ),
        &mut support_paths,
        &mut created_paths,
    )?;

    let mut records = Vec::new();
    for seed in DEMO_RECORDS {
        let path = if seed.relative_path.starts_with("candidates/")
            || seed.relative_path.starts_with("rejected/")
        {
            dadr_dir.join(seed.relative_path)
        } else {
            decisions_dir.join(seed.relative_path)
        };
        let content = render_record(seed)?;
        file_store::write_new(&path, &content)?;
        push_unique(&mut created_paths, path.clone());
        records.push(DemoRecordSummary {
            id: RecordId::parse(seed.id)?,
            title: seed.title.to_owned(),
            status: seed.status,
            path,
        });
    }

    Ok(DemoProjectResult {
        example: DemoExample::Lifecycle,
        project_root,
        decisions_dir,
        records,
        support_paths,
        created_paths,
    })
}

fn seed_candidate_review_demo_project(target: &Utf8Path) -> Result<DemoProjectResult, DadrError> {
    let PreparedDemoProject {
        project_root,
        dadr_dir,
        decisions_dir,
        mut support_paths,
        mut created_paths,
    } = prepare_demo_project(target)?;

    write_demo_candidate_template(&dadr_dir, &mut support_paths, &mut created_paths)?;
    write_support_file(
        &project_root,
        "README.md",
        concat!(
            "# Candidate Review Demo\n\n",
            "This project is intentionally focused on the candidate queue. It has no ",
            "accepted decisions yet, so reviewers can accept, reject, and revise ",
            "candidate DADRs without extra ledger noise.\n",
        ),
        &mut support_paths,
        &mut created_paths,
    )?;

    let candidate_records = [
        DemoRecordSeed {
            relative_path: "candidates/01938ba0bcd0-use-adelie-and-gentoo-only.md",
            title: "Use Adelie and Gentoo only",
            status: RecordStatus::Candidate,
            id: "01938ba0-bcd0-7e00-8b00-000000000011",
            successors: &[],
            tags: &["kind/sample", "writer/agent", "confidence/high"],
            extra_front_matter: "",
            body: concat!(
                "## Context\n\n",
                "The first modeling pass compares the two most common penguin species in the dataset.\n\n",
                "## Decision\n\n",
                "Restrict the analysis sample to Adelie and Gentoo penguins.\n\n",
                "## Alternatives\n\n",
                "Include Chinstrap penguins. This would make the species contrast less focused.\n\n",
                "## Consequences\n\n",
                "The model estimates a simpler species contrast and the omitted species needs a note.\n",
            ),
        },
        DemoRecordSeed {
            relative_path: "candidates/01938ba1bcd0-drop-rows-with-missing-model-variables.md",
            title: "Drop rows with missing model variables",
            status: RecordStatus::Candidate,
            id: "01938ba1-bcd0-7e00-8b00-000000000012",
            successors: &[],
            tags: &["kind/sample", "writer/agent", "confidence/high"],
            extra_front_matter: "",
            body: concat!(
                "## Context\n\n",
                "The OLS formula needs body mass, bill length, flipper length, and species indicators.\n\n",
                "## Decision\n\n",
                "Use complete cases for the variables in the body-mass model.\n\n",
                "## Alternatives\n\n",
                "Impute missing measurements before fitting. This is unnecessary for the small demo.\n\n",
                "## Consequences\n\n",
                "The model sample is smaller but the rule is explicit and easy to review.\n",
            ),
        },
        DemoRecordSeed {
            relative_path: "candidates/01938ba2bcd0-use-body-mass-as-the-outcome.md",
            title: "Use body mass as the outcome",
            status: RecordStatus::Candidate,
            id: "01938ba2-bcd0-7e00-8b00-000000000013",
            successors: &[],
            tags: &["kind/measurement", "writer/analyst", "confidence/high"],
            extra_front_matter: "",
            body: concat!(
                "## Context\n\n",
                "Body mass is complete enough for a compact demonstration and has clear biological meaning.\n\n",
                "## Decision\n\n",
                "Use `body_mass_g` as the outcome in the main OLS model.\n\n",
                "## Alternatives\n\n",
                "Use bill depth or flipper length as the outcome. Those are better as predictors here.\n\n",
                "## Consequences\n\n",
                "The model interpretation is straightforward for a short screencast.\n",
            ),
        },
        DemoRecordSeed {
            relative_path: "candidates/01938ba3bcd0-reviewer-edit-clarify-species-dummy.md",
            title: "Clarify species dummy coding",
            status: RecordStatus::Candidate,
            id: "01938ba3-bcd0-7e00-8b00-000000000014",
            successors: &[],
            tags: &["kind/transformation", "writer/analyst", "workflow/needs-edit"],
            extra_front_matter: "",
            body: concat!(
                "## Context\n\n",
                "A reviewer asked whether the species coefficient compares Adelie to Gentoo or Gentoo to Adelie.\n\n",
                "## Decision\n\n",
                "Revise the candidate to state the dummy coding direction clearly.\n\n",
                "## Alternatives\n\n",
                "Leave the dummy implicit in the Python code.\n\n",
                "## Consequences\n\n",
                "The record needs an edit before acceptance so the coefficient is interpretable.\n",
            ),
        },
        DemoRecordSeed {
            relative_path: "candidates/01938ba4bcd0-add-island-fixed-effects.md",
            title: "Add island fixed effects",
            status: RecordStatus::Candidate,
            id: "01938ba4-bcd0-7e00-8b00-000000000015",
            successors: &[],
            tags: &["kind/model", "writer/agent", "confidence/low", "workflow/question"],
            extra_front_matter: "",
            body: concat!(
                "## Context\n\n",
                "Island may confound species differences because species are not evenly distributed.\n\n",
                "## Decision\n\n",
                "Candidate: add island fixed effects to the OLS model.\n\n",
                "## Alternatives\n\n",
                "Keep the compact formula for the first demo.\n\n",
                "## Consequences\n\n",
                "This is low confidence because the screencast may not need that complexity.\n\n",
                "## Open questions\n\n",
                "> Should this be a robustness check rather than the main model?\n",
            ),
        },
    ];

    let mut records = Vec::new();
    for seed in candidate_records {
        let path = dadr_dir.join(seed.relative_path);
        let content = render_record(&seed)?;
        file_store::write_new(&path, &content)?;
        push_unique(&mut created_paths, path.clone());
        records.push(DemoRecordSummary {
            id: RecordId::parse(seed.id)?,
            title: seed.title.to_owned(),
            status: seed.status,
            path,
        });
    }

    Ok(DemoProjectResult {
        example: DemoExample::CandidateReview,
        project_root,
        decisions_dir,
        records,
        support_paths,
        created_paths,
    })
}

fn seed_penguins_demo_project(target: &Utf8Path) -> Result<DemoProjectResult, DadrError> {
    let PreparedDemoProject {
        project_root,
        dadr_dir: _,
        decisions_dir,
        mut support_paths,
        mut created_paths,
    } = prepare_demo_project(target)?;

    write_support_file(
        &project_root,
        "README.md",
        concat!(
            "# Penguins Body-Mass Demo\n\n",
            "This project is a realistic code-to-record target for Dadrock screencasts. ",
            "The analysis script downloads Palmer Penguins data from Rdatasets when run, ",
            "subsets species, creates a dummy variable, and fits a statsmodels OLS model ",
            "with a formula.\n\n",
            "Run the script outside Dadrock with `uv run analysis/penguins_ols.py` if you ",
            "want to execute the analysis. Dadrock's demo seeding only writes files.\n",
        ),
        &mut support_paths,
        &mut created_paths,
    )?;
    write_support_file(
        &project_root,
        "analysis/notes.md",
        concat!(
            "# Analysis notes\n\n",
            "- Outcome: penguin body mass in grams.\n",
            "- Main comparison: Adelie vs Gentoo after excluding Chinstrap.\n",
            "- Predictors: bill length, flipper length, and an Adelie dummy.\n",
            "- Missing model variables are dropped before fitting the OLS model.\n",
        ),
        &mut support_paths,
        &mut created_paths,
    )?;
    write_support_file(
        &project_root,
        "analysis/penguins_ols.py",
        PENGUINS_OLS_SCRIPT,
        &mut support_paths,
        &mut created_paths,
    )?;
    write_support_file(
        &project_root,
        "outputs/.gitkeep",
        "",
        &mut support_paths,
        &mut created_paths,
    )?;

    Ok(DemoProjectResult {
        example: DemoExample::Penguins,
        project_root,
        decisions_dir,
        records: Vec::new(),
        support_paths,
        created_paths,
    })
}

fn seed_task_history_demo_project(target: &Utf8Path) -> Result<DemoProjectResult, DadrError> {
    let mut result = seed_conflicts_demo_project(target)?;
    result.example = DemoExample::TaskHistory;

    for (task_id, run_id, input, output) in task_history_runs() {
        seed_task_run(
            &result.project_root,
            task_id,
            run_id,
            input,
            output,
            &mut result.support_paths,
            &mut result.created_paths,
        )?;
    }

    Ok(result)
}

fn seed_broken_demo_project(target: &Utf8Path) -> Result<DemoProjectResult, DadrError> {
    let mut result = seed_lifecycle_demo_project(target)?;
    result.example = DemoExample::Broken;

    write_support_file(
        &result.decisions_dir,
        "01938b9bbcd0-broken-missing-consequences.md",
        concat!(
            "---\n",
            "title: Broken missing consequences\n",
            "status: accepted\n",
            "id: 01938b9b-bcd0-7e00-8b00-00000000000d\n",
            "tags:\n",
            "  - kind/example\n",
            "---\n\n",
            "## Context\n\n",
            "This intentionally broken record exists for validation screencasts.\n\n",
            "## Decision\n\n",
            "Keep this record invalid until the validation repair episode fixes it.\n\n",
            "## Alternatives\n\n",
            "Use a malformed YAML example. Missing sections are easier to repair live.\n",
        ),
        &mut result.support_paths,
        &mut result.created_paths,
    )?;

    Ok(result)
}

fn seed_conflicts_demo_project(target: &Utf8Path) -> Result<DemoProjectResult, DadrError> {
    let mut result = seed_lifecycle_demo_project(target)?;
    result.example = DemoExample::Conflicts;

    let conflict_records = [
        DemoRecordSeed {
            relative_path: "candidates/01938b9cbcd0-drop-missing-body-mass-rows.md",
            title: "Drop missing body-mass rows",
            status: RecordStatus::Candidate,
            id: "01938b9c-bcd0-7e00-8b00-00000000000e",
            successors: &[],
            tags: &["kind/sample", "writer/agent", "confidence/high"],
            extra_front_matter: "",
            body: concat!(
                "## Context\n\n",
                "The Penguins OLS model cannot use rows with missing body mass or predictor values.\n\n",
                "## Decision\n\n",
                "Drop rows missing any variable used by the body-mass model.\n\n",
                "## Alternatives\n\n",
                "Impute missing values before fitting the model.\n\n",
                "## Consequences\n\n",
                "The fitted sample is smaller but the model code is simple to inspect.\n",
            ),
        },
        DemoRecordSeed {
            relative_path: "candidates/01938b9dbcd0-use-complete-cases-for-body-mass-model.md",
            title: "Use complete cases for body-mass model",
            status: RecordStatus::Candidate,
            id: "01938b9d-bcd0-7e00-8b00-00000000000f",
            successors: &[],
            tags: &["kind/sample", "writer/analyst", "confidence/medium"],
            extra_front_matter: "",
            body: concat!(
                "## Context\n\n",
                "The body-mass model has missing values in bill length, flipper length, and body mass.\n\n",
                "## Decision\n\n",
                "Use a complete-case sample for the model variables.\n\n",
                "## Alternatives\n\n",
                "Keep all rows and let statsmodels silently omit missing observations.\n\n",
                "## Consequences\n\n",
                "This overlaps with the candidate to drop missing body-mass rows and should be merged.\n",
            ),
        },
        DemoRecordSeed {
            relative_path: "candidates/01938b9ebcd0-reintroduce-complete-case-analysis.md",
            title: "Reintroduce complete-case analysis",
            status: RecordStatus::Candidate,
            id: "01938b9e-bcd0-7e00-8b00-000000000010",
            successors: &[],
            tags: &["kind/sample", "writer/agent", "confidence/low"],
            extra_front_matter: "",
            body: concat!(
                "## Context\n\n",
                "An agent suggested returning to the broad complete-case rule.\n\n",
                "## Decision\n\n",
                "Use complete cases for every field in the analysis.\n\n",
                "## Alternatives\n\n",
                "Keep the narrower accepted assignment-log and covariate decisions.\n\n",
                "## Consequences\n\n",
                "This conflicts with the superseded broad complete-case record and needs review.\n",
            ),
        },
    ];

    for seed in conflict_records {
        let dadr_dir = result
            .decisions_dir
            .parent()
            .map(Utf8Path::to_path_buf)
            .unwrap_or_else(|| result.decisions_dir.clone());
        let path = dadr_dir.join(seed.relative_path);
        let content = render_record(&seed)?;
        file_store::write_new(&path, &content)?;
        push_unique(&mut result.created_paths, path.clone());
        result.records.push(DemoRecordSummary {
            id: RecordId::parse(seed.id)?,
            title: seed.title.to_owned(),
            status: seed.status,
            path,
        });
    }

    Ok(result)
}

fn write_demo_candidate_template(
    dadr_dir: &Utf8Path,
    support_paths: &mut Vec<Utf8PathBuf>,
    created_paths: &mut Vec<Utf8PathBuf>,
) -> Result<(), DadrError> {
    write_support_file(
        dadr_dir,
        "templates/candidate.md",
        concat!(
            "## Context\n\n",
            "Describe why this candidate exists.\n\n",
            "## Decision\n\n",
            "State the proposed decision.\n\n",
            "## Alternatives\n\n",
            "List alternatives or say Unknown.\n\n",
            "## Consequences\n\n",
            "Describe expected effects.\n\n",
            "## Review notes\n\n",
        ),
        support_paths,
        created_paths,
    )
}

fn seed_task_run(
    project_root: &Utf8Path,
    task_id: &str,
    run_id: &str,
    input: Value,
    result: Value,
    support_paths: &mut Vec<Utf8PathBuf>,
    created_paths: &mut Vec<Utf8PathBuf>,
) -> Result<(), DadrError> {
    let run_dir = project_root.join("dadr/runs").join(task_id).join(run_id);
    let timestamp = "2026-05-14T12:00:00Z";
    let run = json!({
        "id": run_id,
        "task_id": task_id,
        "task_source": {
            "kind": "builtin",
            "id": task_id
        },
        "task_version": 1,
        "status": "succeeded",
        "created_at": timestamp,
        "updated_at": timestamp,
        "run_dir": run_dir,
    });
    let log_entry = json!({
        "timestamp": timestamp,
        "message": format!("seeded successful {task_id} run for screencast demo"),
    });

    write_json_support_file(&run_dir, "run.json", &run, support_paths, created_paths)?;
    write_json_support_file(&run_dir, "input.json", &input, support_paths, created_paths)?;
    write_json_support_file(
        &run_dir,
        "result.json",
        &result,
        support_paths,
        created_paths,
    )?;
    write_support_file(
        &run_dir,
        "log.jsonl",
        &(log_entry.to_string() + "\n"),
        support_paths,
        created_paths,
    )?;
    Ok(())
}

fn task_history_runs() -> Vec<(&'static str, &'static str, Value, Value)> {
    vec![
        (
            "context-to-record",
            "01938bb0-bcd0-7e00-8b00-000000000016",
            json!({"context": "Reviewer requested an explicit note about using body mass as the primary outcome."}),
            json!({
                "kind": "candidate_draft.v1",
                "title": "Use body mass as primary outcome",
                "tags": ["kind/measurement", "writer/agent", "confidence/high"],
                "body_markdown": "## Context\n\nA reviewer asked the team to make the outcome choice explicit.\n\n## Decision\n\nUse `body_mass_g` as the primary outcome for the Penguins OLS demo.\n\n## Alternatives\n\nUse bill depth or flipper length as the outcome. Those are better predictors in this compact model.\n\n## Consequences\n\nThe interpretation is direct and the candidate can be reviewed before acceptance.\n",
                "source_references": [],
                "warnings": []
            }),
        ),
        (
            "code-to-record",
            "01938bb1-bcd0-7e00-8b00-000000000017",
            json!({"path": "analysis/penguins_ols.py", "focus": "Find consequential analysis decisions in the script."}),
            json!({
                "kind": "candidate_batch.v1",
                "summary": "The script contains sample, transformation, missingness, and model-formula choices.",
                "candidates": [
                    {
                        "id": "species-subset",
                        "title": "Compare Adelie and Gentoo penguins",
                        "tags": ["kind/sample", "writer/agent", "confidence/high"],
                        "body_markdown": "## Context\n\nThe Python script excludes Chinstrap penguins before fitting the OLS model.\n\n## Decision\n\nRestrict the analysis to Adelie and Gentoo penguins.\n\n## Alternatives\n\nInclude all species and estimate a multi-level species contrast.\n\n## Consequences\n\nThe model is easier to explain but excludes one species from the estimand.\n",
                        "source_references": [{"path": "analysis/penguins_ols.py"}],
                        "confidence": "high",
                        "warnings": []
                    },
                    {
                        "id": "formula-choice",
                        "title": "Fit body mass OLS formula",
                        "tags": ["kind/model", "writer/agent", "confidence/medium"],
                        "body_markdown": "## Context\n\nThe script models body mass with bill length, flipper length, and an Adelie dummy.\n\n## Decision\n\nFit `body_mass_g ~ bill_length_mm + flipper_length_mm + is_adelie` with statsmodels OLS.\n\n## Alternatives\n\nAdd island fixed effects or interaction terms.\n\n## Consequences\n\nThe compact formula is suitable for a demo but not a complete biological analysis.\n",
                        "source_references": [{"path": "analysis/penguins_ols.py"}],
                        "confidence": "medium",
                        "warnings": ["Consider whether island should be a robustness check."]
                    }
                ]
            }),
        ),
        (
            "supersede-record",
            "01938bb2-bcd0-7e00-8b00-000000000018",
            json!({"record_id": ID_COMPLETE_CASE, "reason": "Replace the broad complete-case rule with model-specific missingness handling."}),
            json!({
                "kind": "supersession_proposal.v1",
                "summary": "Draft a narrower successor for the broad complete-case rule.",
                "superseded_record_id": ID_COMPLETE_CASE,
                "superseding_title": "Use model-specific complete cases",
                "tags": ["kind/sample", "writer/agent"],
                "superseding_body_markdown": "## Context\n\nThe original complete-case decision was broader than the current model requires.\n\n## Decision\n\nDrop missing rows only for variables required by each model.\n\n## Alternatives\n\nKeep the broad complete-case rule for all fields.\n\n## Consequences\n\nEach model reports its own analysis sample and the old broad rule is superseded.\n",
                "rationale": "The replacement narrows the rule and better matches the current analysis code.",
                "suggested_actions": ["Write superseding candidate", "Review old record"]
            }),
        ),
        (
            "find-conflicts",
            "01938bb3-bcd0-7e00-8b00-000000000019",
            json!({"status_scope": "live"}),
            json!({
                "kind": "conflict_set.v1",
                "summary": "Two candidate sample rules overlap and one low-confidence candidate conflicts with a superseded rule.",
                "findings": [{
                    "id": "missingness-duplicate",
                    "title": "Duplicate complete-case candidates",
                    "left_record_id": "01938b9c-bcd0-7e00-8b00-00000000000e",
                    "right_record_id": "01938b9d-bcd0-7e00-8b00-00000000000f",
                    "relationship": "overlap",
                    "conflict_explanation": "Both candidates describe dropping rows with missing model variables.",
                    "why_might_not_be_conflict": "They could be merged into one clearer candidate.",
                    "suggested_actions": ["Run merge-records", "Reject duplicate after review"]
                }]
            }),
        ),
        (
            "merge-records",
            "01938bb4-bcd0-7e00-8b00-00000000001a",
            json!({"record_ids": "01938b9c-bcd0-7e00-8b00-00000000000e\n01938b9d-bcd0-7e00-8b00-00000000000f"}),
            json!({
                "kind": "candidate_draft.v1",
                "title": "Use complete cases for model variables",
                "tags": ["kind/sample", "writer/agent", "confidence/high"],
                "body_markdown": "## Context\n\nTwo candidate records described the same missingness rule for the body-mass model.\n\n## Decision\n\nUse complete cases for the variables required by the fitted model.\n\n## Alternatives\n\nLet statsmodels silently omit missing rows.\n\n## Consequences\n\nThe merged candidate is clearer and the duplicate drafts can be rejected after review.\n",
                "source_references": [],
                "warnings": []
            }),
        ),
        (
            "edit-record",
            "01938bb5-bcd0-7e00-8b00-00000000001b",
            json!({"record_id": ID_NEGATIVE_CONTROL, "requested_changes": "Clarify which variable is the negative control."}),
            json!({
                "kind": "record_revision.v1",
                "summary": "Clarifies the open question in the negative-control candidate.",
                "record_id": ID_NEGATIVE_CONTROL,
                "title": "Add negative-control outcome",
                "tags": ["kind/robustness", "writer/agent", "workflow/needs-review"],
                "body_markdown": "## Context\n\nA reviewer asked whether the treatment predicts an outcome it should not affect.\n\n## Decision\n\nAdd one negative-control outcome to the supplement after choosing the exact variable.\n\n## Alternatives\n\nLeave the request unanswered until revision.\n\n## Consequences\n\nIf accepted, this candidate will add a supplemental model and record the chosen negative-control variable.\n",
                "change_explanation": "The revision makes the unresolved variable choice explicit.",
                "warnings": ["The exact negative-control variable still needs human selection."]
            }),
        ),
        (
            "custom",
            "01938bb6-bcd0-7e00-8b00-00000000001c",
            json!({"notes": "Summarize which screencast episodes this ledger can support."}),
            json!({
                "kind": "markdown_report.v1",
                "title": "Screencast Readiness",
                "summary": "The seeded project covers lifecycle, code-to-record, validation, and conflict cleanup demos.",
                "body_markdown": "## Ready episodes\n\n- Candidate review\n- Supersession\n- Conflict cleanup\n- Task result rendering\n\n## Remaining setup\n\nUse the Penguins demo for code-to-record path selection.\n",
                "raw_json": {"episodes": ["candidate-review", "supersession", "conflict-cleanup", "task-history"]}
            }),
        ),
    ]
}

fn write_json_support_file(
    base: &Utf8Path,
    relative_path: &str,
    value: &Value,
    support_paths: &mut Vec<Utf8PathBuf>,
    created_paths: &mut Vec<Utf8PathBuf>,
) -> Result<(), DadrError> {
    let content = serde_json::to_string_pretty(value)?;
    write_support_file(base, relative_path, &content, support_paths, created_paths)
}

const PENGUINS_OLS_SCRIPT: &str = concat!(
    "# /// script\n",
    "# dependencies = [\"pandas\", \"statsmodels\"]\n",
    "# ///\n",
    "\"\"\"Fit a small Palmer Penguins OLS model for Dadrock demos.\"\"\"\n\n",
    "from pathlib import Path\n\n",
    "import pandas as pd\n",
    "import statsmodels.formula.api as smf\n\n",
    "DATA_URL = \"https://vincentarelbundock.github.io/Rdatasets/csv/palmerpenguins/penguins.csv\"\n",
    "OUTPUT_DIR = Path(__file__).resolve().parent.parent / \"outputs\"\n",
    "OUTPUT_DIR.mkdir(exist_ok=True)\n\n",
    "penguins = pd.read_csv(DATA_URL)\n\n",
    "# Decision: compare Adelie and Gentoo only; Chinstrap is excluded from this model.\n",
    "species_subset = penguins[penguins[\"species\"].isin([\"Adelie\", \"Gentoo\"])].copy()\n\n",
    "# Decision: encode Adelie as the focal species indicator for the OLS formula.\n",
    "species_subset[\"is_adelie\"] = (species_subset[\"species\"] == \"Adelie\").astype(int)\n\n",
    "model_data = species_subset.dropna(\n",
    "    subset=[\"body_mass_g\", \"bill_length_mm\", \"flipper_length_mm\", \"is_adelie\"]\n",
    ").copy()\n\n",
    "# Decision: body mass is modeled as a function of bill length, flipper length, and species.\n",
    "model = smf.ols(\n",
    "    formula=\"body_mass_g ~ bill_length_mm + flipper_length_mm + is_adelie\",\n",
    "    data=model_data,\n",
    ").fit()\n\n",
    "(OUTPUT_DIR / \"penguins_ols_summary.txt\").write_text(model.summary().as_text())\n",
    "print(model.summary())\n",
);

fn decisions_dir_for_target(target: &Utf8Path) -> Utf8PathBuf {
    if target.file_name() == Some("dadr") {
        target.join("accepted")
    } else {
        target.join("dadr/accepted")
    }
}

fn ensure_new_or_empty_dir(path: &Utf8Path) -> Result<(), DadrError> {
    if !path.exists() {
        return Ok(());
    }
    if !path.is_dir() {
        return Err(DadrError::ProjectInvariant(format!(
            "demo target `{path}` exists but is not a directory"
        )));
    }
    if fs::read_dir(utf8_to_path(path))?
        .next()
        .transpose()?
        .is_some()
    {
        return Err(DadrError::InvalidInput(format!(
            "demo requires a new or empty target directory; `{path}` is not empty"
        )));
    }
    Ok(())
}

fn write_support_file(
    decisions_dir: &Utf8Path,
    relative_path: &str,
    content: &str,
    support_paths: &mut Vec<Utf8PathBuf>,
    created_paths: &mut Vec<Utf8PathBuf>,
) -> Result<(), DadrError> {
    let path = decisions_dir.join(relative_path);
    file_store::write_new(&path, content)?;
    support_paths.push(path.clone());
    push_unique(created_paths, path);
    Ok(())
}

fn render_record(seed: &DemoRecordSeed) -> Result<String, DadrError> {
    let id = RecordId::parse(seed.id)?;
    let mut output = String::new();
    output.push_str("---\n");
    output.push_str("title: ");
    output.push_str(seed.title);
    output.push('\n');
    output.push_str("status: ");
    output.push_str(seed.status.as_str());
    output.push('\n');
    output.push_str("id: ");
    output.push_str(&id.to_string());
    output.push('\n');

    if !seed.successors.is_empty() {
        output.push_str("successors:\n");
        for successor in seed.successors {
            let successor = RecordId::parse(successor)?;
            output.push_str("  - ");
            output.push_str(&successor.to_string());
            output.push('\n');
        }
    }

    if !seed.tags.is_empty() {
        output.push_str("tags:\n");
        for tag in seed.tags {
            output.push_str("  - ");
            output.push_str(tag);
            output.push('\n');
        }
    }

    output.push_str(seed.extra_front_matter);

    output.push_str("---\n\n");
    output.push_str(seed.body.trim());
    output.push('\n');
    Ok(output)
}

fn push_unique(paths: &mut Vec<Utf8PathBuf>, path: Utf8PathBuf) {
    if !paths.contains(&path) {
        paths.push(path);
    }
}

fn absolutize(path: &Utf8Path) -> Result<Utf8PathBuf, DadrError> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    path_to_utf8(std::env::current_dir()?.join(path.as_std_path()))
}

fn path_to_utf8(path: PathBuf) -> Result<Utf8PathBuf, DadrError> {
    Utf8PathBuf::from_path_buf(path).map_err(|path| DadrError::PathNotUtf8 {
        path: path.display().to_string(),
    })
}
