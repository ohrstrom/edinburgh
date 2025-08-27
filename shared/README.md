# EDInburgh DAB Library

Shared library code to be used in either native Rust or WebAssembly (WASM) environments.

## Architecture

The implementation could be debated ;) - on a bird's eye view, it works like this:

- Acquired DAB frames are fed into the processing chain
- When something "interesting" happens an event is emitted

### Flow

NOTE: This  is not yet com / correcy !!

```mermaid_wip
graph TD
  direction TB
  subgraph implementor["Library Consumer"]
    direction TB
    FG["Frame Generator"]
    EH["Event Handler"]
    subgraph AUDIODEC["Audio Decoder"]
      AUDATA["AU"]
    end
  end
  subgraph library["Shared DAB Library"]
    direction TB
    FE["Frame Extractor"]
    FIG["FIG"]
    subgraph MSC["MSC"]
      direction TB
      AU["AU"]
      XPAD["X-PAD"]
    end
    subgraph PAD["X-PAD"]
      direction TB
      DL["DL"]
      MOT["MOT"]
    end
    subgraph BUS["BUS"]
      direction TB
      EVENT["Event"]
    end
  end
  FG --> FE
  FE --> MSC
  AU --> BUS
  XPAD --> PAD
  DL --> BUS
  FIG --> BUS
  MOT --> BUS
  EVENT --> EH
  EH --> AUDIODEC
```
