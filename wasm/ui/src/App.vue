<script setup lang="ts">
import { ref, computed, reactive, onMounted } from 'vue'

import { EDI } from '../../pkg'

import Ensemble from './components/Ensemble.vue'
import Service from './components/Service.vue'

let edi = new EDI()
let ws: WebSocket | null = null
// let frameBuffer = []

let audioContext: AudioContext
let workletNode: AudioWorkletNode
let decoder: AudioDecoder

const ediHost = ref('edi-ch.digris.net')
const ediPort = ref(8855)

const imgSrc = ref('')

const numFramesReceived = ref(0)
const selectedServiceSid = ref(0)
const selectedScid = ref(0)

const scids = ref([])

let ensemble = reactive<{
  eid: number
  label?: string
  short_label?: string
  services: {
    sid: number
    scid?: number
    label?: string
    short_label?: string
  }[]
}>({
  eid: 0,
  services: [],
})

const services = computed(() => {
  const s = ensemble?.services ?? []
  return s
    .filter((s) => s.label !== undefined)
    .map((service) => ({
      ...service,
      isCurrent: service.sid === selectedServiceSid.value,
    }))
    .sort((a, b) => a.label!.localeCompare(b.label!))
})

const service = computed(() => {
  return services.value.find((s) => s.sid === selectedServiceSid.value)
})

const connect = async () => {
  await initializeAudioDecoder()

  if (!ws) {

    const uri = `ws://localhost:9000/ws/${ediHost.value}/${ediPort.value}/`
    // const uri = `ws://78.47.36.61:80/ws/${ediHost.value}/${ediPort.value}/`

    ws = new WebSocket(uri)
    // ws = new WebSocket("ws://localhost:9000/ws/edi-ch.digris.net/8855/")

    ws.binaryType = "arraybuffer"

    // ws.onmessage = async (event) => {
    //   await edi.feed(new Uint8Array(event.data))
    // };

    ws.onmessage = (event) => {
      edi.feed(new Uint8Array(event.data))
    };

    ws.onclose = () => {
      console.info("WebSocket closed")
      ws = null
    };

    ws.onerror = (e) => {
      console.error("WebSocket error:", e)
    };

  }

  edi.on_ensemble_update(async (ensembleData) => {
    console.log('ENSEMBLE:', ensembleData)
    Object.assign(ensemble, ensembleData)
  })

  edi.on_aac_segment(async (aacSegment) => {
    // console.log('AAC SEGMENT:', aacSegment)
    if (!scids.value.includes(aacSegment.scid)) {
      scids.value = [...scids.value, aacSegment.scid]
    }

    if (aacSegment.scid !== selectedScid.value) {
      return
    }
    // console.log('AAC SEGMENT:', aacSegment)

    aacSegment.frames.forEach((frame) => {
      // console.log('AAC FRAME:', frame)
      processAACSegment(new Uint8Array(frame))
    });

    // await processAACSegment(new Uint8Array(aacSegment.data))
  })

  edi.on_mot_image_received(async (motImage) => {
    console.log('MOT IMAGE:', motImage)
    imgSrc.value = `data:${motImage.mimetype};base64,${motImage.data}`
  })

}



/*
edi.on_edi_frame(async (frameData) => {
  if (numFramesReceived.value < 1) {
    console.log('EDI FRAME:', frameData)
  }
  numFramesReceived.value++
})

edi.on_ensemble_update(async (ensembleData) => {
  console.log('ENSEMBLE:', ensembleData)
  Object.assign(ensemble, ensembleData)
})

edi.on_aac_segment(async (aacSegment) => {
  if (!scids.value.includes(aacSegment.scid)) {
    scids.value = [...scids.value, aacSegment.scid]
  }

  if (aacSegment.scid !== selectedScid.value) {
    return
  }

  await processAACSegment(new Uint8Array(aacSegment.data))
})
*/


// onMounted(() => {
//   // TODO: fix audio context initialization
//   connect()
// })

const disconnect = async () => {

  selectedServiceSid.value = 0
  selectedScid.value = 0


  if (ws) {
    ws.onopen = ws.onmessage = ws.onerror = ws.onclose = null;

    try {
      ws.close(1000, "Client closed connection");

      // Wait for it to fully close
      await new Promise((resolve) => {
        ws.onclose = () => {
          resolve();
        };
      });

    } catch (e) {
      console.warn("WebSocket close threw:", e);
    }

    ws = null;
  }

  edi.reset()

  if (decoder) {
    decoder.reset()
  }
  // reset everything
  Object.assign(ensemble, {
    eid: 0,
    services: [],
  })
}

const initializeAudioDecoder = async () => {
  if (decoder) {
    console.info('decoder already initialized')
    return
  }

  audioContext = new AudioContext({
    latencyHint: 'balanced',
    sampleRate: 24000,
  })

  await audioContext.audioWorklet.addModule('pcm-processor.js')

  workletNode = new AudioWorkletNode(audioContext, 'pcm-processor', {
    outputChannelCount: [2],
  })

  workletNode.connect(audioContext.destination)

  decoder = new AudioDecoder({
    output: (audioData) => {
      playDecodedAudio(audioData)
    },
    error: (e) => console.error('Decoder error:', e),
  })

  const asc = new Uint8Array([0x13, 0x14, 0x56, 0xe5, 0x98])

  decoder.configure({
    codec: 'mp4a.40.5',
    sampleRate: 48000,
    numberOfChannels: 2,
    description: asc.buffer,
  })
}

const processAACSegment = async (aacSegment) => {
  const chunk = new EncodedAudioChunk({
    type: 'key',
    timestamp: audioContext.currentTime * 1e6,
    data: aacSegment.buffer,
  })

  decoder.decode(chunk)
}

const playDecodedAudio = async (audioData) => {
  const numChannels = 2
  const numFrames = audioData.numberOfFrames

  const pcmData = [new Float32Array(numFrames), new Float32Array(numFrames)]

  for (let channel = 0; channel < numChannels; channel++) {
    audioData.copyTo(pcmData[channel], { planeIndex: channel })
  }

  workletNode.port.postMessage({
    type: 'audio',
    samples: pcmData,
  })
}

const setScid = async (scid) => {
  selectedScid.value = scid
  console.log('SCID:', scid)

  decoder.reset()

  const asc = new Uint8Array([0x13, 0x14, 0x56, 0xe5, 0x98])

  decoder.configure({
    codec: 'mp4a.40.5',
    sampleRate: 48000,
    numberOfChannels: 2,
    description: asc.buffer,
  })

  workletNode.port.postMessage({
    type: 'reset',
  })
}

const selectService = async (sid: number) => {
  selectedServiceSid.value = sid
  console.log('SID:', sid)

  await setScid(service.value?.scid ?? 0);
}
</script>

<template>
  <main>
    <div>
      <div>
        <label for="ediPort">EDI Port:</label>
        <input id="ediPort" type="number" v-model="ediPort" />
      </div>
      <div>
        <button @click="connect">Connect</button>
        <button @click="disconnect">Disconnect</button>
      </div>

      <div>
        <Ensemble :ensemble="ensemble" />
      </div>

      <div>
        <pre v-text="service" />
      </div>

      <div v-if="imgSrc">
        <img :src="imgSrc" alt="MOT Image" />
      </div>

      <div>
        <div>
          <h3>Services</h3>
        </div>
        <Service v-for="service in services" :key="service.scid" :service="service"
          :is-current="service.scid === selectedScid" @select="selectService(service.sid)"
          @play="setScid(service.scid === selectedScid ? 0 : service.scid)" />
      </div>
    </div>
  </main>
</template>

<style scoped>
button {
  margin: 5px;
  padding: 8px 12px;
  cursor: pointer;
}
</style>
