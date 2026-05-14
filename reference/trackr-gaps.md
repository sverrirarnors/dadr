# trackr concepts not covered by DADR specs

Comparison of Becker, Moore & Lawrence (2017), *trackr: A Framework for
Enhancing Discoverability and Reproducibility of Data Visualizations and Other
Artifacts in R* (arXiv:1706.04440), against the DADR specs in `../specs/`.

The two systems have different goals — trackr is an **artifact-tracking and
discoverability** framework, DADR is a **decision-record** framework — so most
trackr features sit outside DADR's scope by design. This document catalogs
those features so the gap is explicit and so any future DADR work that wants
to absorb pieces of trackr can find them in one place.

## 1. Core paradigm gap

- **Artifact as a first-class record type.** trackr stores one record per
  computational artifact (plot, `lm`/`glm` fit, `data.frame`, dynamic report,
  vignette). DADR's `record` is always a decision; outputs appear only as
  activities (`write_figure`, `write_table`) or as evidence inputs.
- **"Recording" as an extension of saving.** `record(obj)` = save +
  auto-annotate + index. No DADR analogue.
- **Discoverability as the goal** (find a forgotten plot from last quarter,
  prep a group meeting, prep a manuscript). DADR's goal is governance and
  review.
- **Manual-recording-as-a-feature design principle.** trackr explicitly argues
  against auto-capturing every device draw because intermediate iterations
  pollute discovery queries; explicit `record()` declares importance. DADR
  has no analogous design conversation about artifact granularity.
- **Result gallery as a meta-visualization.** The body of stored artifacts is
  itself a useful visualization that reveals higher-order patterns.

## 2. Metadata categories trackr extracts

trackr extracts four categories of metadata per artifact (paper §2.1). DADR
does not model artifact metadata at all.

### 2a. Computational environment (per artifact)

- Submitting user + submission timestamp on each artifact.
- `sessionInfo()`-style structured capture (R version, package versions,
  locale) attached per artifact. DADR mentions runtime versions only inside
  the opt-in runtime-capture mode of Spec 09.
- Project-type detection: RStudio project root, R package directory.
- 128-bit `fastdigest` (SpookyHash) content-addressable artifact IDs
  (e.g. `SpkyV2_ce8a4f2e063ce6ca3c4a1e770cba1f8b`). DADR uses time-allocated
  UUIDv7 for records and sha256 for evidence fingerprints; rerunning analysis
  would produce a new UUIDv7, never recover the same id.

### 2b. Artifact design / structure

None of this is in DADR:

- Direct mining of ggplot2 plot objects: aesthetic mappings, geoms, stats,
  scales, facets, theme, coordinate system.
- Mining of lattice trellis objects: conditioning variables, panel functions.
- `recordedPlot` mining + generating-code analysis for base graphics.
- Per-artifact-class `FeatureSet` typed metadata containers
  (e.g. `GGplotFeatureSet`) with custom `show()` methods.
- Plot guides as semantic content: titles, axis labels, axis values, legend
  text — mined as searchable text.
- Plotting-system identity (ggplot2 vs lattice vs graphics) as a search axis.
- Model-fit structural metadata: model formula, link function, family, names
  of significant terms in `lm`/`glm`.
- The "extractor depth varies by source" tradeoff: high-level plot objects
  yield more metadata than `recordedPlot` from base graphics.

### 2c. Code-as-context for artifacts

Distinct from DADR's code-as-evidence-for-decisions:

- Generating-function name as a semantic clue (`hist`, `histogram`, `lm`,
  `glm`, `prcomp` as identifiers of artifact type/purpose).
- Upstream-call inference (`plot(density(x))` → density plot;
  `plot(residuals(fit))` → diagnostic plot, with the model's metadata
  propagated onto the plot). DADR has no "propagate metadata from antecedent
  objects to the artifact."
- String constants in code as searchable metadata (filenames, URLs, key
  function arguments).
- `CodeDepends`-style static dependency-chain capture as the runnable subset
  that regenerates the artifact.
- Per-artifact reproducibility script: the slice of code that, given the data,
  recreates the result.
- Packages loaded by the generating code, attached to the artifact.

### 2d. Data summaries (when data are accessible)

None of this is in DADR:

- Per-group medians extracted from boxplots.
- Plotted-point ranges / domain extracted from scatterplots.
- Numerical and categorical variable summaries embedded in the artifact
  metadata.
- Differentiating plots that show different *subsets* of the same dataset via
  summary fingerprints.
- Pulling plotted data out of `trellis`/`ggplot` objects automatically;
  falling back to `CodeDepends` heuristics for base graphics / generic R
  objects.
- User-provided supplemental annotations layered on top of automatically
  inferred ones.

## 3. Dynamic / woven reports as a tracked artifact class

DADR mentions Quarto only as an output format for DADR's own reports. It does
not treat woven reports as a tracked-artifact concept. trackr does:

- Reports as results in their own right with bidirectional discovery —
  find a plot, retrieve its containing report; query the report's text,
  retrieve the plots inside.
- Five metadata categories per report: environment, set-of-result-artifacts,
  descriptive text, full body of code, header/encoded metadata.
- Set-of-results summarization: how many results, how many of each kind,
  dependency relations among them.
- Whole descriptive-text indexing for backend full-text search (e.g. find
  reports whose narrative says "gene X was upregulated").
- Future-direction: key-phrase / concept extraction from descriptive text.
- rmarkdown/knitr header metadata propagation (author, title, destination
  format).
- R package vignette metadata (keywords, vignette package deps).
- `knit_and_record` end-to-end function that weaves, captures every displayed
  artifact to a temp backend, generates a report-level record, associates
  inner artifacts ↔ report by id, then commits all to the active backend.
- Auto-record-everything-displayed exception for dynamic reports: "if it's
  important enough to be displayed, it's important enough to record."

## 4. Expression-history capture (the `histry` package)

- A separately published abstraction for retaining successfully-evaluated
  top-level expressions, with pluggable trackers (interactive session,
  knitr/rmarkdown weave, potential Sweave tracker). Spec 09's runtime-capture
  mentions "executed cell/command text" but doesn't define a formal history
  abstraction or tie it to artifact recording.
- Two-step algorithm: (1) capture all successful top-level expressions;
  (2) filter via `CodeDepends` to the subset relevant to a given artifact.
- Guarantee that code is always available at record-time because `histry` is
  loaded automatically on `library(trackr)`.

## 5. The trackr API surface

DADR's CLI is for decision lifecycle. trackr's API is for artifact CRUD +
search and is wholly absent:

- `record(obj, ...)` — capture + annotate + persist an in-memory R object.
- `findRecords(pattern, ret_type=...)` — pattern search (often regex) across
  the metadata index.
- `rmRecord(obj_or_id)` — delete by object identity or by id.
- `listRecToFeatureSet(...)` / `show(fs)` — rehydrate a record into a typed
  metadata view.
- A search-by-metadata surface in general (keyword/regex across plot titles,
  variable names, geom types, dataset names, function names). DADR's
  `dadr find` matches UUID/prefix/title/filename only.

## 6. Per-class extensibility / extractors

- Customization of metadata extraction by R object class (paper §3.4): two
  mechanisms — customize the *tags* extracted from a class, or define new
  formal fields + a custom extraction method.
- Documented `Extending-trackr` vignette workflow.
- Default extractor as a fallback for arbitrary R objects.
- Specialized extractors shipped for plots, `data.frame`, `lm`/`glm` fits.

DADR's extension surface is templates, schema, and decision-detectors — not
extractors-for-objects-by-class.

## 7. Storage backends

trackr's backends are the canonical store for an artifact metadata index, not
a state mirror of Git:

- JSON file backend as default for single-user/single-machine use.
- Solr backend via the `rsolr` package for group/department scale.
- S4-generic-based backend abstraction; users extend by implementing methods
  on a small set of S4 generics.
- Documented user-defined custom backends.
- Configurable per-session backend swap.
- Default location is a "standard, user-specific location."

## 8. Graphical / collaborative frontends

Major sections of the trackr paper (§3.6) with no DADR analogue.

### 8a. Analyst-embedded frontend

- Shiny RStudio addin with text-query input and a thumbnail/gallery output,
  runnable inside the IDE.
- Result gallery as a usable visualization in its own right.

### 8b. Collaborative frontend (Blacklight)

- Out-of-the-box Solr image-gallery frontend (Project Blacklight) repurposed
  from libraries/museums for tracked artifacts.
- Auto-generated GUI faceting controls (e.g. by registration timestamp,
  package, sample size, username).
- Paginated thumbnail display.
- Bookmarking and query-history tracking.
- Solr's full search-syntax surface exposed to the user.
- Visual catalog navigation (browse all `diamonds`-using plots).

### 8c. Result feeds (real-time collaboration)

- RSS feed of new artifacts as a standard dissemination mechanism.
- Filtered/scoped feeds (per-user, per-tag, per-dataset) encoded in the URL.
- Auto-updating-search-as-feed abstraction.
- RSS used even in single-user mode to observe the history of an interactive
  analysis session.
- Streaming/real-time collaboration story. DADR has SSE for review events but
  not artifact-stream feeds.

## 9. R-ecosystem integrations

DADR is intentionally language-agnostic and does not commit to any of these:

- ggplot2 internal-API coupling (with the documented maintenance cost — paper
  §4.2.1 notes ggplot2 2.0.0 broke trackr).
- lattice internal-API coupling.
- Base R `recordedPlot` integration.
- knitr integration.
- rmarkdown integration.
- Sweave (potential, named).
- RStudio IDE integration via the addin surface.
- `fastdigest` for IDs.
- `CodeDepends` for static dependency analysis.
- `rsolr` for the Solr backend.
- `shiny` for the analyst-embedded UI.
- `archivist` named as related prior work for caching/restoring R objects with
  manual annotations.
- The R-parser-drops-comments fact, which motivates code-based metadata over
  comment-based metadata. DADR makes the opposite bet via comment decorators
  (Spec 10) — a philosophical conflict worth flagging.

## 10. Discussion topics DADR is silent on

### 10a. Prior-work framing

- Three-bucket prior-work taxonomy: history-capture in scientific viz
  platforms; provenance capture in scientific-workflow systems; data
  sharing/archival.
- Specific systems: GRASPARC, VisTrails, taverna, DataONE, Lee & Grinstein's
  ExBase, ProvStore, Figshare, Plotly, Open Provenance Model / W3C PROV.
- Granularity-of-history axis from formally-modelled high-level decisions down
  to mouse clicks/menu selections.
- "Metadata extracted by trackr could be submitted to external provenance
  stores (ProvStore) or sharing services (Figshare, Plotly)" interop vision.

### 10b. Limitations explicitly raised

- Internal-API fragility of plot frameworks as a maintenance cost the
  framework owners absorb.
- Infeasibility of auto-recording every device draw.

### 10c. Future directions

- Automatic plot-type classification (histogram vs. residual scatter vs.
  heatmap, etc.) via heuristics + statistical learning + text mining.
- Extension to interactive graphics: tooltips as direct visual representations
  of annotations; interactive controls as encoded analyst assumptions about
  which inputs matter.
- Snapshotting interactive-plot state (parameters chosen, current selection,
  data subset in view) as a fingerprint of "the question and answer."
- Transforming static graphics into interactive ones by combining captured
  metadata with a transformation engine (e.g. auto-adding a year-selector).
- Cross-artifact linking inference (deduce two plots use the same dataset →
  propose a linked combined plot).
- Annotating the analysis as a whole as a collection of artifacts, with
  inheritance of top-level annotations from constituent artifacts and
  identification of "representative" artifacts within the collection.

## 11. Identifier and addressing model

- Content-addressable artifact identity via 128-bit hash. trackr's hash gives
  "same artifact → same id" semantics; DADR's UUIDv7 is time-allocated.
- Versioned hash prefix (`SpkyV2_`) to allow algorithm migration.
- Identity-by-object-handle for deletion (`rmRecord(d)` resolves the live R
  object back to its stored id). DADR has no live-object → record handle.

## 12. Scale tiers

trackr's tiers are about *catalog* scale; DADR's tiers (Spec 13) are about
*review-workflow* scale.

- Solo analyst on one machine → JSON backend + Shiny addin.
- Research group / department → Solr backend + Blacklight gallery.
- Real-time collaboration / live-stream → RSS feeds.

## 13. Smaller specifics

- Per-artifact "tags" populated automatically (e.g. `ggplot, gg, ggplot` in
  the `show()` output). DADR tags are user/agent-applied to records.
- `location` field as a composite locator on every artifact (lines-of-code
  count, source filename, RStudio project, package name).
- `titles` field as a top-level searchable slot.
- `vars` field with role tags `<x>`, `<y>`, `<group.color>`.
- Distinction between "plot type" and "plotting system" as two separate
  metadata axes.
- Regex-vs-exact-match as a property of the backend, not the API.
- Solr-config recipe for an RSS endpoint.
- `set.seed(620)` treated as part of the captured generating expressions —
  seeds are first-class in the reproducible expression bundle.
- Explicit base-graphics fidelity-tier difference: "trackr is unable to
  extract as much information from base graphics."

## Summary of biggest gaps

1. No artifact-as-record primitive; DADR records describe decisions, not
   outputs.
2. No per-class typed extractors (`FeatureSet`/`GGplotFeatureSet`) that mine
   plot/model objects in memory.
3. No content-addressable artifact IDs.
4. No searchable metadata index with regex/full-text/faceted search.
5. No plot-object internal mining.
6. No data-summary capture (medians, ranges, variable summaries) on artifacts.
7. No expression-history abstraction with pluggable trackers and code-slice
   filtering for reproducibility.
8. No woven-report-as-artifact with bidirectional report↔artifact linking and
   a `knit_and_record`-style end-to-end function.
9. No graphical artifact-discovery frontend (no Shiny addin, no Blacklight
   gallery, no thumbnail-faceted browser).
10. No RSS / live-feed dissemination of new artifacts.
11. No pluggable storage backends with a generic dispatch surface; DADR's
    SQLite/Postgres are state mirrors, not catalog backends.
12. No R/RStudio/knitr/rmarkdown/Sweave integrations as first-class surfaces.
13. No automatic plot-type classification roadmap, no interactive-graphics
    support, no static→interactive transformation vision.
14. No archivist/Figshare/Plotly/ProvStore interop.
15. No manual-recording-as-explicit-importance-declaration design principle
    for outputs.
