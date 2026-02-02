# TODO

## Security audit follow-ups (medium+)

- [ ] Issue: Spoofable client IP allows rate-limit bypass (trusts `X-Forwarded-For`).
      Proposed fix: Only trust `X-Forwarded-For` when the immediate peer IP is in
      a configured allowlist (`TRUSTED_PROXIES`). Otherwise use `ConnectInfo`
      only. Ensure the reverse proxy strips inbound `X-Forwarded-For` and sets
      it itself. Update config and the IP extractor in `src/auth.rs`.

- [ ] Issue: Unbounded in-memory rate limiter can grow without bound (DoS risk).
      Proposed fix: Replace `RateLimiter` with a bounded, evicting limiter (e.g.
      `governor`/`tower_governor` or LRU+TTL). For multi-instance/prod, use Redis
      or another shared backend. Update `src/ratelimit.rs` and call sites.

- [ ] Issue: Internal DB/S3 error strings are returned to clients.
      Proposed fix: Return generic errors to clients, log detailed errors only.
      Optionally include a request ID for correlation. Update `src/error.rs`.

- [ ] Issue: Missing security headers (CSP, Referrer-Policy, etc.).
      Proposed fix: Add response header layer:
      - `X-Content-Type-Options: nosniff`
      - `Referrer-Policy: no-referrer`
      - `X-Frame-Options: DENY` or CSP `frame-ancestors 'none'`
      - CSP tailored to `/h` (`default-src 'self'; img-src 'self' data: https:;`
        `style-src 'self'; script-src 'self'; base-uri 'none'`)
      Implement via `tower_http::set_header::SetResponseHeaderLayer` in `src/lib.rs`.

- [ ] Issue: No per-account quotas or retention controls.
      Proposed fix: Enforce max total bytes and max request count per account in
      `create_request`/`update_request`, add optional TTL (`expires_at`) on
      requests/revisions, and run a cleanup job to delete expired rows + S3 keys.
      Add config in `src/config.rs`.
