# Zenoh Tailscale discovery

Zenoh generally does discovery on local networks using multicast UDP
But tailscale doesn't propagate multicast

This little app can be installed on each Tailscale node to serve as a relay for gossip discovery so that each zenoh client can discover peers across the tailnet

## Status

This project is very unfinished

## Deploy

```bash
TARGET_HOST=somepi TARGET_USERNAME=pi make deploy-docker
```
