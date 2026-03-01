# fragmentation docs

Content-addressed, arbitrary-depth, circular-reflexive trees. Reality for git.

## Reading Order

1. **[What Fragmentation Is](WHAT-FRAGMENTATION-IS.md)** -- the data structure, the self-similar property, content addressing.
2. **[Witnessed](WITNESSED.md)** -- why the observer is part of the hash.
3. **[Modules](MODULES.md)** -- how store, walk, and diff compose on the core types.
4. **[Agent Guide](AGENT-GUIDE.md)** -- what future agents need to know that the code can't say.

## Why This Order

Start with what the thing is. Then understand the deepest design decision (Witnessed). Then learn how the pieces fit. Then read the guide for using it in practice.

The types are small enough to hold in your head. Four types, four modules. Everything else is consequences.
