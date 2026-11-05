---
name: explain-and-teach
description: Load before explaining how code, a system, a design, or a concept works, in any task, including chat answers to "how does X work", "why do we need X", "teach me X", or "walk me through X", and written explanations or docs meant to teach. Teaches in reading order with one concrete example, defines each term where it is first needed, never opens with a glossary, and stays short enough to read between interruptions.
---

# Explain and Teach

When the user asks how something works, teach it in the order the user reads it.
The user reads between interruptions, so each paragraph must make sense without scrolling back or ahead.
These rules come from instructional-design research, cited after the list.

- Open with one or two sentences on what the thing does and why it exists.
- Follow one small, concrete example through the system, step by step, and state the general rule only after the example shows it.
- Define each term in the sentence where the reader first needs it.
  Use plain words first, then the code's name: "each loaded copy of the config, which the code calls a generation".
- Never open with a glossary, a vocabulary section, or terms to learn before the content.
- Put each explanation next to the thing it explains, such as the table row, code line, or step, not in a separate section.
- Leave out everything the question does not need, even when it is true and interesting.
  Name what was left out in one sentence so the user can ask for it.
- Keep it short enough to read in a few minutes, and answer in chat instead of writing a long document unless the user asks for one.
- Do not explain what an experienced developer already knows, such as how a read-write lock or a bit array works.
- Verify every claim against the code before writing it.
  A short, correct explanation beats a long one that needs corrections.

## Evidence

- Defining terms during the task, instead of before it, avoids temporal split attention and improved learning in [Kester et al. (2001)](https://www.researchgate.net/publication/222527674_Just-in-time_information_presentation_and_the_acquisition_of_complex_cognitive_skills).
- Making readers mentally join separated sources adds extraneous load, per the [split-attention principle](https://www.davidlewisphd.com/courses/EDD8121/readings/2006-AyersSweller.pdf).
- Studying a worked example helps a reader new to a topic more than working it out alone, per the [worked-example effect](https://en.wikipedia.org/wiki/Worked-example_effect).
- Concrete examples that fade into the abstract rule transfer better than either alone, per [Fyfe et al. (2014)](https://www.researchgate.net/publication/262943993_Concreteness_Fading_in_Mathematics_and_Science_Instruction_A_Systematic_Review).
- Cutting extraneous material and splitting content into reader-paced segments help, per Mayer's [coherence and segmenting principles](https://www.researchgate.net/publication/23478495_Applying_the_Science_of_Learning_Evidence-Based_Principles_for_the_Design_of_Multimedia_Instruction).
- Guidance that helps novices can hinder experts, per the [expertise reversal effect](https://www.uky.edu/~gmswan3/EDC608/Kalyuga2007_Article_ExpertiseReversalEffectAndItsI.pdf).
- Mayer's pretraining principle says learners do better when they already know the names of a system's main parts.
  Read it as a few names introduced in passing, never as a vocabulary section.
