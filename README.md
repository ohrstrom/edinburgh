# EDInburgh

Analyzes and plays DAB+ EDI streams.

```shell
RUST_BACKTRACE=1 cargo run -- edi-ch.digris.net:8855
```


## WASM

#### Setup

```shell
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```



## References

https://github.com/hradio/edihttp/blob/master/edihttp.go
https://github.com/hradio/edisplitter/blob/master/edisplitter.go


https://github.com/Opendigitalradio/ODR-DabMux-GUI/blob/master/src/config.rs

https://github.com/haileys/hailsplay/tree/main




## DEBUG

```
DEC: info: CStreamInfo {
    sampleRate: 0,
    frameSize: 0,
    numChannels: 0,
    pChannelType: 0x0000000110008280,
    pChannelIndices: 0x00000001100082a0,
    aacSampleRate: 24000,
    profile: 1,
    aot: 2,
    channelConfig: 2,
    bitRate: 0,
    aacSamplesPerFrame: 960,
    aacNumChannels: 0,
    extAot: 5,
    extSamplingRate: 48000,
    outputDelay: 0,
    flags: 32768,
    epConfig: -1,
    numLostAccessUnits: 0,
    numTotalBytes: 32472,
    numBadBytes: 0,
    numTotalAccessUnits: 0,
    numBadAccessUnits: 0,
    drcProgRefLev: -1,
    drcPresMode: -1,
}
```

```
DEC: info: CStreamInfo {
    sampleRate: 48000,
    frameSize: 1920,
    numChannels: 2,
    pChannelType: 0x13482ca80,
    pChannelIndices: 0x13482caa0,
    aacSampleRate: 24000,
    profile: 1,
    aot: 2,
    channelConfig: 2,
    bitRate: 65800,
    aacSamplesPerFrame: 960,
    aacNumChannels: 2,
    extAot: 5,
    extSamplingRate: 48000,
    outputDelay: 3602,
    flags: 32768,
    epConfig: -1,
    numLostAccessUnits: 0,
    numTotalBytes: 78902,
    numBadBytes: 0,
    numTotalAccessUnits: 242,
    numBadAccessUnits: 0,
    drcProgRefLev: -1,
    drcPresMode: -1,
}
```