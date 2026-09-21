# 001 - Maro connects his X account

Maro opens Slix for the first time and connects his X account, so that Slix can start showing him what's happening on his account.

## Acceptance Criteria

1. After connecting, Maro sees his X handle and a sign that Slix is connected to that account.
2. When Maro opens Slix again later, he is still connected and doesn't have to connect again.

## Technical Design

**Stack:** Rust. Dev environment via Nix flake (A/B/C) — one `flake.nix` providing the Rust toolchain.

**Scope constraint:** Slix is read-only against X. It never writes/posts. The API client exposes only GET operations; this is structural, not a comment.

**Auth:** OAuth 1.0a PIN flow (no auto-opened browser; the authorize URL is printed and Maro pastes back the PIN). Signing delegated to an OAuth crate.

### Components

1. **`config`** — `Config { consumer_key, consumer_secret, access_token: Option, access_token_secret: Option }`
   - Knows the file path, XDG-aware → `~/.config/slix/config.json`.
   - `load()`: missing file → unconnected, never errors. `save()`: enforces 0600 perms.
   - Connected iff the token fields are `Some`.

2. **`oauth`** — the PIN flow on top of the OAuth crate
   - `authorize_url()`: the URL we print to the terminal.
   - `exchange(pin)`: trades the PIN for the access token pair.
   - Depends on the OAuth crate + `reqwest`.

3. **`api`** — `XApiClient`, read-only
   - Base `https://api.x.com/2/`; signs each request with the stored consumer + access tokens.
   - Today: `users/me` → the handle (`@username`).
   - Errors map to *connected* vs *needs-reconnect*.

4. **`main`** — the single implicit entrypoint (no subcommands)
   - `load config` →
     - token present → `api.users/me` succeeds → print `@handle · connected`
     - token present → verification fails → print "reconnecting…", run PIN flow silently, overwrite token
     - token absent → run PIN flow, save, verify → print `@handle · connected`

```
            main
        ┌─────┼─────┐
      config oauth  api
        │            │
      fs   oauth-crate + reqwest
```

**Credentials storage:** plaintext in the single config file, 0600. Consumer keys and the personal access token/secret live together in `config.json`.

### Implementation order (TDD)

1. `config`: load/save round-trip; 0600 on write; missing file → unconnected.
2. `oauth`: authorize URL shape; PIN exchange against a stubbed/sandbox HTTP layer.
3. `api`: signed `users/me` request; error → needs-reconnect mapping.
4. `main`: first-run flow, verify-on-startup flow, silent-reconnect flow.
