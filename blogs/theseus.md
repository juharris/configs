# Keep Building the Ship

Teams often frame big technical decisions as a choice between starting fresh and living forever with the current system.
That framing hides the better option: change the system in place until it becomes the system the product needs.

Greenfield work feels clean because it has no scars yet.
It also has no production hardening, no hidden edge cases, no migration path, and none of the small capabilities users quietly rely on every day.

> The old system contains more knowledge than the team remembers.

A rewrite throws much of that knowledge away and asks the new project to rediscover it under pressure.
TK Add something about the whole is greater than the sum of its parts and why.

Prefer building in situ.
Change one part, preserve the capabilities that still matter, and let tests prove that the system keeps its promises.
TK Correct this AI 'it's not x; it's y' frustrating phrasing. This is a blog post; it's not a tweet.
This approach does not romanticize legacy code.
It treats the existing system as evidence.

## Ship of Theseus

TK Talk about how this relates to the Ship of Theseus, where replacing parts of a ship one by one raises the question of whether it remains the same ship. We're changing the system over time and even though it might have completely different components when we're done, it will still function with the same promises (TK Add more and better words).

## Let Tests Define the Contract

Tests should describe the behavior the system must keep.
Before a major redesign, strengthen the tests around the capabilities that users and other teams depend on.
Those tests give the team a way to change internals without arguing from memory.

TK Add a paragraph about the importance of end-to-end tests such as integration tests for a UI that calls a service.

Test-driven development helps here because it turns requirements into executable constraints before the implementation settles.
When the current system changes in place, the tests catch regressions.
When a new codebase becomes truly necessary, the same tests can guide the replacement.

Do not write tests merely to preserve every accidental behavior.
Write tests for the promises the system should keep.
That distinction matters because a good migration keeps product value, not every historical quirk.

TK Add examples of good overall tests vs. brittle tests.

## Use Libraries as Bridges

Sometimes a team does need a new project.
A fundamentally different architecture, runtime model, framework, or deployment boundary can justify that move.
Even then, the team can often avoid a cliff-edge rewrite.

Extract the shared capability into a library first.
Use that library in the existing system.
Battle-test it in production.
Refine the API while real callers depend on it.
Then reuse it if the new system still makes sense.

TK For Justin: Add a diagram showing 2 services in different languages sharing a Rust library.

Languages such as Rust can make this strategy especially useful.
A Rust library can package performance-sensitive or correctness-sensitive logic behind a stable interface and expose bindings to multiple host languages.
The team can modernize a core capability without forcing every surrounding system to move at once.

TK Avoid this AI phrasing. Probably just delete the next line, then add another after saying what the important move is.
The important move is not Rust itself.
The important move is creating a stable boundary that both systems can share.

## Microservices as Bridges

TK Explain how it's similar to how a library can be used as bridge.
Indeed there is a latency cost, but as explained above, it will be much easier and faster for development because the new code can be trusted via a gradual rollout.
TK Explain more about how it makes it easier to switch.

## Do Not Rewrite for an Org Chart

Conway's Law says organizations tend to design systems that mirror their communication structures.
That observation can explain architecture, but it should not excuse unnecessary rewrites.
Do not start a new codebase only because a new team will own the work.

Build the boundary that the product and the code need.
If ownership must become explicit, use lighter tools first.
One repository can still have clear folders, clear APIs, and code owners.
TK Add link to GitHub code owners documentation and a brief explanation about how to use it effectively.
GitHub's `CODEOWNERS` feature can require reviews from the right people for specific paths without forcing the team to split a working system too early.

Libraries and SDKs also help when ownership needs a stronger boundary.
They give teams a published interface, versioning discipline, and independent release cadence.
Use that cost when the boundary deserves it.
Do not pay it just to make the org chart look tidy.

## Start Fresh Only for Fundamental Change

Starting fresh can be correct when ... (TK explain in the first sentence)
A new framework, a different interaction model, a hard deployment separation, or a major change in data ownership can make the old structure fight every important decision.
When the foundation blocks the product direction, a new system may cost less than endless contortions.

Changing programming languages alone should not decide the question.
Teams can integrate new languages into existing systems through libraries, services, or narrow execution boundaries.
They should ask what capability the new language unlocks and whether that capability requires a new application.

The practical default remains simple: evolve the current system until the evidence says it cannot become what the product needs.
That evidence should come from constraints, not vibes.
