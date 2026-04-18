# AI Review Guidelines

Keep this file concise. GitHub Copilot code review only reads the first 4,000 characters of a custom instruction file. If review guidance becomes path-specific, move it into `.github/instructions/**/*.instructions.md` instead of growing this file.

Rust-specific review guidance lives in `.github/instructions/rust.instructions.md`.

When reviewing code in this repository:

- Treat AI review as advisory and supplemental. Do not act as the sole approver for risky changes.
- Start with functional evidence first: build errors, test failures, static analysis, dependency risks, and security issues.
- Review changes against the repository's stated architecture and intent in `README.md`, `IMPLEMENTATION_PLAN.md`, and `docs/*.md`. Flag drift from documented design, not from guessed preferences.
- Prioritize high-signal findings: correctness, restore fidelity, event ordering, idempotence, backward compatibility of proto/store/snapshot formats, local-vs-remote symmetry, resource leaks, and security.
- Be skeptical of new dependencies. Comment on necessity, maintenance, and license fit when a change introduces new packages or services.
- Avoid style-only comments unless they affect readability, maintainability, or correctness. Formatting and low-value nits should be left to automation.
- If a pull request is large or spans multiple concerns, lower confidence explicitly and prefer recommending a split or focused follow-up review over speculative comments.
- If a comment depends on an assumption, state the assumption clearly instead of presenting it as a fact.
- Prefer a few precise, actionable comments over many low-confidence suggestions.
- Do not invent repository rules that are not written here or in the project docs. If a recurring pattern deserves enforcement, suggest adding or refining repo-local instructions.
