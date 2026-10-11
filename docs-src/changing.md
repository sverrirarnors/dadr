# Changing decisions

Once approved, a decision's substantive content is preserved. Minor corrections, such as typos and broken links, remain possible. Changing the decision itself means accepting a successor that supersedes the original.

The original remains accepted, and its `successors` field names the records that supersede it. “Superseded” describes an accepted record with successors, not a fourth stored status. DADRock keeps superseded records in its [archive](layout.md), next to rejected ones, and offers a superseded filter for convenience.

## Drafting a successor

An accepted decision is not rewritten. When an accepted choice changes, draft a successor: a new candidate that supersedes the old record, its predecessor. Use **Supersede** on the record in the [web interface](dadrock/web.md) or the [standalone app](dadrock/app.md), or write the candidate yourself. The successor names its predecessor in a `## Supersedes` section:

```markdown
## Supersedes

[[01938c20-a3b4-7000-8000-111122223333|Winsorize income at p99]]

Winsorizing at p99 still left implausible incomes in two waves.
```

The section starts with a record link to the predecessor by its full UUID, `[[uuid]]` or `[[uuid|label]]`. Any text after the link is the reason for the change, in ordinary Markdown. A record has at most one `## Supersedes` section. When DADRock writes the section, it is the first section of the body, and the link is labeled with the predecessor's title. For now, a successor supersedes exactly one predecessor.

## Accepting a successor

On acceptance, DADRock checks that the predecessor is accepted and not already superseded, adds the successor's `id` to the predecessor's `successors`, and moves the predecessor to `dadr/archive/` with its body unchanged. The section stays in the successor's body as the record of why the decision changed. It is a DADRock convention, not an additional required section of the DADR format.

For two records that are already accepted, `dadrock supersede --from OLD --to NEW` links them explicitly and moves `OLD` to `dadr/archive/`. Records linked this way have no `## Supersedes` section.
