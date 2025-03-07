<script>
  let ws;
  let audioContext;
  let decoder;
  let frameBuffer = [];

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
                  // console.debug("audioData:", audioData);
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

      window.d = decoder;
  };

  const processAACFrame = (aacFrame) => {
      if (!decoder) initializeAudioDecoder();

      const chunk = new EncodedAudioChunk({
          type: "key",
          timestamp: (audioContext.currentTime * 1e6), // Ensure correct timestamps
          duration: 2048 * (1000000 / 48000),
          data: aacFrame.buffer,
      });

      decoder.decode(chunk);
  };

  const playDecodedAudio = async (audioData) => {
    if (!audioContext) return;

    console.debug("AD:", audioData);

    const numChannels = 2;
    const sampleRate = 24000;
    // const numFrames = audioData.numberOfFrames;
    const numFrames = audioData.numberOfFrames;

    // // Create an AudioBuffer to hold the decoded PCM samples
    const audioBuffer = audioContext.createBuffer(numChannels, numFrames, sampleRate);

    for (let channel = 0; channel < numChannels; channel++) {
        const channelData = new Float32Array(numFrames);
        audioData.copyTo(channelData, { planeIndex: channel });
        audioBuffer.copyToChannel(channelData, channel);
    }

    // Create a buffer source to play the decoded audio
    const source = audioContext.createBufferSource();
    source.buffer = audioBuffer;
    source.connect(audioContext.destination);
    source.start();

    console.debug(`Playing ${numFrames} frames at ${sampleRate}Hz`);
};

</script>
  
<main>
    <div>
        <h1>AAC Stream</h1>
        <button on:click={connect}>Connect</button>
    </div>
    <audio id="audioPlayer" controls></audio>
</main>
  