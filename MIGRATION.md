# Migration Notes

## What changed

- Python runtime removed.
- Single Rust binary: `ddns-rs`.
- Cloudflare update flow is now explicit: `lookup -> create/update/no-op`.
- `once` and `run` modes are both supported.
- `check-config` and `print-ip` were added for safer rollout.

## Old config compatibility

The Rust loader keeps the original structure intentionally:

- `globals.interval` is preserved.
- `domains[].name` is preserved.
- `domains[].ip_type` is preserved.
- `domains[].ip_urls` is preserved.
- `domains[].provider` is preserved.
- `provider[].api_key` is still accepted and treated as the Cloudflare API token.

## Recommended rollout

1. Copy `sample_config.yml` to `config.yml`.
2. Fill in your real token and `zone_id`.
3. Run `ddns-rs -c config.yml check-config`.
4. Run `ddns-rs -c config.yml print-ip`.
5. Run `ddns-rs -c config.yml once`.
6. Enable `ddns-rs.service` only after the one-shot pass behaves correctly.
