
## Reading

[ABMs in economics and finance (Axtell and Farmer, 2025)](https://ora.ox.ac.uk/objects/uuid:8af3b96e-a088-4e29-ba1e-0760222277b7/files/s6969z182c)

## Claude Code Configuration

TODO: Set up custom instructions to optimize for planning and research translation

1. **Create CLAUDE.md** - Run `/init` and customize with:
   - Project purpose: "Translate ABM research papers into modular Rust simulations"
   - Core architecture: Explain DES framework and modular example pattern
   - Research focus areas: List papers/models (Axelrod, Kirman & Vriend, ZI traders, etc.)
   - Working style: "Focus on architectural design and research translation, not syntax"
   - Domain terminology: Define agents, events, simulation mechanics

2. **Create custom slash commands** in `.claude/commands/`:
   - `/paper-simulation` - Paste paper excerpt, get simulation design outline
   - `/agent-design` - Design agent state and behavior patterns
   - `/framework-review` - Review how new examples integrate with DES core

3. **Session start template**: Provide paper section, key agents, event types, metrics
   - Ask architectural questions: "How should I structure agents for X behavior?"
   - Not: "Can you implement this?"

4. **Commit configuration to git** (team-shared)

## Recreate

TODO:

- The Evolution of Cooperation (Axelrod, 1984)
- TRANSIMS code (Barrett et al., 1995, Nagel, Beckman and Barrett, 1998)
- drug addiction (Agar and Wilson, 2002, Hoffer, Bobashev and Morris, 2009, Heard, Bobashev and Morris, 2014)
- Kirman and Vriend (2000, 2001) - fish market, loyalty
- policy relevant and exercised to study policy alternatives (Dawid et al., 2012)
- Donier et al. (2015) showed that a linear virtual order book profile
- Aymanns et al. (2016) leverage cycles
- "zero-intelligence" (ZI) agents (Gode and Sunder, 1993, 1997)
- K-level cognition (Camerer, Ho and Chong, 2004) has found use in ABMs (Latek, Kaminski and Axtell, 2009)
