<script setup lang="ts">
import { ref, computed, reactive, onMounted } from 'vue'
import {storeToRefs} from "pinia"

import { EDI } from '../../pkg'

import {useEDIStore} from "@/stores/edi";

// store mappings
const ediStore = useEDIStore()
const { selectService, updateEnsemble, updateDL, updateSLS } = ediStore



import Connection from '@/components/edi/connection/Connection.vue'
import Ensemble from '@/components/edi/ensemble/Ensemble.vue'
import ServiceDetail from '@/components/edi/service-detail/Service.vue'
import ServiceList from '@/components/edi/service-list/Services.vue'

let edi = new EDI()
let ws: WebSocket | null = null
// let frameBuffer = []

let audioContext: AudioContext
let workletNode: AudioWorkletNode
let decoder: AudioDecoder

const selectedServiceSid = ref(0)
const selectedScid = ref(10)

// const {selectedScid} = storeToRefs(ediStore)


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

const connect = async (e) => {

  console.debug("connect", e)

  await initializeAudioDecoder()

  if (!ws) {

    const uri = `ws://localhost:9000/ws/${e.host}/${e.port}/`

    console.debug(uri)

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
    // console.log('ENSEMBLE:', ensembleData)
    // Object.assign(ensemble, ensembleData)
    await updateEnsemble(ensembleData)
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
    // console.log('MOT IMAGE:', motImage)
    // imgSrc.value = `data:${motImage.mimetype};base64,${motImage.data}`
    await updateSLS(motImage)
  })

  edi.on_dl_object_received(async (dlObj) => {
    // console.log('DL OBJ:', dlObj)
    await updateDL(dlObj)
  })

}

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

// const selectService = async (sid: number) => {
//   selectedServiceSid.value = sid
//   console.log('SID:', sid)
//
//   await setScid(service.value?.scid ?? 0);
// }

</script>

<template>
  <main>

    <h1>{{selectedScid}}</h1>

    <header>
      <Ensemble />
      <Connection @connect="connect" @reset="disconnect" />
    </header>

    <section class="service-detail">
      <ServiceDetail />
    </section>

    <section class="service-list">
      <ServiceList
          @select="selectService"
      />
    </section>

  </main>
</template>

<style scoped>

main {
  width: 100%;
  min-height: 100vh;
  max-width: 1024px;
  margin-inline: auto;

  > header {
    display: flex;
    justify-content: space-between;
    background: white;

    margin-top: 16px;
    margin-bottom: 16px;
    padding: 8px;
    border: 1px solid #000;
    box-shadow: 4px 4px #000;

  }

  .service-detail {
    margin-bottom: 16px;
    padding: 8px;
    border: 1px solid #000;
    box-shadow: 4px 4px #000;
  }

  .service-list {
    margin-bottom: 16px;
    //padding: 8px;
    border: 1px solid #000;
    box-shadow: 4px 4px #000;
  }

}


button {
  margin: 5px;
  padding: 8px 12px;
  cursor: pointer;
}
</style>
