# Roadmap

These are proposed directions, not currently available guarantees or delivery commitments. See [DADRock](dadrock/index.md) for current capabilities.

- **Change detection:** inspect Git diffs of analysis code and suggest consequential choices worth documenting. Checking edits to accepted records against Git is already available ([Checking changes](changing.md#checking-changes)).
- **Code-comment scanning:** resolve `#dadr/` annotations in analysis source files.
- **Faithfulness checks:** compare accepted decisions with code, analysis plans, and manuscripts; report discrepancies for human review.
- **Reporting:** draft methods sections and decision narratives with references to records. A built-in agent task already synthesizes accepted decisions into a spec ([Custom workflows and runs](llms.md#custom-workflows-and-runs)).
- **Coverage and robustness:** identify undocumented choices and possible sensitivity analyses.
- **Process metrics:** compute review latency, coverage, and follow-through from appropriate lifecycle data.
- **Additional validation:** citation resolution, plan-deviation requirements, and placeholder detection.

Future checks should help people investigate discrepancies. A missing link does not establish that an analysis is wrong, and a clean validation report does not establish scientific correctness.
