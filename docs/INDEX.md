# fragmentation docs

Content-addressed, arbitrary-depth, self-similar trees. Native git objects.

## Reading Order

1. **[What Fragmentation Is](WHAT-FRAGMENTATION-IS.md)** -- the data structure, the self-similar property, content addressing.
2. **[Witnessed](WITNESSED.md)** -- why the observer lives on commits, not content. What we got wrong and what we learned.
3. **[Modules](MODULES.md)** -- how fragment, store, walk, diff, encoding, and git compose.
4. **[Agent Guide](AGENT-GUIDE.md)** -- what future agents need to know that the code can't say.

## Why This Order

Start with what the thing is. Then understand the design decision that changed (Witnessed) -- why it moved from content to commits, and why that's right. Then learn how the pieces fit. Then read the guide for building with it.

Three core types, eight modules. Everything else is consequences.
