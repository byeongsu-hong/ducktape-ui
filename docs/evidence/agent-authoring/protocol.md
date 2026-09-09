# Agent authoring comparison protocol

This prospective comparison addresses G04 in the [UI quality worklist](../../ui-quality-roadmap.md).
It is a bounded usability observation, not a model benchmark or a claim that
documentation alone improves all UI tasks. Record unsuccessful and neutral
results as well as improvements.

## Conditions

The source baseline is `ffb89ea1920bb42ef529276495868708ad8210c4`.
Run each condition in its own task worktree, with a fresh-context agent using
the same model and reasoning settings. Both receive the request below, repository
instructions, and normal access to their checkout. The after condition changes
only the author guidance on that baseline; concurrent widget fixes must not enter
one condition alone. Do not supply solutions or another condition's output.

The coordinator may supply the checkout and an exclusive Cargo target directory,
and answer infrastructure questions. Record every such intervention. Record
build time separately: cache temperature and concurrent compilation are not UI
authoring skill. Do not impose an unreported time limit or count compiler errors
as structural UI mistakes. Keep source, logs, native captures and review findings
for both conditions. Review against the same criteria before giving repair advice.

## Identical request

> Improve the existing Settings example into a compact workspace preferences
> screen using Ice's default components. Keep the display name, email, workspace
> name and notification controls. Show a clear title and short explanation.
> At both 960×820 and 360×320, keep Save changes visible while the fields scroll;
> the last field must remain reachable. Let the workspace field have a visibly
> different corner radius and padding while keeping normal editing and keyboard
> behavior. Empty email should show useful wrapping feedback. Correcting it and
> saving should show success without losing other edits. The form must also work
> with an empty optional explanation and a long workspace label. Use the existing
> example's local save behavior; no service or persistence is requested. Verify
> the actual native layout and keyboard interaction, inspect rendered captures,
> and report the commands and results. Deliver a focused reviewable change.

## Review criteria declared before either run

| Area | Evidence required |
| --- | --- |
| Discovery | Existing default source is imported; no invented package import, copied catalog adapter or replacement design system. |
| Layout | Native captures and geometry at both sizes show inset content, bounded field scrolling, reachable last field and visible Save; labels and errors stay inside their bounds. |
| Customization | Workspace field changes radius/padding; actual typing updates its bound value and keyboard navigation still reaches Save. |
| State | Empty email produces feedback; correction and save clear it, show success and retain edits in other fields. |
| Content | Empty optional explanation leaves no reserved blank row; a long label wraps without obscuring the next control. |
| Verification | Assertions cover behavior and geometry, images are inspected, and any newly introduced regression assertion rejects its intended temporary behavior mutation before restoration. |

A structural mistake is a distinct unmet criterion observed in the submitted
UI or its evidence, not every compiler retry or every failing assertion reporting
the same cause. Count the initial submission and any coordinator-assisted repairs
separately. Record manual interventions verbatim with their purpose. Report time
from task dispatch to the first independently verified usable result, alongside
build/tool waiting time where measurable. If a criterion remains unverified,
report no verified result rather than replacing it with compilation success.

## Result record

The before condition is in progress in `.worktree/agent-authoring-before`
on `agent/agent-authoring-before`, using a fresh agent with inherited model and
reasoning settings and exclusive target `/tmp/ice-compact-input-layout-target`.
Dispatch occurred on 2026-09-09; the agent records exact task timestamps. The
after condition starts from `c4524da0` in `.worktree/agent-authoring-after`:
the same source plus six documentation files. It uses exclusive target
`/tmp/ice-adapter-state-updates-target`. The follow-up report must include both revisions,
agent settings, start/end timestamps, submitted commits, commands, capture paths,
criterion-by-criterion findings, structural mistakes and manual interventions.
One pair cannot establish a general causal effect; use it to identify the next
specific authoring obstacle. G04 remains open until the observations are recorded.

### Execution deviation

The fresh after-agent spawn was rejected with `agent thread limit reached`.
The after task therefore reuses the agent that completed custom interaction
contracts. Its prior repository experience differs from the fresh before agent.
Both receive the identical UI request and no other condition's solution, but
this is now a descriptive authoring observation, not a controlled measurement
of the guidance's causal effect. Report the experience and cache differences
alongside timing; do not attribute any speed difference solely to documentation.
