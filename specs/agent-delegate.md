# Agent delegation contract

These rules govern the prompt delivered to an execution agent.

DELEGATE-1: Present sections follow Context, Read first, Task, Acceptance criteria, Reviewer checks, Out of scope, Files to modify, Scoring, Environment notes. Prior attempts and What was actually implemented are outside this ordered list; Instructions follows Environment notes. Scoring retains the [D:_/B:_/U:_ → Eff:_] glyph shape.

DELEGATE-2: Optional sections are omitted when empty: Read first, Task, Acceptance criteria, Reviewer checks, Out of scope, Files to modify, Prior attempts and What was actually implemented. Files to modify falls back to module when the explicit list is empty.

DELEGATE-3: Acceptance criteria use checked boxes for done tasks and unchecked boxes for other statuses.

DELEGATE-4: Context always includes Target repo, using task.target_repo when explicit and the roadmap project otherwise.

DELEGATE-5: Per-agent environment footers describe execution runtime without naming a language, toolchain or package manager.
