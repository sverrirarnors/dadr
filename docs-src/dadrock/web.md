# Review decisions in your browser

```sh
dadrock preview
dadrock preview --port 4005 path/to/project
```

Open the local URL printed in the terminal. Use **Decisions** to find existing records and **To review** to work through candidates. Open a candidate, edit its explanation, save it, and accept or reject it.

Review both the observed choice and its rationale. An agent-generated description of code may be accurate while the reason for that choice remains unknown. Supply missing rationale before endorsing the decision.

When an accepted choice changes, use **Change this decision** to prepare a replacement and link it to the original. The original stays available as history.

## Optional drafting

With a [configured agent](../llms.md), use drafting actions to propose decisions from code or supplied context. Start with a small batch and review it before asking for more. Agent outputs remain proposals.

Advanced controls provide tags, validation, agent workflow configuration, and run history. Local run state is stored under `dadr/runs/` and ignored by default because it may contain pasted context, source excerpts, and model transcripts.
