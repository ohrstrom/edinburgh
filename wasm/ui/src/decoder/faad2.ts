import Faad2Module from '@/lib/faad2.js'


const SAMPLE_RATE = {
  1: 8000,
  2: 16000,
  3: 22050,
  4: 32000,
  5: 44100,
  6: 48000,
  7: 64000,
  8: 88200,
  9: 96000,
};

class FAAD2Decoder {

  private module: any = null
  private initialized = false
  private output: (audioData: AudioData) => void
  private error: (error: DOMException) => void

  private inputBuffer = new Uint8Array(0);
  
  constructor({
    output,
    error
  }: {
    output: (audioData: AudioData) => void
    error: (error: DOMException) => void
  }) {
    this.output = output
    this.error = error
  }

  async configure(
    {
      codec,
      description
    }: {
      codec: string; 
      description: Uint8Array
    }
  ): Promise<void> {

    const asc = new Uint8Array(description);


    // console.debug('FAAD2Decoder:configure', codec, asc)

    try {
      if (!this.module) {
        this.module = await Faad2Module()
        // console.debug('FAAD2: module loaded')
        // console.debug('FAAD2: capabilities', this.module._get_faad_capabilities())
      }

      const ascPtr = this.module._malloc(asc.length)
      this.module.HEAPU8.set(asc, ascPtr)

      const result = this.module._init_decoder(ascPtr, asc.length)
      this.module._free(ascPtr)

      if (result < 0) {
        throw new Error('Failed to initialize FAAD2 decoder')
      }

      this.initialized = true

      // console.debug('FAAD2Decoder:configured', codec, asc)

      console.debug(
        'FAAD2Decoder: configured',
        codec,
        Array.from(asc).map(b => `0x${b.toString(16).padStart(2, '0').toUpperCase()}`).join(', ')
      );

    } catch (err) {
      this.error(new DOMException((err as Error).message, 'InvalidStateError'))
    }
  }

  async reset() {
    console.debug('FAAD2Decoder: reset')
  }

  async decode(chunk: EncodedAudioChunk): Promise<void> {
    // try {
      if (!this.module || !this.initialized) {
        throw new Error('Decoder not initialized');
      }
  
      const input = new Uint8Array(chunk.byteLength);
      chunk.copyTo(input);

      const inputLength = input.length;
      const pad = 64; // Increased padding to prevent read-overruns in the decoder

      const inPtr = this.module._malloc(inputLength + pad);
      this.module.HEAPU8.set(input, inPtr);
      this.module.HEAPU8.fill(0, inPtr + inputLength, inPtr + inputLength + pad);

      const maxFrames = 2048 * 2; // SBR worst-case (2048 samples), with a 2x safety margin
      const maxChannels = 2;      // PS expansion mono → stereo
      // The total buffer size must account for stereo output, hence maxFrames * maxChannels.
      const maxSamples = maxFrames * maxChannels;
      const outputSize = maxSamples * Float32Array.BYTES_PER_ELEMENT;

      const outPtr = this.module._malloc(outputSize);

      const packed = this.module._decode_frame(inPtr, input.length, outPtr, outputSize);
      this.module._free(inPtr);
      if (packed <= 0) {
        this.module._free(outPtr);
        return;
      }

      const samplerateIndex = (packed >>> 28) & 0xF;
      const numChannels     = (packed >>> 24) & 0xF;
      const samples         = packed & 0xFFFFFF;

      const samplerate = SAMPLE_RATE[samplerateIndex] || 0;

      // console.debug(`FAAD2: channels=${numChannels} samplerate=${samplerate} samples=${samples}`);

      // const samples = packed & 0xFFFFFF;
      // const numChannels = (packed >>> 24) & 0xFF;

      // 
      const numFrames = samples / numChannels;
      const planeSize = numFrames * Float32Array.BYTES_PER_ELEMENT;

      const raw = new Float32Array(this.module.HEAPU8.buffer, outPtr, samples);
      
      const buffer = new ArrayBuffer(planeSize * numChannels);
      const left = new Float32Array(buffer, 0, numFrames);
      const right = new Float32Array(buffer, planeSize, numFrames);
      
      // Deinterleave from interleaved `raw`
      for (let i = 0; i < numFrames; i++) {
        left[i] = raw[i * 2];
        right[i] = raw[i * 2 + 1];
      }
      
      this.module._free(outPtr);
      
      // Create AudioData using planar buffer
      const audioData = new AudioData({
        format: 'f32-planar',
        // sampleRate: 48_000,
        sampleRate: samplerate,
        numberOfFrames: numFrames,
        numberOfChannels: 2,
        timestamp: chunk.timestamp,
        data: buffer,
        transfer: [buffer], // (optional) if you're sending across threads
      });
    
      this.output(audioData);
  
    // } catch (err) {
    //   this.error(new DOMException((err as Error).message, 'DecodingError'));
    // }
  }
}

export default FAAD2Decoder
