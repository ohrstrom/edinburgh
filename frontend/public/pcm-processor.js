class PCMProcessor extends AudioWorkletProcessor {
    constructor() {
        super();
        this.audioQueue = [];
        this.bufferL = new Float32Array(0);
        this.bufferR = new Float32Array(0);

        this.port.onmessage = (event) => {
            const buffer = new Float32Array(event.data);
            const deinterleaved = this.deinterleave(buffer);
            this.audioQueue.push(deinterleaved); // Store deinterleaved PCM
        };
    }

    deinterleave(buffer) {
        const left = new Float32Array(buffer.length / 2);
        const right = new Float32Array(buffer.length / 2);

        for (let i = 0, j = 0; i < buffer.length; i += 2, j++) {
            left[j] = buffer[i];
            right[j] = buffer[i + 1];
        }

        return { left, right };
    }

    process(inputs, outputs) {
        const outputL = outputs[0][0];
        const outputR = outputs[0][1];

        if (this.bufferL.length < outputL.length) {
            if (this.audioQueue.length > 0) {
                const nextBuffer = this.audioQueue.shift();
                this.bufferL = new Float32Array([...this.bufferL, ...nextBuffer.left]);
                this.bufferR = new Float32Array([...this.bufferR, ...nextBuffer.right]);
            } else {
                // No data available, output silence
                outputL.fill(0);
                outputR.fill(0);
                return true;
            }
        }

        // Copy the correct amount of samples
        outputL.set(this.bufferL.subarray(0, outputL.length));
        outputR.set(this.bufferR.subarray(0, outputR.length));

        // Remove the samples we just played
        this.bufferL = this.bufferL.slice(outputL.length);
        this.bufferR = this.bufferR.slice(outputR.length);

        return true;
    }

    static get parameterDescriptors() {
        return [];
    }
}

registerProcessor("pcm-processor", PCMProcessor);
