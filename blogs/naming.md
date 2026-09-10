# Name Code After What It Does

A name should tell callers what a component does and how to use it before they read the implementation.
Names become liabilities when they preserve temporary implementation details, historical reasons, or the name of a vendor that callers do not need to know about.

Name classes, methods, variables, and modules after their behavior.
Callers should depend on a promise rather than on the mechanism that currently fulfills it.
Keep details about the mechanism inside the implementation or in the integration layer that owns it using comments, private method names, or less prominent classes.

```Python
# Good class names
InvoiceTotalCalculator
RetryableRequest
SearchResultLimiter

# Avoid names tied to replaceable implementation details:
RedisInvoiceTotalCalculator
ExponentialBackoffRetryableRequest
SqlSearchResultLimiter
```

An implementation-specific name is appropriate when the implementation is part of the public contract.
For example, `RedisCacheStore` is useful when callers deliberately depend on Redis-specific behavior,
when the class sits at the Redis integration boundary,
or when implementing a `CacheStore` interface.
Callers should still depend on the `CacheStore` interface rather than the specific Redis implementation using techniques such as dependency injection or factory methods to obtain an instance.
The name is misleading when the class simply happens to use Redis behind a general cache interface.

Product and vendor names belong at integration boundaries where they identify the external contract.
They usually do not belong in domain code.

```Python
# Names that describe integrations
AnthropicClient
OpenAiClient
GitHubWebhookVerifier

# Names that couple domain code to an implementation vendor
OpenAiSummaryGenerator
GitHubSubscriptionState
```

Domain code should use the language of the product rather than the name of the vendor that currently powers it.
`SummaryGenerator` can change providers without changing every caller.
`AnthropicSummaryGenerator` makes the provider part of a domain-level dependency even when that dependency has no business meaning.
It's also a confusing name because it sounds like it generates summaries of Anthropic, which doesn't make sense.

Names should describe behavior instead of the reason a change happened.
Names such as `ChargebackFixQueue`, `Q3PaymentRetryPolicy`, and `PrivacyLaunchCustomerFilter` preserve temporary context that will become misleading.
The reason may still matter, so record it in a nearby comment, docstring, test, or design document.
Pull requests, commit messages, and issues provide useful history, but they should not be the only place to look for a reason that affects future maintenance
because developers and AI will likely not dig that deeply to find context.
See [Agentic Coding Harness](./coding-harness.md) for more on keeping that context easily available to people and AI agents.

Names should also make the operation specific enough to review.
Avoid vague words such as `handle`, `process`, and `manage` which hide the decision the code makes.
They can fit a genuinely broad interface, but a narrower verb usually gives callers more useful information.

```Python
# Better
parse_webhook_payload(payload)
send_password_reset_email(user)
archive_expired_sessions(now)

# Weaker
handle_payload(payload)
process_user(user)
manage_sessions(now)
```

Clear names reduce the need for comments.
When a method name explains what it does, a comment can explain why it takes a surprising path rather than repeating the method's behavior.
Callers can understand the contract first and read the implementation if they are curious about the mechanics
and reasons why the code makes the decisions it does.
