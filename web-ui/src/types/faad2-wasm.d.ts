declare module '@ohrstrom/faad2-wasm/faad2_decoder.js' {
  interface FAAD2DecoderOptions {
    output: (audioData: AudioData) => void;
    error: (err: DOMException) => void;
  }

  interface ConfigureOptions {
    codec: string;
    description: ArrayBuffer | Uint8Array;
  }

  interface DecodeChunk {
    byteLength: number;
    timestamp: number;
    copyTo(target: Uint8Array): void;
  }

  export default class FAAD2Decoder {
    constructor(options: FAAD2DecoderOptions);
    configure(options: ConfigureOptions): Promise<void>;
    reset(): Promise<void>;
    decode(chunk: DecodeChunk): Promise<void>;
  }
}