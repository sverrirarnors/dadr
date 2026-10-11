# Data Analysis Decision Records

A Data Analysis Decision Record answers one question: **what did we decide, and why?** Keep one Markdown file per consequential choice, alongside your analysis.

## Four sections

- **Context:** the question and relevant evidence.
- **Decision:** what you chose, stated directly.
- **Alternatives:** other approaches considered and why you set them aside.
- **Consequences:** implications, tradeoffs, and follow-up work.

A file also has three required YAML fields: `title`, `status`, and `id`. DADRock generates the ID for you. Tags and links are optional; you do not need a classification system to get started.

## Draft, review, preserve

A new record is a **candidate**. A human reviews it and marks it **accepted** or **rejected**. Acceptance keeps the same ID. An agent may help draft a candidate, but cannot decide whether you endorse it.

Once approved, a decision's substantive content is preserved. Corrections that leave the meaning intact are made in place; changing the decision means accepting a successor that supersedes the original. The original remains accepted, and its `successors` field names the records that supersede it. “Superseded” describes an accepted record with successors, not a fourth stored status. DADRock keeps superseded records in its archive, next to rejected ones, and offers a superseded filter for convenience.

## What deserves a record?

Start with choices a collaborator would ask you to explain: who enters the sample, how a variable is defined, why a model was chosen, or how missing values are handled. Skip routine formatting and mechanical refactoring. A few useful explanations are better than an unmanageable review queue.

Code can reveal what an analysis does without revealing why. When a draft says the rationale is unknown, supply it or leave the record as a candidate until you can review it.

[Install DADRock](dadrock/index.md) and [record your first decision](dadrock/web.md#first-decision), or consult the [full specification](specification.md) for exact syntax and implementation rules.
