# Mdow Pro Program Design

Date: 2026-07-10
Status: Approved in collaborative design review

## Executive Summary

Mdow Pro is a paid, individual-developer workspace that combines four capabilities:

- Bidirectional synchronization of Markdown and HTML folders across Mdow installations.
- Temporary sharing of a synchronized document through an expiring link.
- Managed document chat powered by Cloudflare Workers AI and paid through Mdow credits.
- A web account surface for authentication, billing, usage, devices, export, and deletion.

Mdow Pro is one product bundle, not four separately marketed products. The complete paid promise is
available before the commercial private beta, but engineering delivery is divided into independently
testable initiatives. This program design defines their shared boundaries. Each major initiative must
receive its own design specification and implementation plan before code is written.

The architecture is a transactional modular monolith on Cloudflare. A new Hono Worker hosts a
versioned oRPC API. PlanetScale Postgres, reached through Cloudflare Hyperdrive and Drizzle ORM, is
the canonical relational store. R2 stores immutable document bodies. Postgres transactions and a
transactional outbox establish durable state; Cloudflare Queues update derived systems such as
Vectorize and notifications. Durable Objects, KV, Workflows, AI Gateway, and external hosted model
providers are not launch dependencies.

## Product Goal

The first customer is an individual developer who keeps Markdown or HTML projects on more than one
computer, occasionally needs to share a document, and wants zero-setup AI assistance over a current
workspace.

The private-beta demand target is 50 paying subscribers within 90 days. This is the primary product
success metric. It does not weaken the release guardrails: synchronized content must have no
unrecovered data loss, managed AI spend must be bounded, and account export and deletion must work.

The architecture should comfortably support approximately 1,000 paying individual users without a
fundamental redesign. It should optimize for correctness and low operational burden rather than
speculative hyperscale.

## Commercial Package

Mdow Pro has one entitlement bundle:

| Item | Initial entitlement |
| --- | --- |
| Monthly price | USD $10 |
| Annual price | USD $100 |
| Active devices | 5 |
| Workspace count | Unlimited within storage and rate limits |
| Total version storage | 1 GB per account |
| Maximum document size | 10 MB |
| Maximum documents | 50,000 per account |
| Supported synchronized files | `.md`, `.markdown`, `.mdx`, `.html`, and `.htm` (case-insensitive) |
| Recoverable version history | 30 days |
| Share-link expiry | 1 hour, 24 hours, 7 days, or 30 days |
| Included managed AI | 1,000 Mdow credits per month |
| Intended included AI usage | Approximately 50 typical workspace questions |
| AI top-up | 1,000 credits for USD $5 |

Monthly credits expire at the end of their grant period. Purchased credits do not expire while the
account exists. Consumption uses expiring monthly grants first, then purchased lots in FIFO order.
The user sees Mdow credits, not raw tokens or a currency balance. Each approved Workers AI operation
and model has a versioned credit rate derived from measured input and output usage.

Polar is the merchant of record for subscriptions, invoices, taxes, refunds, and payment collection.
PlanetScale is the authority for Mdow entitlements, quotas, AI reservations, and the credit ledger.

## Launch Surfaces

### Electron Desktop

The cross-platform Electron app is the first Pro client. It adds:

- Email-code account connection through the system browser.
- Device registration and revocation state.
- Attaching an existing local folder as a synchronized workspace.
- Sync status, offline operation status, and per-file errors.
- Explicit conflict selection or manual merge.
- Recovery of document versions retained during the 30-day window.
- Temporary-link creation, revocation, mode, and expiry controls.
- Managed Workers AI chat over the current synchronized workspace.
- Selection between managed AI and the existing local ACP design.
- Credit balance and per-request usage feedback.

Local ACP remains available as a privacy and control alternative. It does not consume Mdow credits.

### Web

The existing TanStack Start web application adds:

- Email-code authentication.
- Polar checkout and customer-portal entry points.
- Subscription, entitlement, credit, and storage usage.
- Active-device management.
- Account export and deletion.
- Cancellation-grace status.

The web application does not become a cloud document library or editor. Temporary document views are
served through the new API Worker under a dedicated share route or hostname.

### Deferred Clients

The native macOS beta does not support Pro at launch. A full browser workspace, mobile client, and
other native clients are separate future initiatives.

## Scope Boundaries

The following are explicitly outside the first commercial scope:

- Teams, organizations, roles, shared workspace ownership, and collaborative editing.
- End-to-end encryption or per-workspace encryption modes.
- Images, referenced assets, PDFs, code repositories, archives, or general file synchronization.
- A public docs-site builder, custom domains, permanent public publishing, themes, or analytics.
- Automatic text merge or silent last-writer-wins conflict handling.
- A browser document library or editor.
- Native macOS Pro integration.
- External managed model providers, OpenRouter, BYOK, or AI Gateway routing.
- Pro plan tiers beyond the one individual subscription.
- A permanent free cloud-storage tier.

## Delivery Strategy

The destination is an all-in-one workspace. Delivery remains incremental behind feature flags:

1. Account foundation.
2. Trusted synchronization.
3. Temporary document sharing.
4. Managed AI and the credit ledger.
5. Commercial billing and private-beta operations.

Staff dogfood and an invite-only reliability beta may expose incomplete slices without charging.
The paid private beta begins only after the complete bundle is coherent.

## System Architecture

### Deployment Boundary

Add `apps/api` as one Cloudflare Worker application. Hono is the outer HTTP application and owns:

- Middleware, request IDs, CORS, security headers, and rate limiting.
- Better Auth routes.
- Polar webhooks.
- Health and operational endpoints.
- Temporary-share HTTP delivery.
- Mounting the oRPC product API.

oRPC owns the versioned, authenticated product procedures consumed by Electron and the web app. The
protocol must remain compatible with desktop versions already installed in the field. Breaking
changes require a new procedure or protocol version rather than changing an existing response in
place.

Queue consumers and scheduled reconciliation handlers may live in the same Worker deployment while
the traffic and operational model remain small. They must be modular units with independent tests and
idempotency boundaries.

### Cloudflare And External Services

| Service | Responsibility | Authority |
| --- | --- | --- |
| Cloudflare Workers | API, auth adapters, shares, AI orchestration, consumers, reconciliation | Request orchestration only |
| Hyperdrive | Pooled Worker connectivity to PlanetScale Postgres | Not a data authority |
| PlanetScale Postgres | Accounts, workspaces, heads, versions, shares, entitlements, ledger, outbox | Canonical relational authority |
| R2 | Immutable Markdown and HTML version bodies | Canonical bytes for accepted versions |
| Queues | Rendering, indexing, email, audit, cleanup, and other retryable side effects | Non-authoritative, at-least-once delivery |
| Vectorize | Workspace-scoped semantic retrieval candidates | Derived index only |
| Workers AI | Embeddings, optional reranking, and managed chat generation | Model execution only |
| Cloudflare Email Service | Login codes and account/security notifications | Delivery provider only |
| Turnstile and rate limiting | Bot and abuse mitigation | Never authorization or durable quota |
| Cloudflare observability | Worker logs, platform metrics, and operational analytics | Telemetry |
| Sentry | Worker exceptions and Electron errors/crashes | Telemetry |
| Polar | Checkout, payment, tax, invoice, refund, and subscription events | Payment authority |

R2 buckets remain private. Vectorize, Cache, Queues, telemetry, and local device state never determine
cloud authorization. Durable Objects are not a default authority; introduce them only when measured
contention demonstrates a need for per-workspace serialization beyond Postgres compare-and-swap.

### Repository Units

Keep shared packages narrow:

- `packages/contracts`: versioned oRPC contracts, Zod schemas, stable error codes, and transport-safe
  types shared by API, web, and Electron.
- `packages/sync`: pure operation types, revision/CAS rules, conflict state machine, and model tests.

Postgres schema and migrations live under `apps/api`. Electron's local SQLite schema and migrations
live under `apps/desktop`. The databases have different authorities and lifecycles and must not share
a schema package merely because both use Drizzle.

## Approved Dependencies

The design approves these dependencies or platforms beyond the current repository stack:

- Hono for the Worker HTTP shell.
- oRPC for versioned product procedures and typed clients.
- Zod for runtime request and response schemas.
- Drizzle ORM and Drizzle Kit for Postgres and local SQLite schema/migrations.
- Postgres.js as the Drizzle driver through Hyperdrive.
- Better Auth with its email OTP support.
- Polar for merchant-of-record commerce.
- PlanetScale Postgres.
- Sentry for correlated Worker and Electron errors.
- `rehype-parse`, `rehype-sanitize`, and `rehype-stringify` for Worker-side HTML sanitization.
- Electron's built-in `node:sqlite` through the applicable Drizzle adapter.

No other third-party dependency is implicit. Each implementation spec must ask for approval before
introducing another runtime or platform dependency.

## Canonical Data Model

The exact columns and indexes belong in the initiative specifications. The program requires these
domain records and invariants.

### Identity

- User and Better Auth records.
- Browser sessions.
- Registered Electron devices and rotated device credentials.
- Browser-to-app authorization attempts, PKCE challenges, single-use codes, and expiry.

### Workspace And Documents

- Workspace, owner, quota usage, and monotonic change sequence.
- Document stable identity, normalized relative path, current version, and deleted state.
- Immutable document version with content hash, R2 key, byte size, creating device, accepted sequence,
  and retention deadline.
- Device cursor and acknowledgement state.
- Idempotent sync operation receipt.

### Sharing

- Share token hash, target document, target mode, optional fixed version, expiry, revocation, creator,
  and audit timestamps.
- Snapshot mode always resolves the fixed version.
- Live mode resolves the current authorized document head at view time.

### Commerce And Credits

- Polar customer and subscription projection.
- Data-driven Pro entitlements even though only one plan ships.
- Immutable credit grant lots for monthly allowances and purchases.
- Credit reservations for in-flight AI requests.
- Append-only credit ledger entries for grant, reserve, release, consume, refund, adjustment, and
  expiry.
- Provider event receipts and reconciliation state.

### AI And Delivery

- AI request, stable client request ID, workspace, model/rate version, reservation, measured usage,
  final debit, status, and cited document versions.
- Transactional outbox event written in the same Postgres transaction as the canonical mutation.
- Consumer receipt or unique side-effect key for at-least-once processing.

## Authentication And Device Authorization

Sign-in uses email verification codes only. Passwords and social OAuth are not launch features.
Cloudflare Email Service sends login and security messages through a narrow internal mail adapter.

Electron opens the system browser for sign-in. The flow uses Authorization Code with PKCE (`S256`),
a fresh high-entropy verifier, exact state validation, and an `mdow://` callback. The callback carries
only a short-lived, single-use authorization code. It never carries access or refresh tokens. The API
binds the code to the authorization attempt, PKCE challenge, redirect URI, and client. Electron's main
process validates the exact callback shape before exchanging the code.

Electron stores device refresh credentials through Electron `safeStorage`. Access credentials are
short lived. Refresh credentials rotate and replay revokes the affected credential family. Users can
revoke any of their five active devices from the web account surface.

## Sync Protocol

### Local State

Electron's main process owns a Drizzle database over the runtime's built-in `node:sqlite`. It stores:

- Local workspace path and server workspace ID.
- Normalized file identity and fingerprints.
- Last accepted server version per document.
- Workspace change cursor.
- Pending operation journal with stable UUIDs.
- In-flight status, retry metadata, acknowledgements, and conflicts.

Renderer code never owns synchronization credentials or writes the local sync database directly.
Typed IPC exposes bounded actions and status.

### Push

1. Electron detects or scans a supported file operation and journals it before network work.
2. A mutation includes the operation UUID, document identity or create intent, normalized relative
   path, observed base version, content metadata, and body when applicable.
3. The Worker authenticates the device and checks workspace ownership, subscription state, quota,
   path rules, extension, size, case collisions, and current base version.
4. The Worker hashes the content and writes it to an immutable R2 key.
5. A short Postgres transaction compare-and-swaps the document head, increments the workspace change
   sequence, accounts for quota, records the operation result, and inserts an outbox event.
6. The response returns the accepted version and workspace sequence. Repeating the operation UUID
   returns the original result.

If the compare-and-swap sees a stale base, the mutation is not accepted. The API returns the local
base, current remote version, and metadata required for explicit resolution. Any unreferenced R2 body
is safe to reclaim later because no canonical record points to it.

### Pull

Each device requests changes after its workspace cursor. The API returns ordered accepted changes,
including creates, content updates, moves, and deletions, plus the next cursor. Applying a pulled
change and advancing the cursor is one local SQLite transaction. Duplicate pages and retries are
safe.

### Conflicts

A conflict blocks only the affected file. Other documents continue synchronizing. The UI preserves
both candidate versions and requires the user to:

- Keep the remote version locally.
- Confirm the local version as a new update based on the current remote head.
- Produce and submit a manually merged version based on the current remote head.

The first release does not automatically merge, create silent conflict files, or choose a winner.
Moves, deletions, offline scans, Unicode normalization, and case-only path changes receive explicit
protocol and cross-platform tests in the sync initiative.

## Versioning, Retention, And Deletion

Document versions are immutable. Replaced and deleted versions remain recoverable for 30 days. A
snapshot share may pin its target until the link expires if that is later than the normal retention
deadline. After all retention and share pins expire, cleanup may remove the Postgres metadata and R2
body.

When a subscription ends, the account enters a 30-day read-only grace period:

- New cloud sync mutations and managed AI are disabled.
- Local files remain untouched.
- Cloud download, export, and account access remain available.
- Resubscribing restores service without reattaching workspaces.
- At the end of grace, cloud content is deleted unless a legal or payment requirement prevents a
  particular record from being erased.

Account deletion is distinct from cancellation. It revokes sessions and devices, revokes shares,
stops processing, and schedules deletion of content and derived indexes. Financial records retain
only what Polar, tax, fraud, and applicable law require. The security initiative must define the
exact operational deletion proof and backup expiry process before launch.

## Temporary Sharing

Creating a link produces at least 256 bits of random entropy. The API returns the bearer token once
and stores only its cryptographic hash. The record defines an expiry preset and one of two modes:

- Snapshot: a fixed immutable document version.
- Live: the document's current head at request time.

Every view checks the token hash, expiry, and revocation in Postgres. Edge cache state never decides
access. Rendered output may cache by immutable version ID after authorization. Cache lifetime must not
extend the link's remaining lifetime.

Markdown is rendered through Mdow's approved server rendering path. HTML passes through
`rehype-parse`, a minimal audited `rehype-sanitize` schema, and `rehype-stringify`. The launch schema
excludes scripts, inline event handlers, forms and form controls, frames, objects, embeds, CSS/style,
SVG/MathML, and unsafe URL protocols. A restrictive CSP applies even after sanitization. Original
active HTML never executes in an Mdow origin.

True single-view links are not supported. Link scanners and browser prefetchers make them unreliable.

## Billing, Entitlements, And Credit Ledger

Polar events are inputs to a Mdow-owned projection. A webhook must be signature-verified and its
provider event ID claimed once before processing. Event handling updates the subscription projection,
entitlements, monthly grant, or purchased grant in one Postgres transaction. Delivery order must not
directly rewrite ledger history.

The credit ledger is append-only. Mutable cached balances may exist for efficient checks but must be
derivable and reconcilable from grants and entries. Reservations prevent concurrent AI requests from
overspending a balance:

1. Determine eligible unexpired grant lots in consumption order.
2. Atomically reserve the maximum permitted cost without allowing a negative available balance.
3. Execute the model once.
4. Finalize measured usage into consumption entries and release excess reservation.
5. On a known pre-inference failure, release the reservation.
6. On an uncertain inference result, do not automatically repeat model execution; reconcile the
   reservation and request status.

Periodic jobs compare Polar customers, subscriptions, purchases, refunds, and expected grants with
Mdow projections. Corrections use explicit adjustment or refund entries rather than editing history.

Polar's exact current webhook names, signature API, customer-portal behavior, and one-time checkout
mechanism must be verified against official documentation during the mandatory billing risk spike.
If Polar cannot meet the approved package, planning stops for a provider decision; implementation
must not infer undocumented behavior.

## Managed AI

Managed AI uses Cloudflare Workers AI only. Each chat is scoped to one current synchronized workspace.
The request path is:

1. Authenticate and authorize the workspace.
2. Claim the stable client request ID and reserve credits.
3. Retrieve workspace-filtered candidates from Vectorize.
4. Re-authorize every candidate document and exact version through Postgres.
5. Build bounded context and invoke the approved Workers AI model once.
6. Stream the answer and citations to Electron.
7. Validate citations against the authorized context packet.
8. Finalize credits and store usage metadata without storing document content in logs.

Vectorize is eventually consistent and is never an authorization source. Indexing is driven by
idempotent outbox events. Missing or delayed index entries reduce retrieval quality but cannot expose
another workspace.

Launch model IDs and credit-rate versions are operational configuration, not permanent product
contracts. The mandatory model evaluation chooses the initial chat, embedding, and optional reranking
models from Workers AI based on document-Q&A quality, latency, context limits, and measured cost. It
must calibrate 1,000 monthly credits to approximately 50 representative questions while preserving a
documented gross-margin ceiling.

## Security And Privacy

The service uses a conventional server-readable SaaS privacy boundary. Transport is encrypted and
provider storage encryption is enabled. End-to-end encryption is not promised. Product messaging
must state that synchronized content is processed by Mdow's cloud services and may be sent to Workers
AI when the user selects managed AI.

Required controls include:

- Derive the user from the verified session; never trust client-supplied owner IDs.
- Authorize workspace and document access for every procedure and derived lookup.
- Keep R2 private and avoid globally deduplicating content across tenants.
- Reject absolute paths, traversal, symlinks, unsupported types, oversized bodies, invalid Unicode
  normalization, and unsafe case collisions.
- Store share tokens, email codes, refresh credentials, and other secrets only in hashed or encrypted
  forms appropriate to their use.
- Apply Turnstile and rate limits to high-risk anonymous and email-triggering routes, while enforcing
  durable quotas in Postgres.
- Refuse to log document bodies, prompts, AI responses, email codes, bearer tokens, or credentials.
- Scrub file paths, email addresses, and user content from Sentry unless an explicit safe field is
  required.
- Maintain auditable device, share, billing, credit-adjustment, export, and deletion actions.

## Failure Handling And Reconciliation

No cross-service transaction spans Postgres, R2, Queues, Vectorize, Email Service, Polar, or Workers
AI. The design assumes duplicate requests, duplicate Queue delivery, delayed events, and partial
failure.

- Client operation IDs make sync retries return the original result.
- Immutable R2 keys make repeated body writes harmless.
- Postgres uniqueness and compare-and-swap protect document heads and grants.
- A transactional outbox records side-effect intent with the canonical mutation.
- Queue consumers deduplicate stable event or side-effect IDs, use bounded retries, and route final
  failures to a dead-letter queue.
- A scheduled reconciler republishes stranded outbox events and reclaims unreferenced R2 objects.
- Index reconciliation compares current versions with Vectorize projection state.
- Email intent is durable even though mailbox delivery cannot be exactly once.
- Polar reconciliation repairs missed or reordered webhook delivery.
- Credit reconciliation repairs stale reservations without silently repeating uncertain inference.

Transient Electron operations back off with jitter. Authentication, authorization, validation, and
quota failures do not retry blindly. The UI keeps actionable errors attached to the relevant
workspace, file, share, or AI request.

## Observability And Operations

Every request and asynchronous event receives a correlation ID that can cross Electron logs, Worker
logs, outbox events, Queue delivery, and Sentry without including user content. Operational dashboards
must cover:

- Auth email request, delivery, verification, abuse rejection, and callback exchange rates.
- Sync mutation latency, conflicts, failed operations, cursor lag, orphan cleanup, and quota usage.
- Queue age, retries, dead-letter count, and outbox lag.
- R2 storage and version-retention growth.
- Share creation, rejection, expiry, revocation, and sanitizer failures.
- Workers AI latency, failure, reservation age, credits per request, and cost by rate version.
- Polar webhook failures, projection lag, reconciliation differences, and grant adjustments.
- Export, cancellation, deletion, and restore outcomes.

Alerts require a runbook and an owner. Public rollout requires backup and restore exercises, deletion
evidence, a credit-adjustment procedure, support tooling for device and sync diagnosis, and explicit
monthly Cloudflare, PlanetScale, Polar, Sentry, and AI budgets.

## Testing Strategy

### Sync

- Model-based tests generate two-device operation sequences with online/offline transitions.
- Cover duplicate requests, delayed pulls, create/update/move/delete races, stale bases, and explicit
  resolution.
- Exercise Windows/macOS/Linux path separators, Unicode normalization, reserved names, case-only
  changes, and case collisions.
- Verify that unrelated files continue while one file is conflicted.
- Verify cursor application and local operation acknowledgement are atomic SQLite transactions.

### Database And Billing

- Transaction tests prove document-head CAS, quota accounting, outbox insertion, idempotency, credit
  grant uniqueness, reservation bounds, consumption order, refunds, expiry, and cancellation grace.
- Replay valid Polar events, duplicate delivery, delayed order, refunds, and reconciliation snapshots.
- Apply every migration to an empty database and a representative prior schema in CI/staging.

### Security And Rendering

- Authorization-matrix tests cover every oRPC procedure and share mode.
- Adversarial HTML fixtures cover scripts, event handlers, forms, embeds, style/CSS, URL schemes,
  malformed markup, SVG, and namespace tricks.
- Fuzz path normalization and callback parsing.
- Verify log and Sentry scrubbing with representative secrets and document content.

### AI And Asynchronous Work

- Duplicate and delayed Queue-delivery tests prove idempotent projections.
- Retrieval tests prove workspace filtering and post-retrieval reauthorization.
- Citation tests reject sources outside the authorized context packet.
- AI request tests cover reservation, stream cancellation, known failure, uncertain result, stale
  reservation, and finalization.

### End To End

Playwright and packaged-Electron checks cover email sign-in, browser callback, device revocation,
workspace attachment, two-device sync, conflict resolution, version restore, snapshot/live links,
checkout sandbox, allowance/top-up display, managed AI citations, cancellation grace, export, and
deletion.

Staging must exercise real Hyperdrive, PlanetScale, R2, Queues, Email Service, Vectorize, Workers AI,
and Polar sandbox behavior. Mocks alone are insufficient for a commercial release.

## Mandatory Risk Spikes

These are planning gates, not optional implementation chores:

1. **Electron SQLite:** prove the pinned Electron runtime exposes the required stable `node:sqlite`
   API through Drizzle in packaged macOS, Windows, and Linux builds. If it fails, return for explicit
   driver approval rather than silently adding `better-sqlite3`.
2. **Auth and Hyperdrive:** prove Better Auth email OTP, Postgres.js, Drizzle, PlanetScale Postgres,
   and Hyperdrive work under the Worker runtime. Prove PKCE, callback replay rejection, token rotation,
   and device revocation.
3. **Polar:** verify official subscription, annual billing, customer portal, webhook signatures,
   refunds, and one-time credit-pack checkout. Document exact supported events before schema design.
4. **Cloudflare Email:** verify account availability, sender-domain setup, arbitrary-recipient delivery,
   quotas, deliverability, bounce handling, and abuse controls.
5. **Workers AI and sanitization:** evaluate model quality, latency, cost, credit calibration, Vectorize
   indexing, and the rehype sanitizer's Worker bundle and adversarial behavior.

A failed spike changes the design before production implementation continues.

## Initiative Specifications

Do not produce one giant Mdow Pro implementation plan. Create and approve these specifications in
order, with separate implementation plans:

1. **Platform foundation and risk spikes**
   - `apps/api`, bindings, environments, contracts, migrations, outbox, observability, and all five
     risk-spike results.
2. **Identity and device authorization**
   - Better Auth schema, email OTP, PKCE callback, device credentials, safeStorage, revocation, and
     account UI.
3. **Sync protocol and local engine**
   - Canonical schema, local SQLite schema, operation protocol, pull cursors, paths, conflicts,
     retention, quotas, recovery, and cross-platform tests.
4. **Temporary sharing**
   - Token lifecycle, snapshot/live resolution, rendering, sanitization, CSP, caching, audit, expiry,
     and revocation.
5. **Billing, entitlements, and credit ledger**
   - Polar-verified event mapping, checkout, portal, grants, reservations, ledger, refunds,
     reconciliation, cancellation, and deletion interactions.
6. **Managed AI**
   - Indexing, retrieval, model evaluation, rate versions, streaming, citations, privacy, cancellation,
     and spend controls.
7. **Commercial beta and operations**
   - Packaging, production checkout, dashboards, alerts, runbooks, support tools, backups, restore,
     export, deletion, invite rollout, and the 90-day demand measurement.

## Rejected Alternatives

### Narrow Paid Wedge

Launching only publishing, private storage, or managed AI would reduce initial scope. It was rejected
because the selected product promise is the combined workspace. Engineering remains sliced even
though the commercial beta waits for the bundle.

### Durable Object Workspace Authority

Serializing every workspace through a Durable Object simplifies ordering but creates another
authoritative-looking state store without a transaction spanning R2 and Postgres. Postgres CAS is
sufficient for the initial scale. Durable Objects remain an evidence-driven future option.

### Event-Sourced Workspace Platform

An append-only workspace event system would improve replay and future collaboration but adds
projection, compaction, migration, and debugging work before demand is proven. Immutable versions,
operation receipts, workspace sequences, and an outbox preserve a later migration path.

### D1 Control Plane

D1 would maximize Cloudflare ownership, but the selected stack uses PlanetScale Postgres through
Hyperdrive for relational, sync, billing, and ledger state.

### Full Public Publishing

Permanent public sites, custom domains, navigation, themes, and analytics were rejected in favor of
expiring bearer links. This keeps the first sharing capability aligned with private developer work.

### Multi-Provider Managed AI

AI Gateway, OpenRouter, direct model providers, and BYOK increase model, billing, privacy, and support
paths. Workers AI is the only managed provider at launch; local ACP preserves user-controlled access.

## Design Completion Criteria

This program design is complete when it provides the stable constraints for the seven initiative
specifications above. It does not authorize implementation of the complete program as one task. The
next planning session should start with the platform foundation and risk-spike specification.
