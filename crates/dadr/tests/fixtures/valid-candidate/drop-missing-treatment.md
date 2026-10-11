---
title: Drop records with missing treatment
status: candidate
id: 01938b8e-4d70-7abc-8def-0123456789ab
tags:
  - kind/sample
  - scope/preprocessing
---

## Context

The treatment variable is missing for a small number of imported rows.

## Decision

Drop rows with missing treatment from the primary analysis sample.

## Alternatives

Keep the rows with an explicit missingness category.

## Consequences

The analysis sample is smaller, but the treatment contrast remains interpretable.
