<script>
  let ws;
  let audioContext;
  let workletNode;
  let decoder;

  let isDecoding = $state(false);

  const connect = async () => {
    await initializeAudioDecoder();

    if (!ws) {
      ws = new WebSocket("ws://localhost:9001");

      ws.binaryType = "arraybuffer";

      ws.onmessage = async (event) => {
        await processAACFrame(new Uint8Array(event.data));
      };

      ws.onclose = () => {
        console.info("WebSocket closed");
        ws = null;
      };

      ws.onerror = (e) => {
        console.error("WebSocket error:", e);
      };

    }
    console.debug("ws:", ws);
  };

  const disconnect = async () => {
    if (ws) {
      ws.close();
      ws = null;
    }
  };

  const initializeAudioDecoder = async () => {
    if (decoder) {
      console.info("decoder already initialized");
      return;
    }

    audioContext = new AudioContext({
      latencyHint: "balanced",
      sampleRate: 24000,
    });
    await audioContext.audioWorklet.addModule("pcm-processor.js");

    workletNode = new AudioWorkletNode(audioContext, "pcm-processor", {
      outputChannelCount: [2],
    });
    workletNode.connect(audioContext.destination);

    decoder = new AudioDecoder({
      output: (audioData) => {
        playDecodedAudio(audioData);
      },
      error: (e) => console.error("Decoder error:", e),
    });

    const asc = new Uint8Array([0x13, 0x14, 0x56, 0xe5, 0x98]);

    decoder.configure({
      codec: "mp4a.40.5",
      sampleRate: 48000,
      numberOfChannels: 2,
      description: asc.buffer,
    });

    decoder.ondequeue = (e) => {
      // console.debug("decoder.ondequeue", e);
    };

  };

  const processAACFrame = async (aacFrame) => {
    const chunk = new EncodedAudioChunk({
      type: "key",
      timestamp: audioContext.currentTime * 1e6, // timestamp is needed but has no effect
      data: aacFrame.buffer,
    });

    decoder.decode(chunk);

  };

  const playDecodedAudio = async (audioData) => {
    const numChannels = 2;
    const numFrames = audioData.numberOfFrames;

    let pcmData = [new Float32Array(numFrames), new Float32Array(numFrames)];

    for (let channel = 0; channel < numChannels; channel++) {
      audioData.copyTo(pcmData[channel], { planeIndex: channel });
    }

    workletNode.port.postMessage({
      type: "audio",
      samples: pcmData,
    });

    isDecoding = true;

  };
</script>

<main>
  <div>
    <h1>HE-AAC</h1>
    <button onclick={connect}>Connect</button>
    <button onclick={disconnect}>Disconnect</button>
    <div>
      <p>Decoding: {isDecoding ? "Yes" : "No"}</p>
    </div>
  </div>
</main>
