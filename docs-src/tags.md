# Tags

Tags help organize DADR records. They make records easier to find, group
related decisions, and add lightweight structure without forcing every project
into the same vocabulary.

Tag vocabularies can and often should be project-specific or team-specific.
Different projects need different categories, naming habits, and review
workflows. Start with a small shared vocabulary that fits your team, then
extend it only when it helps.

For the full syntax and format rules, see [Tags in the
specification](specification.md#tags). For code comments and source-file
annotations, see [Code tags](specification.md#code-tags).

## Where tags live

Tags usually appear in one of three places:

1. **Front matter** for tags that apply to the whole record.
2. **Inline** in the markdown text for tags attached to a specific paragraph or sentence.
3. **Code comments** with a `#dadr/` prefix when tagging source files.

### Front matter

```yaml
---
title: Drop observations with missing treatment assignment
status: accepted
id: 01938c20-a3b4-7000-8000-111122223333
tags:
  - kind/sample
  - confidence/high
---
```

### Inline

```markdown
> Drop the 47 respondents without recorded treatment
> assignment from the analysis sample.
```

### Code comments

```r
# #dadr/01938c20a3b4
df <- df[!is.na(df$treatment), ]
```

## Suggested vocabularies

Most projects only need a small shared vocabulary at the start. The table below
summarizes common suggestions. Use the ones that fit your team, rename them if
needed, and ignore the rest.

<table>
  <thead>
    <tr>
      <th style="width: 18%;">Namespace</th>
      <th>Use it for</th>
      <th style="width: 30%;">Example values</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td><code>#kind/</code></td>
      <td>What sort of decision the record captures</td>
      <td><code>#kind/sample</code><br/><code>#kind/modeling</code><br/><code>#kind/inference</code><br/><code>#kind/communication</code></td>
    </tr>
    <tr>
      <td><code>#confidence/</code></td>
      <td>How settled or uncertain a decision is</td>
      <td><code>#confidence/high</code><br/><code>#confidence/medium</code><br/><code>#confidence/low</code></td>
    </tr>
    <tr>
      <td><code>#plan/</code></td>
      <td>Which statistical analysis plan (SAP), pre-registration, protocol, or other plan the record references</td>
      <td><code>#plan/sap-2026</code><br/><code>#plan/pap-main</code><br/><code>#plan/protocol-v2</code></td>
    </tr>
    <tr>
      <td><code>#people/</code></td>
      <td>People and review roles</td>
      <td><code>#people/vincent</code><br/><code>#people/decider/alice</code><br/><code>#people/approver/bob</code></td>
    </tr>
    <tr>
      <td><code>#topic/</code></td>
      <td>Free-form subjects</td>
      <td><code>#topic/missingness</code><br/><code>#winsorization</code></td>
    </tr>
    <tr>
      <td><code>#writer/</code></td>
      <td>Decision provenance</td>
      <td><code>#writer/analyst</code><br/><code>#writer/agent</code></td>
    </tr>
    <tr>
      <td><code>#multiverse/</code><br /> <code>#robustness/</code></td>
      <td>Alternative specifications or robustness work</td>
      <td><code>#multiverse/model/ols</code><br/><code>#robustness/placebo</code></td>
    </tr>
  </tbody>
</table>

There is no single required vocabulary. A team may decide to use only
`kind/...` and `people/...`, or to add its own namespaces for plans, review,
datasets, outputs, or anything else that helps keep records organized.

## Recommended `kind/` Values

`kind/...` is the main classification axis. It describes the analytic activity
or review surface that the decision belongs to. Start with the values below and
add project-specific values only when they clarify review.

| Value | Use it for |
|---|---|
| `kind/setup` | Seeds, library loading, paths, global options, version pins |
| `kind/input` | Reading files, querying databases, API fetches, snapshots |
| `kind/sample` | Eligibility, row filters, deduplication, joins, date windows |
| `kind/measurement` | Outcomes, treatments, covariates, labels, units, coding |
| `kind/transformation` | Imputation, winsorization, scaling, aggregation, derived features |
| `kind/exploration` | Summaries, diagnostics, missingness reviews, exploratory plots |
| `kind/modeling` | Formulas, estimators, priors, hyperparameters, model families |
| `kind/inference` | Standard errors, tests, intervals, validation metrics |
| `kind/robustness` | Sensitivity checks, placebo tests, alternative specifications |
| `kind/visualization` | Plot encodings, scales, facets, annotations |
| `kind/communication` | Tables, captions, rounding, manuscript/report wording |
| `kind/output` | Written data, figures, models, dashboards, published artifacts |
| `kind/governance` | SAP/PAP/protocol relations, deviations, amendments, approvals |
| `kind/quality` | Assertions, schema checks, reconciliation, validation checks |
| `kind/orchestration` | Pipeline targets, DAG rules, dependencies, parameters |

The list is intentionally broad. A DADR records a decision, not every activity
in a script or notebook. Use `kind/...` to classify consequential choices that a
reviewer, collaborator, or future analyst would want explained.

Common subactivities can help pick the closest `kind/...` value:

| Kind | Example subactivities |
|---|---|
| `kind/setup` | `seed`, `library_load`, `global_option`, `version_pin`, `path_config` |
| `kind/input` | `file_read`, `database_query`, `api_fetch`, `snapshot_select`, `schema_select` |
| `kind/sample` | `row_filter`, `eligibility_rule`, `deduplication`, `date_window`, `cohort_join`, `missingness_filter` |
| `kind/measurement` | `outcome_definition`, `treatment_definition`, `covariate_definition`, `unit_conversion`, `label_mapping`, `missing_value_code` |
| `kind/transformation` | `imputation`, `winsorization`, `scaling`, `aggregation`, `interaction`, `encoding`, `derived_score` |
| `kind/exploration` | `summary_statistic`, `missingness_summary`, `distribution_check`, `diagnostic_plot`, `correlation_check` |
| `kind/modeling` | `formula`, `estimator`, `prior`, `hyperparameter`, `fixed_effect`, `random_effect`, `resampling_design` |
| `kind/inference` | `standard_error`, `test`, `interval`, `metric`, `multiple_testing`, `validation_split` |
| `kind/robustness` | `alternative_specification`, `placebo`, `negative_control`, `leave_one_out`, `specification_curve` |
| `kind/visualization` | `encoding`, `scale`, `facet`, `annotation`, `theme`, `axis_transform` |
| `kind/communication` | `table_format`, `rounding`, `model_selection_for_display`, `caption`, `report_render` |
| `kind/output` | `write_table`, `write_figure`, `write_model`, `cache_artifact`, `publish_artifact`, `data_export` |
| `kind/governance` | `plan_reference`, `data_cleaning_plan`, `sop_reference`, `plan_deviation`, `amendment`, `approval` |
| `kind/quality` | `assertion`, `schema_check`, `row_count_check`, `snapshot_check`, `test_expectation` |
| `kind/orchestration` | `target`, `rule`, `dependency`, `parameter`, `resource`, `schedule` |

## Tidy tags

Keep tags short, consistent, and predictable. In practice that usually means:

- prefer one spelling for each concept;
- use singular forms consistently, such as `kind/sample` rather than mixing
  `kind/sample` and `kind/sampling`;
- avoid inventing near-duplicate topic tags.

## Roadmap

The current **dadrock** command surface for tags is intentionally small:

```bash
dadrock tags list
dadrock tags list --json
dadrock validate tags
```

Over time, **dadrock** will include more tools to help teams maintain tag hygiene across a project. This will include both deterministic checks and LLM-assisted workflows to:

- keep tag vocabularies consistent;
- consolidate related categories when a project's tag system grows messy;
- surface inconsistencies, overlaps, and near-duplicates for human review.
