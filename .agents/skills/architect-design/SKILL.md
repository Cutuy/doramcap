---
name: architect-design
description: Discuss architecture and engineering plans interactively through SVG diagrams, alternatives, recommendations, and one-decision-at-a-time Q&A. Use when the user wants to explore or choose a design before implementation.
---

# Architect Design

Help the user make engineering decisions through a visual conversation. Keep the
discussion grounded in the actual system and preserve their decisions across turns.

## Ground the discussion

- Read the referenced PR, plan, or relevant code before proposing alternatives.
- Identify the intended outcome, constraints, existing decisions, and unresolved
  choices. Ask only for context that available sources cannot supply.
- Begin with the decision that most shapes the remaining design. Continue one
  important decision at a time unless the user requests a broader comparison.

## Present a decision

- Offer a small set of meaningful alternatives, usually two or three. Include the
  current approach when useful. Do not manufacture alternatives to fill a quota.
- State your recommendation and the concrete reason it fits the user's constraints.
  Prefer simpler behavior and fewer moving parts when the evidence supports them.
- Explain each option's consequence for the system: data flow, ownership, timing,
  failure behavior, complexity, or cost. Focus on the tradeoffs that change the choice.
- Distinguish observed results, replay estimates, assumptions, and pending validation
  targets. Do not present a configured timer or a proposed test as a proven guarantee.
- When a choice introduces a dependency, compare suitable established libraries with
  the custom approach before recommending adoption.

## Make the comparison interactive

- Use SVG to show the architecture or behavior. When the `visualize` skill is
  available, read and follow it for the interactive rendering and host integration.
  Prefer an HTML fragment containing responsive SVG and native choice controls.
- Let selecting an option update the diagram and its essential tradeoffs locally.
  Keep comparisons readable, with short labels and enough context to decide.
- Provide a clearly labeled action to submit the selected choice and continue the
  conversation. Use `window.openai.sendFollowUpMessage` when supported. Include the
  actual selected option, previous accepted decisions, and the next question.
- Offer a way to explore the selected option's tradeoffs before choosing it.
- Persist useful presentation state through the host's supported state interface.
  Treat a local selection as a preview; acceptance requires a user message or an
  explicitly submitted choice. Never default a recommendation into user approval.
- If interactive rendering is unavailable, show a static SVG and ask for the choice
  in chat. Keep the same decision workflow.

## Continue the Q&A

- Acknowledge each accepted choice briefly and carry it into subsequent decisions.
- Track accepted choices, unresolved questions, and validation still needed. Keep
  this record in the conversation; write to a plan only when authorized.
- When the user asks a follow-up, answer it within the current decision before
  advancing. Let them revise earlier choices and explain affected downstream choices.
- Surface unresolved design details when they matter, including resource bounds,
  failure handling, and compatibility. Avoid adding a generic checklist to every turn.
- A design preference authorizes discussion of its implications. Implementing code,
  editing a PR, or publishing a plan requires the scope established by the user.
- When the requested discussion is complete, summarize the agreed design, remaining
  decisions, and validation needed. Offer the next concrete step without starting
  implementation solely because the discussion reached agreement.
