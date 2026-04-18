# Protocol

The initial transport is a local Unix socket spoken by `resurreccion-cli`,
`resurreccion-zellij-plugin`, and any future standalone UIs.

This is intentionally separate from in-process eventing:

- socket protocol: process boundary
- `rt-events`: observer pattern and event fanout within a running process

Every response should fit a stable envelope:

```json
{"ok":true,"data":{}}
{"ok":false,"error":{"code":"unimplemented","message":"..." }}
```

The detailed request schema remains to be designed after the store and daemon
bootstrapping work begins.
