# Honest Self-Assessment Methodology

## When to Use

- User challenges your work with "re-inspect", "did you actually do that?", "you skipped this"
- After completing a major batch of work
- Before starting new work: check for gaps first
- When a plan has 100+ tasks and you need to track progress

## The Method

### Step 1: Go through EVERY task in the plan

```
For each task:
  - [ ] Not done — no code changes, no research
  - [~] Partially done — some code, incomplete
  - [x] Done — code committed, tested, released
  - [SKIP] Skipped — with documented reason
```

### Step 2: For each SKIP, ask hard questions

- "Did I actually research this or just hand-wave?"
- "Would this take 5 minutes or 5 hours?"
- "Is there an easy path I missed?"
- "What would a real engineer do?"

### Step 3: Categorize by effort/impact

```
Immediate (high impact, low effort): fix now
Medium (moderate effort): plan for next session
Architectural (high effort, high value): research + subtask breakdown
```

### Step 4: Write honest-gaps.md

Save to `.hermes/plans/honest-gaps.md` with:
- Task-by-task status
- Each skip explained with actual research
- Effort/impact categorization
- Concrete next steps

## Red Flags (signs you're lying to yourself)

- "This is fine as-is" — without checking the code
- "Too architectural" — without trying
- "Already handled by X" — without verifying X actually handles it
- "Would break too much" — without checking what breaks
- "Not worth it" — without measuring impact

## Pattern from This Session

I skipped 20 out of 100 tasks. When called out, I researched each one properly:
- 7 were genuinely pre-existing (upstream code)
- 3 were genuinely architectural (dual mode, shared CCR)
- 10 were gaps I should have fixed but didn't

The honest assessment turned "I think it's fine" into "here's exactly what needs doing."
