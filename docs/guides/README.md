# Guides

Guides are walkthrough-style pages: they connect the parts of Aphrodite the
way you use them, rather than document a single component. Reference pages
(architecture, proxy, plugin, classification) describe what each part does;
guides explain how those parts fit into a running Hermes Agent.

## Contents

| Guide                                                | What it covers                                                                                                |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| [Hermes Integration](hermes-integration.md)          | How the Aphrodite plugin connects to Hermes Agent: hooks, tools, runtime home, and why it beats a plain proxy |
| [Tool Output Schemas](hermes-tool-output-schemas.md) | How Hermes tool output is classified and previewed: the content-type pipeline and the 43-shape tool catalog   |

## Related

The guides defer to the reference pages for details:

| Reference page                                                               | Used by                                                   |
| ---------------------------------------------------------------------------- | --------------------------------------------------------- |
| [Plugin Hooks](../plugin/hooks.md)                                           | Hook-by-hook behavior behind the integration overview     |
| [Content Types](../classification/content-types.md)                          | The 30-type taxonomy behind the tool-output schemas guide |
| [Enriched Preview Catalog](../proxy/compression.md#enriched-preview-catalog) | The exact preview strings emitted for each content type   |
