# ADR 0002: Private network first; Tailscale as optional transport

- Status: Proposed
- Date: 2026-09-10

## Context

The daemon primarily runs on a trusted local network. Android/mobile access is desired remotely.

## Proposed decision

Default to loopback. Permit explicit LAN binding. Plan for access over Tailscale without embedding Tailscale-specific assumptions into the GTD domain or CRDT layers.

Do not implement Noise or WireGuard inside samgtd.

## Rationale

Tailscale already provides authenticated encrypted networking between peers. The application should define its own API authentication/authorization policy independently.

## Future options

- direct tailnet IP listener;
- Tailscale Serve;
- tailnet DNS/MagicDNS;
- Android client connected to the same tailnet.
