class PCMProcessor extends AudioWorkletProcessor {
    constructor() {
        super();
        this.audioQueue = [];
        this.bufferL = new Float32Array(0);
        this.bufferR = new Float32Array(0);

        this.maxQueueSize = 32;
        this.maxBufferSize = 48000 * 2;

        this.port.onmessage = (event) => {
            if (event.data && event.data.type === "reset") {
                console.debug("PCMProcessor: reset");
                this.audioQueue = [];
                this.bufferL = new Float32Array(0);
                this.bufferR = new Float32Array(0);

            }
            if (event.data && event.data.type === "audio") {
                let left = event.data.samples[0];
                let right = event.data.samples[1];

                if (this.audioQueue.length >= this.maxQueueSize) {
                    console.debug(`PCMProcessor: dropping buffer: ${this.maxQueueSize - this.audioQueue.length}`);
                    this.audioQueue.shift();
                }

                this.audioQueue.push({ left, right });
            }
        };
    }

    process(inputs, outputs) {
        const outputL = outputs[0][0];
        const outputR = outputs[0][1];

        // Ensure we have enough buffered audio before starting playback
        if (this.audioQueue.length < 4) {
            // console.debug(`PCMProcessor: filling buffer: ${16 - this.audioQueue.length} missing`);
            outputL.fill(0);
            outputR.fill(0);
            return true;
        }

        // Fill bufferL and bufferR if they are too small
        // if (this.bufferL.length < outputL.length) {
        //     if (this.audioQueue.length > 0) {
        //         const nextBuffer = this.audioQueue.shift();
        //         this.bufferL = new Float32Array([...this.bufferL, ...nextBuffer.left]);
        //         this.bufferR = new Float32Array([...this.bufferR, ...nextBuffer.right]);
        //
        //         // Prevent bufferL/bufferR from growing too large
        //         if (this.bufferL.length > this.maxBufferSize) {
        //             this.bufferL = this.bufferL.slice(-this.maxBufferSize);
        //             this.bufferR = this.bufferR.slice(-this.maxBufferSize);
        //         }
        //     } else {
        //         outputL.fill(0);
        //         outputR.fill(0);
        //         return true;
        //     }
        // }

        // Fill bufferL and bufferR until we have enough data to output
        while (this.bufferL.length < outputL.length && this.audioQueue.length > 0) {
            const nextBuffer = this.audioQueue.shift();
            this.bufferL = new Float32Array([...this.bufferL, ...nextBuffer.left]);
            this.bufferR = new Float32Array([...this.bufferR, ...nextBuffer.right]);
        }

        // If still not enough, output silence
        if (this.bufferL.length < outputL.length) {
            outputL.fill(0);
            outputR.fill(0);
            return true;
        }


        // Output available samples
        outputL.set(this.bufferL.subarray(0, outputL.length));
        outputR.set(this.bufferR.subarray(0, outputR.length));

        // Remove played samples from the buffer
        this.bufferL = this.bufferL.slice(outputL.length);
        this.bufferR = this.bufferR.slice(outputR.length);

        return true;
    }

    static get parameterDescriptors() {
        return [];
    }
}

registerProcessor("pcm-processor", PCMProcessor);
