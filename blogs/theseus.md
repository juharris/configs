Start fresh vs. morph the system into what you need.
It's better to build in situ than tabula rasa.
It's better to build the system into what you need so that you can adapt it over time.
The new system and code will lack the important capabilities of the old one.


## Test driven development

Use test driven development to ensure the system meets your requirements and maintains its capabilities over time.
Those tests can be used for a new codebase if truly needed.

## Use libraries

If a new project will be needed for fundamental reasons such as a new architecture or framework, it may justify starting fresh.

Libraries backed by low-level languages such as Rust can be built to bridge the gap between different systems.
Start building what is needed in both systems into a library and use it in the existing codebase so that the new library can be battle-tested in production and refined.
Then if a new project is needed later the library can be reused and the new project will be easier to trust.

## Before of Conway's Law

Build what makes sense.
Don't start new because a new team will maintain it.
TK Add link and brief explanation about Conway's Law.

Libraries and SDKs can be used if code really should have strict owners that are hard to maintain in our repository.
Specific folders can also be used in one repository with code owners set up.
TK Add link to GitHub code owners documentation and a brief explanation about how to use it effectively.

## When to start fresh

Fundamental new paradigms such as a new development MVC framework may justify starting fresh.
TK Add some details.

Only changing languages shouldn't mean starting fresh immediately.
Libraries backed by low-level languages such as Rust can easily be integrated into existing projects without needing to start fresh as mentioned above.
