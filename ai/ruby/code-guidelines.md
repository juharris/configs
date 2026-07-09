# Ruby Code Guidelines

- Avoid `Struct` and `Data.define` for Ruby value objects because their constructors and typing surfaces are easy to misuse.
  Prefer explicit classes with typed readers with documentation (if necessary for more complex use cases) and clear constructors.

- Prefer positional arguments over keyword arguments for constructors in Ruby classes where call sites are specific and do not need flexible named options.
  Positional arguments are slightly faster than keyword arguments.
  Keyword arguments are an older style that were used when type safety was not enforced.
  With type checking calls are much safer so we don't need to use keyword arguments for safety and we can use positional arguments for clarity and speed.
  Keyword arguments are still useful for optional arguments and for cases where the call site is not specific and needs flexible named options with callers in many places.
  For example, for private methods, positional arguments are almost always preferred.

- Avoid unnecessary `to_s` conversions in Ruby.
  `to_s` hides `nil` by converting it to an empty string, which is misleading and can mask the real issue.
  Prefer passing the original value through or validating the expected type explicitly.

- Never use the patten of `Class.call(...)` which does `new(...).execute`
  because it's wasteful and creates stateful classes with complex coupling.
  Encourage callers to instantiate a class and use the method(s) they need directly.
  This matches a dependency injection pattern used in many languages where processor singleton classes are re-used.

## Type Checking

Prefer `# typed: strict` at the top of files.
Using `strict` instead of `true` helps enforce full proper typing and checks in a few places such as for instance variables of classes.
See https://sorbet.org/docs/static for details.
Question changes that reduce type strictness without a clear reason because it's easy to reduce the strictness and often tempting for AI to get lazy and reduce strictness.
Then people will inevitably make changes that will make it even harder to increase strictness later.
