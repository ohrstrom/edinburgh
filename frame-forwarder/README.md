# Frame Forwarder

The Frame Forwarder is a service that extracts EDI Application Layer
frames from the incoming TCP stream and makes them available via WebSocket.

Used by the [Web UI](../web-ui/) to receive EDI frames in the browser - as
direct TCP connections from within the browser are not possible.

## Usage

```shell
cargo run -- --help

Usage: edinburgh-frame-forwarder [OPTIONS]

Options:
      --host <HOST>  Server listening address [default: 127.0.0.1]
      --port <PORT>  Server listening port [default: 9000]
```

## WebSocket Connection

EDI connections are dynamically estaplished according to the request URI:

```shell
websocat ws://127.0.0.1:9000/ws/edi-ch.digris.net/8855 | hexdump -C
```
