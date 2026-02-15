---
name: Summarise paper
description: Summarise a paper and put the outputs in the prior-art/ directory
---

Your task is to summarise a research paper and write the output to the prior-art directory.

The research paper most likely describes some form of agent-based model along with some experiments
demonstrating one or more key findings.

You are to read the paper, understand the experiment set-up and key findings, then create a markdown
file containing a summary and instructions for implementations such that an intelligent
Large Language Model can implement the paper using the agent-based modelling techniques
in this project to recreate the key findings.

Focus on:
- Description of the main aims and findings of the paper.
- A detailed description of the experiment set-up and how this relates to the findings.
- Expected outcomes of the experiments.
  The expected outcomes will be used to iteratively check that the experiment(s) have been implemented correctly.

DO NOT focus on:
- Testing strategy.
- References to other papers.
- Writing code - pseudo-code can be helpful when explaining a particularly complex idea.

Summarise the paper in this path: $ARGUMENTS
