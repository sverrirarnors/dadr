# Review decisions in your browser

```sh
dadrock preview
dadrock preview --port 4005 path/to/project
```

Open the local URL printed in the terminal. The interface has two places and one button.

## Inbox

The Inbox is the home screen and the work queue. Everything that needs a human lands here, grouped by kind:

- **Candidates** wait for a decision. Read the record, fix its explanation if needed, then **Accept** or **Reject**. **Revise with agent** sends it back to a local agent with your instructions.
- **Agent proposals** are decisions a local agent drafted from your notes or your code. **Save as candidate** writes the file so you can review it like any other candidate; **Discard** drops it.
- **Proposed revisions** show an agent's edit to an existing record as a diff. **Apply** writes it; **Discard** drops it.
- **Conflicts** come from **Check for conflicts**, which asks the agent to look for overlapping or contradictory decisions. From a conflict you can **Merge** two candidates, **Replace** an accepted decision, or **Dismiss** the finding.
- **Issues** are validation findings: structure, links, tags, and project layout. Validation runs on every load, so there is nothing to trigger. **Open record** jumps to the editor; **Fix with agent** asks for a revision; warnings and notes can be dismissed.

Review both the observed choice and its rationale. An agent-generated description of code may be accurate while the reason for that choice remains unknown. Supply the missing rationale before accepting.

An empty Inbox is the goal. Use `j` and `k` to move through items, `a` and `r` for the primary and secondary action, `e` to edit, and `o` to open the record in the Ledger.

## Ledger

The Ledger is what has been decided. It lists accepted records, newest first, with one search box and a tag facet. **Show history** reveals superseded and rejected records. Each record shows its supersession chain, an inline editor, and two verbs:

- **Supersede** prepares a replacement when a choice changes. The original stays as history, and accepting the replacement links the two.
- **Revise with agent** asks a local agent to rewrite the record from your instructions. The result appears in the Inbox as a proposed revision.

Dadrock records a pending replacement in an optional Markdown section:

```markdown
## Replaces

[[01938c20-a3b4-7000-8000-111122223333|Previous decision]]
```

The link uses the original record's full UUID. Keep this section if the candidate should replace that decision: it preserves the intended relationship after reloading or accepting through the CLI. On acceptance, Dadrock validates the predecessor and updates its successors together with accepting the replacement, preserving the old body. This is a Dadrock convention, not an additional required section of the DADR format.

## Draft

**Draft** opens a composer from anywhere. Describe the decision in the text box, or attach files and folders to have a [configured agent](../llms.md) look for decisions in your code. Attach two candidates to merge them. Opened from a record, the same composer revises or replaces that record. Results land in the Inbox as proposals.

Without an agent, **Write it by hand** gives you a plain form with the candidate template. **Replace with existing decisions instead** links an accepted record to records that already exist.

## Project

The project name in the header opens a small panel with the project root, the detected agent, and three actions: initialize a new project, run validation now, and clear the agent's run history. Local run state is stored under `dadr/runs/` and ignored by default because it may contain pasted context, source excerpts, and model transcripts.

Old links still work: `/records`, `/candidates/new`, `/tasks`, `/validation`, `/tags`, and `/settings` redirect to the Inbox, the Ledger, or the composer.
