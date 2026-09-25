# Nous Portal - authenticating third-party apps against the subscription

Use when the user asks whether an external app (Karakeep, OpenWebUI,
LibreChat, OpenViking, LangChain pipeline, n8n flow, etc.) can use their Nous
Portal subscription without copy-pasting an API key - ideally via the Portal
login they already have.

The honest answer has three architectural layers people conflate. Walk through
them in order before proposing solutions.

## Layer 1 - Is this thing a Hermes plugin, or a separate app?

This is the question to answer FIRST. The "OpenViking" case in particular
trips agents up.

| Surface                                                      | What it actually is                                                                                          | Auth path                                                                                                                                                      |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **OpenViking memory plugin** (`plugins/memory/openviking/`)  | Code that runs **inside the Hermes process**. Its LLM calls go through Hermes's already-configured provider. | Already uses Portal if user's Hermes is configured for Portal. Nothing extra needed. `OPENVIKING_API_KEY` is the OpenViking _server's_ own auth, not LLM auth. |
| **OpenViking the standalone server** (separate container)    | A separate context-DB service. If it ever calls an LLM on its own, that's a separate HTTP client.            | Same as any external app - Layer 2/3 below.                                                                                                                    |
| **Karakeep, n8n, LibreChat, OpenWebUI, any self-hosted app** | Different process, often different machine. Makes its own HTTPS calls to `inference-api.nousresearch.com`.   | Layer 2/3 below.                                                                                                                                               |

**Pitfall to avoid**: do not pitch "OAuth into Portal" as the solution for a
plugin that already runs inside Hermes - including an Aphrodite plugin such as
the CCR engine. That LLM call is already authenticated via Hermes's provider
config. The plugin's own server auth (e.g. `OPENVIKING_API_KEY` for talking to
the OpenViking REST API) is unrelated to Portal.

## Layer 2 - For genuinely external apps, what does Portal actually expose?

Portal at `https://inference-api.nousresearch.com/v1` is an OpenAI-compatible
inference endpoint. It accepts **bearer-token authentication only**: either

1. **A static API key** from `portal.nousresearch.com → API Keys`, or
2. **An x402-protocol payment header** (Solana USDC, beta, anonymous, per-request).

There is **no general OAuth 2.0 authorization server**. There is no
"Sign in with Nous Portal" SSO that third-party apps can register as clients
against. There is no shared cookie or session that browser-Portal-login
extends to other apps on the same machine.

What Hermes Agent has that _feels_ like OAuth - `hermes login --provider nous`
opening a browser, user signs in, token lands in `~/.hermes/auth.json` - is a
**Hermes-specific browser flow**. Under the hood it produces a credential
Hermes uses as a bearer. It is not a public OAuth provider that Karakeep et al.
can implement a client for, because it isn't an OAuth provider at all from the
outside.

## Layer 3 - Can we bridge the gap without Portal changing anything?

Yes. The pattern is a **local credential-broker proxy**. Even without a public
OAuth flow, an app on the user's machine can:

1. Read Hermes's existing Portal credential out of `~/.hermes/auth.json`.
2. Expose a local OpenAI-compatible endpoint at `http://localhost:NNNN/v1`.
3. Forward incoming requests to `inference-api.nousresearch.com/v1` with that
   bearer attached.

Karakeep/OpenWebUI/etc. then point at `http://localhost:NNNN/v1` with any
placeholder key. The user never copies their Portal key around - the proxy
rides on the credential Hermes already holds.

Where this could live in Hermes:

- `gateway/platforms/api_server.py` is the precedent - it exposes the agent
  over a local OpenAI-compatible endpoint, but routes through the full agent
  loop (tool calls and all). The proxy variant is **pure inference
  pass-through**: no agent loop, no tools, just forward `/chat/completions`
  upstream with the user's stored Portal bearer.
- ~150 lines as a new gateway adapter or a plugin under `plugins/`.
- Token refresh: if the browser-OAuth flow produces a refreshable token, the
  credential pool's refresh logic already exists. If it's a long-lived static
  bearer, even simpler.

In the Aphrodite monorepo, keep this broker **out of the compression engine**:
`crates/aphrodite` exists to compress tool output (the CCR proxy), not to hold
third-party credentials. The broker belongs in the Hermes plugin or gateway
adapter layer - never in `~/.hermes/aphrodite/aphrodite.toml`.

This is genuinely useful and worth shipping - it's the answer to "use my
Portal sub with $external_app without copy-pasting keys."

## Real OAuth provider on Portal - when is it worth pitching?

Only when the consumer is _another first-party Nous thing_ (a future SDK, a
Nous-branded extension, a Discord-bot integration that needs per-user
delegation, etc.). Pitching it as the answer to "use my Portal sub with
Karakeep" is selling the user a thing that won't reach them: even if Portal
shipped OAuth tomorrow, Karakeep's LLM-provider config UI is `base_url +
bearer_token` with no OAuth client, no callback handler, no token refresh.
The OpenAI ecosystem standardized on static bearers and downstream apps
won't rebuild their config UX to accommodate a new auth flow.

The features that would actually help users today, and that Portal could ship
without depending on third-party app changes:

- **Scoped, named, revocable API keys** with last-used timestamps. Same UX
  benefits people want from OAuth (revoke a compromised key, see what's using
  the sub, scope a key to specific models), in a shape every existing app
  already supports.
- **Per-key rate limits** so a noisy app can be capped without eating the
  user's headroom for Hermes itself.

## Talking-points cheatsheet (for next time)

When the user asks "can $APP use my Portal subscription":

1. First decide: Hermes plugin (runs inside Hermes) or separate app? If plugin,
   it already uses Portal via Hermes's provider config - done.
2. If separate app: today, paste the static API key from Portal → API Keys.
   Base URL `https://inference-api.nousresearch.com/v1`. Rate limits are
   subscription-tier based, applied per-key.
3. If the user pushes back with "but I don't want to paste a key" - that's
   the local-broker-proxy answer (Layer 3). Worth building. Not a Portal-side
   OAuth roadmap problem.
4. Mixed setup ("Portal for some things, OpenRouter/Ollama Cloud for the
   Hermes agent itself") is fully supported. Hermes treats agent
   provider/model and tool-side LLM calls as independent config; you can
   point each at a different endpoint.

**Note on the Tool Gateway**: the "no separate accounts, no API key juggling"
pitch in the Tool Gateway announcement is specifically about Hermes Agent's
_tools_ (web search, browser, image gen, TTS) flowing through the Portal
subscription when Hermes is configured to use Portal as its provider. It is
**not** a claim that arbitrary third-party apps inherit Portal auth.

**Stop if** the user proposes storing an external app's key in
`~/.hermes/aphrodite/aphrodite.toml` - that file configures the CCR engine;
credentials belong in Hermes's auth/provider config, not the engine config.

**Recovery** - permitted: keep the key in `~/.hermes/auth.json`
(Hermes-managed) and let a local broker proxy ride on it. Prohibited: pasting
Portal keys into plugin source or fixtures.

## Verification

| Claim                                   | Test                                         | Pass condition                              |
| --------------------------------------- | -------------------------------------------- | ------------------------------------------- |
| Plugin-in-Hermes needs no extra auth    | Classify the app against Layer 1             | Plugin row → "already uses Hermes provider" |
| `hermes auth add nous` writes the token | Run it, then check the credential exists     | Present without printing the token          |
| Engine config stays credential-free     | Inspect `~/.hermes/aphrodite/aphrodite.toml` | No API keys or tokens in the file           |
