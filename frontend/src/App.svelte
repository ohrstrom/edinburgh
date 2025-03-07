<script>
  let ws;
  let audioContext;
  let decoder;
  let frameBuffer = [];
  let nextPlayTime = 0;

  const connect = async () => {

      if (!ws) {
          ws = new WebSocket("ws://localhost:9001");

          ws.binaryType = "arraybuffer";
  
          ws.onmessage = (event) => {
            processAACFrame(new Uint8Array(event.data));
          };
      }
      console.debug("ws:", ws);
  };

  const initializeAudioDecoder = () => {
      if (!audioContext) {
          audioContext = new AudioContext();
      }

      if (!decoder) {
          decoder = new AudioDecoder({
              output: (audioData) => {
                  playDecodedAudio(audioData);
              },
              error: (e) => console.error("Decoder error:", e),
          });

          const asc = new Uint8Array([0x13, 0x14, 0x56, 0xE5, 0x98]);

          decoder.configure({
              codec: "mp4a.40.5",
              sampleRate: 48000,
              numberOfChannels: 2,
              description: asc.buffer,
          });
      }

  };

  const processAACFrame = (aacFrame) => {
      if (!decoder) initializeAudioDecoder();

      frameBuffer.push(aacFrame);

      // Process only when 3 frames are received
      if (frameBuffer.length === 3) {
        for (let i = 0; i < frameBuffer.length; i++) {
            const chunk = new EncodedAudioChunk({
                type: i === 0 ? "key" : "delta",
                timestamp: (audioContext.currentTime * 1e6) + i * 8000,
                duration: 1024 * (1000000 / 48000),
                data: frameBuffer[i].buffer,
            });

            decoder.decode(chunk);
        }

        frameBuffer = [];
      }

      // const chunk = new EncodedAudioChunk({
      //     type: "key",
      //     timestamp: (audioContext.currentTime * 1e6), // Ensure correct timestamps
      //     duration: 2048 * (1000000 / 48000),
      //     data: aacFrame.buffer,
      // });

      // decoder.decode(chunk);
  };

  const playDecodedAudio = async (audioData) => {
    if (!audioContext) return;

    const numChannels = 2;
    const sampleRate = 24000;
    const numFrames = audioData.numberOfFrames;

    const audioBuffer = audioContext.createBuffer(numChannels, numFrames, sampleRate);

    for (let channel = 0; channel < numChannels; channel++) {
        const channelData = new Float32Array(numFrames);
        audioData.copyTo(channelData, { planeIndex: channel });
        audioBuffer.copyToChannel(channelData, channel);
    }

    const source = audioContext.createBufferSource();
    source.buffer = audioBuffer;
    source.connect(audioContext.destination);

    if (nextPlayTime < audioContext.currentTime) {
        nextPlayTime = audioContext.currentTime + 0.5;
    }

    source.start(nextPlayTime);
    nextPlayTime += audioBuffer.duration;
};

</script>
  
<main>
    <div>
        <h1>HE-AAC</h1>
        <button on:click={connect}>Connect</button>
    </div>
</main>
  