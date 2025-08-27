# EDInburgh WASM Module

WASM build of the shared [EDI library](../shared).

Used by the [Web UI](../web-ui/) to process EDI in the browser.

## Build UI

```shell
make build
```

## Usage

See [Web UI](../web-ui/src/App.vue) for an implementation example.

```javascript
import { EDI } from '<path-to>/pkg'

const edi = new EDI()

// feeding frames
const ws = new WebSocket('<frame-forwarder-url>')

ws.binaryType = 'arraybuffer'

ws.addEventListener('message', (e) => {
    edi.feed(new Uint8Array(e.data))
})

// event listeners
edi.addEventListener('ensemble_updated', async (e) => {
    console.debug('ensemble_updated', e.detail)
})

edi.addEventListener('mot_image', async (e) => {
    console.debug('mot_image', e.detail)
})

edi.addEventListener('dl_object', async (e) => {
    console.debug('dl_object', e.detail)
})

edi.addEventListener('aac_segment', async (e) => {
    console.debug('aac_segment', e.detail)
})
```
