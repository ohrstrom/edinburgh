# EDInburgh DAB Library

Shared library code to be used in either native Rust or WebAssembly (WASM) environments.

## Architecture

The implementation could be debated ;) - on a bird's eye view, it works like this:

- Acquired DAB frames are fed into the processing chain
- When something "interesting" happens an event is emitted

### Flow

Not the best graph ;) - but to get a rough idea of the data flow:

```mermaid
graph TB

  subgraph CALLER["Caller"]
    direction LR
    FE["Frame Extractor"]
    subgraph LIB["Shared DAB Library"]
      FP["Frame Parser"]
      ENS["Ensemble"]
      MSC["MSC"]
      subgraph PAD["PAD"]
        direction TB
        DL["DL (+)"]
        MOT["MOT"]
      end
      BUS["Event-Bus"]
    end

    EH["Event Handler"]
  end

  FE           --> FP
  FP  -- DETI  --> ENS
  FP  -- EST   --> MSC
  MSC -- X-PAD --> PAD

  ENS -- ENS   --> BUS
  MSC -- AU    --> BUS
  DL  -- DL    --> BUS
  MOT -- SLS   --> BUS

  BUS          --> EH

  %% --- Styles ---
  classDef caller fill:#22222233,stroke-width:2px;
  classDef lib fill:#22222299,stroke-width:2px;
  classDef pad fill:#22222299,stroke-width:1px;

  class CALLER caller;
  class LIB lib;
  class PAD pad;
```
