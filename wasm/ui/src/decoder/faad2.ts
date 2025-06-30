import Faad2Module from '@/lib/faad2.js'

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


    console.debug('FAAD2Decoder:configure', codec, asc)

    try {
      if (!this.module) {
        this.module = await Faad2Module()
        console.debug('FAAD2: module loaded')
        console.debug('FAAD2:capabilities', this.module._get_faad_capabilities())
      }

      const ascPtr = this.module._malloc(asc.length)
      this.module.HEAPU8.set(asc, ascPtr)

      const result = this.module._init_decoder(ascPtr, asc.length)
      this.module._free(ascPtr)

      if (result < 0) {
        throw new Error('Failed to initialize FAAD2 decoder')
      }

      this.initialized = true
      console.debug('FAAD2 decoder initialized with ASC:', asc)
    } catch (err) {
      this.error(new DOMException((err as Error).message, 'InvalidStateError'))
    }
  }

  async reset(asc) {

  }

  async decode(chunk: EncodedAudioChunk): Promise<void> {
    try {
      if (!this.module || !this.initialized) {
        throw new Error('Decoder not initialized');
      }


  
      const input = new Uint8Array(chunk.byteLength);
      chunk.copyTo(input);

    //   console.debug('FAAD2Decoder:chunk', input.length, chunk)
  
      const inPtr = this.module._malloc(input.length);
      const outPtr = this.module._malloc(4096 * 4 * 2); // adjust size if needed
  
      this.module.HEAPU8.set(input, inPtr);
  
      const samples = this.module._decode_frame(inPtr, input.length, outPtr, 4096 * 4 * 2);

    //   console.debug('FAAD2Decoder:samples', samples.length, samples)
  
      this.module._free(inPtr);
  
      if (samples <= 0) {
        this.module._free(outPtr);
        return;
      }

      const numChannels = 2;
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
        sampleRate: 48000,
        numberOfFrames: numFrames,
        numberOfChannels: 2,
        timestamp: chunk.timestamp,
        data: buffer,
        transfer: [buffer], // (optional) if you're sending across threads
      });

    //   console.debug('FAAD2Decoder:decoded', audioData)
      
      this.output(audioData);
  
    } catch (err) {
      this.error(new DOMException((err as Error).message, 'EncodingError'));
    }
  }
}

export default FAAD2Decoder
