---
name: Review implementation
description: Review a module implementing a paper from the prior-art/ directory
---

Your task is to review a module implementing a paper from the prior-art/ directory.

Apply critical thinking and reason from first principles to appraise the implementation.

Pay close attention to the stated goals of the paper vs the outputs of the implementation.

Favour fewer agents over many - look out for instances where agents might exist purely to keep track of state
and suggest how Agent state might be used instead.

If the outputs do not provide sufficient evidence to form an opinion, that is feedback in itself.

Review the implementation and provide suggestions for improvements in a markdown file inside the module.

## Diagram Requirements

After your written analysis, generate Mermaid diagrams to visualize the implementation architecture. Include these in a dedicated "## Architecture Diagrams" section near the top of the review (after the Summary).

Generate the following diagrams:

### 1. Agent Structure (Class Diagram)
Create a Mermaid class diagram showing:
- The `Agent<T, S>` trait from the DES framework
- All agent implementations in this module
- Their key fields (especially state variables and configuration)
- Implementation relationships

### 2. Event Flow (State Diagram)
Create a Mermaid state diagram showing:
- All Event enum variants
- Transitions between events (which events trigger which other events)
- Include annotations for which agents emit/handle each event

### 3. Agent-Event Relationships (Graph)
Create a Mermaid graph showing:
- Agents as nodes
- Events as nodes
- Directed edges showing "Agent X emits Event Y" and "Event Y handled by Agent Z"
- Use different shapes/colors to distinguish agents from events

### 4. Recommended Architecture (if applicable)
If you identify architectural improvements (e.g., consolidating agents, restructuring event flows), create a second set of diagrams showing the proposed changes. Use diff-style annotations:
- ❌ Red/struck-through for removed elements
- ✅ Green for new elements
- ⚠️  Yellow for modified elements

## Diagram Format

Use Mermaid syntax in markdown code blocks:

```mermaid
[diagram content]
```

Add brief explanatory text before each diagram explaining what it shows and key insights.

## Example Diagram Annotations

For identified issues, annotate diagrams with brief notes:
- "Too coupled" for tight agent coupling
- "No state" for stateless agents
- "Stateful" for agents with meaningful state
- "Event router only" for agents that just forward events

Review: $ARGUMENTS
