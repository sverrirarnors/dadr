# Dadrock Screencast Specification

## Recording Rules

- Every recording is UI-only. Do not show terminal setup, command-line help, or
  shell output.
- Start each recording with Dadrock already open at the intended local URL.
- Prepare demo examples and reset states off-camera.
- Use deterministic task results for agent-assisted demos whenever possible.
- Show the human approval boundary: tasks propose records or reports, and the
  user decides whether to save, accept, reject, merge, or supersede.
- Do not present hooks, code-tag scanning, or faithfulness checks as shipped
  workflows. Keep them out of the main series unless the episode is explicitly a
  roadmap discussion.

## Demo Examples

| Example | Setup intent | Screencasts |
|---|---|---|
| `lifecycle` | Default DADR lifecycle showcase with accepted, candidate, rejected, superseded, split, consolidated, tagged, and linked records | Workbench tour, candidate review, supersession, tags/search/navigation |
| `candidate-review` | Focused five-candidate queue with strong, weak, editable, agent-origin, and low-confidence records | Create/review first candidates, edit-record |
| `penguins` | Realistic Python analysis target for `code-to-record` | Code-to-record, path picker, task results |
| `broken` | Lifecycle-style project with intentional validation defects | Validation queue and repair |
| `conflicts` | Lifecycle-style project with overlapping and contradictory candidates | Find conflicts, merge records, supersession follow-up |
| `task-history` | Lifecycle/conflict project with persisted successful task runs for all important output kinds | Task history and typed result rendering |

The Penguins project contains `analysis/penguins_ols.py`, a `uv` script that
downloads Palmer Penguins data from Rdatasets when run separately, subsets
Adelie and Gentoo penguins, creates an `is_adelie` dummy, drops missing model
variables, and fits a statsmodels OLS formula:

```text
body_mass_g ~ bill_length_mm + flipper_length_mm + is_adelie
```

Dadrock demo seeding only writes the files. It does not run Python or download
data.

## Episode Order

1. **Initialize a project and create the first candidate**
   Start from an uninitialized folder in the UI. Use the initialize action and
   continue into candidate creation. Add tags, fill the four standard sections,
   save the candidate, and show it in the candidate queue.

2. **Review a candidate and tour the workbench**
   Use the `candidate-review` example. Edit a candidate, run validation if
   visible, accept a strong candidate, and reject a weak one. While moving
   through the review, show record filters, status views, the detail pane, tags,
   links, validation entry points, settings, and task navigation.

3. **Context to record**
   Use a populated lifecycle project. Paste a short analysis note or reviewer
   comment into the task, generate one candidate, edit it, and save it for human
   review.

4. **Code to records**
   Use `penguins`. Choose `analysis/penguins_ols.py` in the UI path picker, run
   `code-to-record`, inspect the candidate batch, save the best proposal, and
   leave weak proposals unsaved.

5. **Supersede a decision**
   Use `lifecycle`. Start from an accepted record, draft or select a successor,
   accept the successor if needed, and show the old record as superseded.

6. **Validation and repair**
   Use `broken`. Open the validation queue, inspect issues, repair one missing
   required section or malformed field, and rerun validation.

7. **Find conflicts**
   Use `conflicts`. Run conflict detection, inspect duplicate or contradictory
   candidates, and choose the appropriate follow-up path.

8. **Merge records**
    Continue from `conflicts`. Merge two overlapping candidates into one clearer
    candidate, then show that the originals remain unchanged until the user
    explicitly rejects or leaves them.

9. **Edit record task**
    Use `candidate-review`. Select a candidate with reviewer
    feedback, run `edit-record`, inspect the proposed revision, and apply it.

10. **Task history and results**
    Use `task-history`. Show typed result rendering for candidate drafts,
    candidate batches, supersession proposals, conflict reports, record
    revisions, and markdown reports.

## First Batch

Record these first because they cover the highest-value workflows:

1. Initialize a project and create the first candidate.
2. Review a candidate and tour the workbench.
3. Run context-to-record.
4. Run code-to-record on Penguins.
5. Supersede an accepted decision.
6. Repair validation issues.
7. Find conflicts and merge duplicate candidates.

## Fixture Requirements

- The `lifecycle` demo must remain the default for `dadrock demo`.
- The `candidate-review` demo must keep the queue focused enough for quick
  accept, reject, and edit recordings.
- The `penguins` demo must include realistic analysis code, not only records.
- The `broken` demo must fail validation immediately on load.
- The `conflicts` demo must contain at least two overlapping candidates and one
  candidate that points toward supersession rather than direct editing.
- The `task-history` demo must include persisted successful task runs for
  `context-to-record`, `code-to-record`, `supersede-record`, `find-conflicts`,
  `merge-records`, `edit-record`, and `custom`.
- All demos must be safe to seed without network access.
