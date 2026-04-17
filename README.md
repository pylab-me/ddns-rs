# ddns-rs

A small Rust DDNS client and daemon for Cloudflare.

This is a one-off Rust rewrite of the original Python project. The new version keeps the original YAML workflow, but
adds a production-clean binary shape:

- `once`: run one sync pass and exit
- `run`: daemon mode with interval loop
- `check-config`: validate YAML before deployment
- `print-ip`: show the resolved public IP used for each domain
- Cloudflare `create / update / no-op`
- `A` and `AAAA` support
- tighter config validation

## Build

```bash
cargo build --release
```

Binary:

```bash
./target/release/ddns-rs
```

## Commands

```bash
./target/release/ddns-rs -c sample_config.yml check-config
./target/release/ddns-rs -c sample_config.yml print-ip
./target/release/ddns-rs -c sample_config.yml once
./target/release/ddns-rs -c sample_config.yml run

# because --config is global, these also work:
./target/release/ddns-rs once -c sample_config.yml
./target/release/ddns-rs run -c sample_config.yml
```

## Configuration

The loader accepts the original project layout and also some stricter optional fields.

### Example

```yaml
globals:
  interval: 300
  timeout_secs: 10
  user_agent: "ddns-rs/0.1"
  log_level: "info"

domains:
  - name: abc1.example.com
    ip_type: 4
    ip_urls:
      - "https://api.ipify.org"
      - "https://ifconfig.me/ip"
    provider:
      - name: cloudflare
        api_key: your_cloudflare_api_token
        zone_id: your_zone_id
        proxied: true
        ttl: 1

  - name: abc2.example.com
    record_type: AAAA
    ip_urls:
      - "https://api64.ipify.org"
    provider:
      - name: cloudflare
        api_key: your_cloudflare_api_token
        zone_id: your_zone_id
        proxied: false
        ttl: 120
```

## Notes

- `api_key` from the old config is treated as a Cloudflare API token.
- `provider` may be either one object or a list.
- `ttl: 1` means Cloudflare auto TTL.
- If the record does not exist, the tool will create it.
- If the record exists and already matches the resolved IP, the tool performs no update.

## systemd

See `ddns-rs.service`.
