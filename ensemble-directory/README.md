# Ensemble Directory

Scans EDI host and port-ranges for DAB+ ensembles. Provides an HTTP API
to read the scanned ensembles and respective services as JSON.

## Usage

```shell
cargo run -- --help
```

```shell
cargo run -- \
  --scan edi-ch.digris.net:8851-8853 \
  --scan edi-fr.digris.net:8855-8858 \
  --scan-timeout 10 \
  --scan-interval 300 \
  --scan-parallel 16 \
  --verbose
```

## API

```shell
curl  http://127.0.0.1:9001/ensembles
```

```json
[
  {
    "host": "edi-ch.digris.net",
    "port": 8853,
    "eid": 17411,
    "al_flag": false,
    "label": "DIG D04 - WS",
    "short_label": "DIG D04 - WS",
    "services": [
      {
        "sid": 19919,
        "label": "105 DJ RADIO",
        "short_label": "DJ RADIO",
        "components": [
          {
            "scid": 16,
            "language": "German",
            "user_apps": [
              "SLS"
            ],
            "audio_format": {
              "sbr": true,
              "ps": false,
              "codec": "HE-AAC",
              "samplerate": 48,
              "bitrate": 72,
              "au_count": 3,
              "channels": 2
            }
          }
        ]
      },
      ...
```
