# R2 delta review lane template (to be filled per changed file group)

TASK: Review ONLY the uncommitted changes in the files listed below of the Rust repository /Users/indo/code/project/omon-gateway. Look for defects the change introduced or a defect it failed to fix. Return a findings report as your final response.

SCOPE (HARD BOUNDARY): `git diff -- <files>` is the review target. You may READ any file to confirm a finding. You are READ-ONLY: do not edit files, and do not run cargo or bun. Use only read, grep, ast-grep, lsp, and `git diff`.

FIXED DEFECTS TO RE-VERIFY (each must be truly fixed, with no regression):
<paste CONFIRMED-r1 entries for these files>

SEVERITY RUBRIC: identical to R1 (P0/P1 need a concrete reachable trigger; otherwise P2).

DELIVERABLE: `# Lane <name>`, then a `### [P0|P1|P2]` block for each finding, with Location / Trigger / Impact / Evidence / Fix. Then a `## Fix verification` list with one line per fixed defect: VERIFIED or NOT FIXED <reason>. If you find nothing, write exactly: NO FINDINGS (the Fix verification list is still required).

VERIFY: re-open every cited path:line before answering.
STOP WHEN: every changed hunk is reviewed and every fixed defect has a verdict.
