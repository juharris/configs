# Name Code After What It Does

Names carry design pressure.
A good name tells the next developer what a thing does and how to use it before they read the implementation.
A weak name leaks history, implementation detail, branding, or motivation into every call site.
That noise makes code harder to review, harder to change, and easier to misuse.

Name classes, methods, variables, and modules after their behavior.
Callers usually care about the promise a component makes, not the mechanism it uses today.
If the mechanism matters, keep that detail inside the implementation or inside the narrow integration layer that owns it.

TK Use Python for examples, not Ruby.

```Python
# Good
InvoiceTotalCalculator
RetryableRequest
SearchResultLimiter

# Usually too implementation-focused
RedisInvoiceTotalCalculator
ExponentialBackoffRetryableRequest
SqlSearchResultLimiter
```

The implementation-focused names may become correct when the implementation defines the public contract.
For example, `RedisCacheStore` makes sense when the caller deliberately chooses Redis-specific behavior implementing a `CacheStore` interface.
It does not make sense when the class merely happens to use Redis behind a broader cache interface.

Brand and product names follow the same rule.
Avoid them near domain logic.
Use them at integration boundaries where the name tells the truth about the contract.

```Python
# Good near an integration boundary
AnthropicClient
OpenAiClient
GitHubWebhookVerifier

# Usually too coupled for domain code
OpenAiSummaryGenerator
GitHubSubscriptionState
```

Domain code should speak the language of the product, not the vendor that currently powers it.
Names like `SummaryGenerator` or `SubscriptionState` leave room to change the implementation without changing every caller.
Names like `OpenAiSummaryGenerator` or `StripeSubscriptionState` force the vendor into places that should not care.

Avoid naming things after why they exist.
The reason for a change belongs in a comment that explains surprising context.
It's tempting to put reasons in pull request descriptions, commit messages, issues, but developers and AI will likely not dig that deeply most of the time to find context.
See [Agentic Coding Harness](./coding-harness.md) for guidance on how to structure documentation and guardrails to help both humans and AI understand the code.
The name should still describe behavior.

```ruby
# Good
FraudReviewQueue
PaymentRetryPolicy
CustomerVisibilityFilter

# Usually too tied to motivation or history
ChargebackFixQueue
Q3PaymentRetryPolicy
PrivacyLaunchCustomerFilter
```

Names should also avoid vague verbs.
Words such as `handle`, `process`, and `manage` often hide the decision the code makes.
Sometimes they fit a broad interface, but they should earn that breadth.

```TypeScript
// Better
parseWebhookPayload(payload)
sendPasswordResetEmail(user)
archiveExpiredSessions(now)

// Weaker
handlePayload(payload)
processUser(user)
manageSessions(now)
```

Clear names make comments lighter.
When a method name explains what it does, comments can focus on why the code takes a non-obvious path.
That separation matters.
Callers need the contract first.
Maintainers can read the implementation when they need the mechanics.
