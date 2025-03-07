<script>
const CHANNELS = 2;
const SAMPLE_RATE = 48000;

let audioContext;
let audioNode;

let analyser;
let frequencyData;

let ws;

async function setupAudio() {
    audioContext = new AudioContext({ sampleRate: SAMPLE_RATE });

    await audioContext.audioWorklet.addModule("pcm-processor.js");

    audioNode = new AudioWorkletNode(audioContext, "pcm-processor", {
        outputChannelCount: [CHANNELS],
    });

    analyser = audioContext.createAnalyser();
    analyser.fftSize = 2048;
    frequencyData = new Uint8Array(analyser.frequencyBinCount);

    audioNode.connect(analyser);
    audioNode.connect(audioContext.destination);
}

const connect = async () => {

  if (!audioContext || audioContext.state === "suspended") {
      await setupAudio();
      audioContext.resume();
  }

  console.debug("Audio context:", audioContext.state);

  if (!ws) {
      ws = new WebSocket("ws://localhost:9001");
      ws.binaryType = "arraybuffer";
      ws.onmessage = (event) => {
          const pcmData = new Float32Array(event.data);
          console.log("Received PCM data:", pcmData.length);

          if (audioNode) {
            audioNode.port.postMessage(pcmData);
          }
      };
  }

  console.debug("ws:", ws);
}
</script>

<main>
  <div>
    <h1>PCM</h1>
    <button on:click={connect}>Connect</button>
  </div>
  <!--
  <pre>{JSON.stringify(ws)}</pre>
  -->
</main>

<style>

</style>
