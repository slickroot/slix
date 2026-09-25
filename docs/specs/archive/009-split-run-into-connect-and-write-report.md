# 009 - Split `run` into `connect` and `write_report`

## Technical Design

Purely technical — no user story. `run` in `src/main.rs` currently takes 9
parameters because it welds two responsibilities together: authenticating
(check existing connection, reconnect/PIN-flow on failure, build the
`XApiClient`) and reporting (already isolated in `write_report`). Splitting
by responsibility fixes the parameter count without resorting to grouping
unrelated params into structs.

### `main.rs`

- New `connect` function, next to `run` and `write_report`:

  ```rust
  fn connect(
      config: config::Config,
      api_base: &str,
      oauth_base: &str,
      path: &std::path::Path,
      input: impl std::io::BufRead,
      mut output: impl std::io::Write,
  ) -> Result<(config::Config, api::XApiClient, api::Me), Box<dyn std::error::Error>>
  ```

  Owns everything `run` currently does *except* calling `write_report`: the
  `is_connected` check, the `users_me` probe and its "reconnecting…"
  fallback, the PIN flow (`authorize_url`, reading a PIN line from `input`,
  `exchange`), saving the updated `config` to `path`, and building the
  `XApiClient` — exactly once per branch, at the point it's known which
  tokens to use. Returns the (possibly updated) `config` along with the
  authenticated `XApiClient` and `Me`.

- `run` becomes a thin wrapper: calls `connect(...)`, then
  `write_report(&api, &me, history, today_goal, now, &mut output)` with the
  result, and returns the updated `config`. Keeps its current 9-parameter
  signature so `main` and the existing tests have one call site.

- `api_base` and `oauth_base` stay as two separate `&str` params (not
  grouped into an `Endpoints` struct) — they're used by two unrelated
  constructors (`XApiClient::for_endpoint`, `PinFlow::for_endpoints`) and
  grouping them wouldn't reflect a real relationship between them.

- `path` stays a separate parameter, not a field on `config::Config` —
  `Config` stays a pure data struct; `path` is I/O plumbing the tests need
  to redirect independently of `Config`'s fields.

- `connect` stays in `main.rs` rather than its own module — it's small and
  already sits next to `run`/`write_report`, which it composes with.
