# Your first decision

After installation, this five-minute walkthrough takes you from an empty project to one accepted decision. You do not need an agent account or tags.

## 1. Install from source

Install Rust/Cargo and Bun, then clone this repository and build Dadrock:

```sh
git clone https://github.com/vincentarelbundock/dadr.git
cd dadr
DADROCK_REQUIRE_JS_BUILD=1 cargo install --path crates/dadrock --bin dadrock
dadrock --version
```

The first build downloads dependencies and may take several minutes. Bun is needed to include the web interface. These instructions use the source checkout; they do not depend on a published package or release installer.

## 2. Start in your analysis project

Change to the project where you keep your analysis, then run:

```sh
dadrock init
dadrock new "Exclude observations without treatment assignment"
```

`init` creates a `dadr/` directory without overwriting existing records. `new` prints the location of a candidate Markdown file and generates its ID.

## 3. Write the explanation

```sh
dadrock preview
```

Open the local URL printed in your terminal. Select your candidate and edit its four sections (or edit the Markdown file in your usual editor):

- **Context:** Some observations lack treatment assignment.
- **Decision:** Exclude those observations from this analysis.
- **Alternatives:** Recover assignment from the original log; explain whether this was possible.
- **Consequences:** Report how many observations were removed and consider whether missingness affects interpretation.

Replace these illustrative statements with what actually happened in your project. In particular, explain *why* you chose this approach.

## 4. Review and accept

Check both that the record describes your analysis accurately and that you endorse the rationale. Save your edits, then choose **Accept**. Your record moves to `dadr/accepted/`, keeping its ID. Commit the record with your analysis code using your usual version-control workflow.

You can also accept from another terminal with `dadrock accept ID`, replacing `ID` with the candidate's ID or an unambiguous prefix.

## 5. Find it again

```sh
dadrock list --status accepted
dadrock validate
```

Validation checks the records' structure and internal references. It does not establish that the analysis implements your decision correctly.

When your choice changes, use **Supersede** on the record in the web interface to prepare a replacement. Preserve the original explanation. See [the format in one page](core.md) for the lifecycle.

## Optional: ask an agent to draft

Once a local agent is [configured](llms.md), start with a small batch of consequential choices:

```sh
dadrock task run code-to-record --path analysis/ --limit 5
dadrock task run context-to-record --context "Document why we exclude rows without treatment assignment"
```

Agent outputs are proposals for review. Code can show what happened without establishing why; correct unsupported claims and supply missing rationale before acceptance. The web interface also offers drafting actions.

For a disposable example instead of your own analysis, run `dadrock demo --example candidate-review /tmp/dadr-example`, then `dadrock preview /tmp/dadr-example`. The demo requires a new or empty target directory.
