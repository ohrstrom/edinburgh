<script setup lang="ts">
import { ComputedRef, Ref, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'

import { EDI } from '../../pkg'

import {initDecoder, decodeAAC} from './lib/decoder.js'

import type * as Types from '@/types'

import { useEDIStore } from '@/stores/edi'

import Connection from '@/components/edi/connection/Connection.vue'
import Ensemble from '@/components/edi/ensemble/Ensemble.vue'
import ServiceDetail from '@/components/edi/service-detail/Service.vue'
import ServiceList from '@/components/edi/service-list/Services.vue'

interface Analyser {
  l: AnalyserNode
  r: AnalyserNode
}

class EDInburgh {
  edi: EDI
  ws: WebSocket | null = null
  audioContext: AudioContext | null = null
  workletNode: AudioWorkletNode | null = null
  decoder: AudioDecoder | null = null
  analyser: Analyser | null = null
  analyserReading = false

  // faad decoder
  faad: any = null

  // Store methods
  updateEnsemble: typeof useEDIStore.prototype.updateEnsemble
  updateDL: typeof useEDIStore.prototype.updateDL
  updateSLS: typeof useEDIStore.prototype.updateSLS
  selectService: typeof useEDIStore.prototype.selectService

  // Reactive store state
  connected: Ref<boolean>
  selectedService: ComputedRef<Types.Service | undefined>

  volume: Ref<Types.Volume>

  constructor({
    updateEnsemble,
    updateDL,
    updateSLS,
    selectService,
    //
    connected,
    selectedService,
  }: {
    updateEnsemble: typeof useEDIStore.prototype.updateEnsemble
    updateDL: typeof useEDIStore.prototype.updateDL
    updateSLS: typeof useEDIStore.prototype.updateSLS
    selectService: typeof useEDIStore.prototype.selectService
    //
    connected: Ref<boolean>
    selectedService: ComputedRef<Types.Service | undefined>
  }) {
    console.log('EDInburgh:init')

    // pinia store mappings
    this.updateEnsemble = updateEnsemble
    this.updateDL = updateDL
    this.updateSLS = updateSLS
    this.selectService = selectService
    //
    this.connected = connected
    this.selectedService = selectedService

    this.volume = ref<Types.Volume>({
      l: 0,
      r: 0,
    })

    /******************************************************************
     EDI Events / Callbacks
     ******************************************************************/


    const edi = new EDI()
    edi.on_ensemble_update(async (data: Types.Ensemble) => {
      await this.updateEnsemble(data)
    })

    edi.on_mot_image_received(async (data: Types.SLS) => {
      await this.updateSLS(data)
    })

    edi.on_dl_object_received(async (data: Types.DL) => {
      await this.updateDL(data)
    })

    edi.on_aac_segment(async (aacSegment) => {
      const selected = this.selectedService.value
      if (!selected) return

      if (aacSegment.scid !== selected.scid) {
        return
      }

      aacSegment.frames.forEach((frame) => {
        this.processAACSegment(new Uint8Array(frame))
      });

    })

    this.edi = edi

    // Watch for selected service changes
    watch(
        this.selectedService,
        (newVal, oldVal) => {
          if (newVal?.scid !== oldVal?.scid) {
            console.debug("Service selected:", newVal)
            ;(async () => {
              await this.resetAudioDecoder()
              await this.startAnalyser()
            })()
          }
        },
        { immediate: true }
    )

    // initDecoder(new Uint8Array([0x12, 0x10]))  // Most standard

    const asc = new Uint8Array([0x13, 0x14, 0x56, 0xe5, 0x98])
    const dec = initDecoder(asc)

    this.faad = dec

  }

  async connect(conn: { host: string; port: number }): Promise<void> {
    const uri = `ws://localhost:9000/ws/${conn.host}/${conn.port}/`
    console.log('EDInburgh:connect', conn.host, conn.port, uri)

    const ws = new WebSocket(uri)

    ws.binaryType = 'arraybuffer'

    /******************************************************************
    Websocket Events
    ******************************************************************/
    ws.onmessage = (event) => {
      this.edi.feed(new Uint8Array(event.data))
    }

    ws.onclose = () => {
      console.info('WebSocket closed')
      this.connected.value = false
      this.ws = null
    }

    ws.onerror = (e) => {
      console.error('WebSocket error:', e)
    }

    this.ws = ws
    this.connected.value = true

    if (!this.decoder) {
      await this.initializeAudioDecoder()
    }

  }

  async reset(): Promise<void> {
    console.log('EDInburgh:reset')

    if (this.ws) {
      this.ws.onopen = this.ws.onmessage = this.ws.onerror = null

      try {
        this.ws.close(1000, 'Client disconnecting')

        await new Promise<void>((resolve) => {
          this.ws!.onclose = () => {
            resolve()
          }
        })
      } catch (e) {
        console.warn('WebSocket close error:', e)
      }

      this.ws = null
    }

    await this.edi.reset()

    this.connected.value = false
  }

  async initializeAudioDecoder(): Promise<void> {
    console.log('EDInburgh:initializeAudioDecoder')
    if (this.decoder) {
      console.info('decoder already initialized')
      return
    }

    const audioContext = new AudioContext({
      latencyHint: 'balanced',
      // sampleRate: 48000,
      sampleRate: 24000,
    })

    await audioContext.audioWorklet.addModule('pcm-processor.js')

    const workletNode = new AudioWorkletNode(audioContext, 'pcm-processor', {
      outputChannelCount: [2],
    })

    /* single / sum analyser
    const analyser = audioContext.createAnalyser()
    analyser.fftSize = 256
    // connect worklet → analyser → destination
    workletNode.connect(analyser)
    analyser.connect(audioContext.destination)
    */

    // channel splitter
    const splitter = audioContext.createChannelSplitter(2)

    // L/R analysers
    const analyserL = audioContext.createAnalyser()
    const analyserR = audioContext.createAnalyser()

    analyserL.fftSize = 256
    analyserR.fftSize = 256

    // connect chain:
    // worklet → splitter
    // splitter → analyserL (channel 0), analyserR (channel 1)
    workletNode.connect(splitter)

    splitter.connect(analyserL, 0)
    splitter.connect(analyserR, 1)

    // NOTE: this fuck's things up!!
    // analyserL.connect(audioContext.destination)
    // analyserR.connect(audioContext.destination)

    // NOTE: just connect the context..
    workletNode.connect(audioContext.destination)


    const decoder = new AudioDecoder({
      output: (audioData) => {
        // console.debug("decoded", audioData)
        this.playDecodedAudio(audioData)
      },
      error: (e) => console.error('Decoder error:', e),
    })

    this.audioContext = audioContext
    this.workletNode = workletNode
    this.decoder = decoder
    this.analyser = {
      l: analyserL,
      r: analyserR,
    }
  }
  async resetAudioDecoder(): Promise<void> {
    console.log('EDInburgh:resetAudioDecoder')

    if (!this.decoder) {
      console.info('decoder not initialized')
      return
    }

    if (!this.workletNode) {
      console.info('worklet node not initialized')
      return
    }

    this.decoder.reset()

    const asc = new Uint8Array([0x13, 0x14, 0x56, 0xe5, 0x98])
    // const asc = new Uint8Array([0x13, 0x14])
    this.decoder.configure({
      codec: 'mp4a.40.5',
      sampleRate: 48000,
      numberOfChannels: 2,
      description: asc.buffer,
    })

    this.workletNode.port.postMessage({
      type: 'reset',
    })

  }

  async __processAACSegment(aacSegment): Promise<void> {
    if (!this.faad) {
      console.info('FAAD not initialized')
      return
    }

    // console.debug("buffer", aacSegment.buffer)

    const pcmData = decodeAAC(new Uint8Array(aacSegment.buffer))

    if (pcmData) {
      this.workletNode?.port.postMessage({
        type: 'audio',
        samples: pcmData,
      })
    }
  }


  async processAACSegment(aacSegment): Promise<void> {

    if (!this.decoder) {
      console.info('decoder not initialized')
      return
    }

    if (!this.audioContext) {
      console.info('context not initialized')
      return
    }

    const chunk = new EncodedAudioChunk({
      type: 'key',
      timestamp: this.audioContext.currentTime * 1e6,
      data: aacSegment.buffer,
    })

    try {
      this.decoder.decode(chunk)
    } catch (e) {
      console.error('Decoder error:', e)
    }
  }


  async playDecodedAudio(audioData): Promise<void> {

    if (!this.workletNode) {
      console.info('worklet not initialized')
      return
    }

    const numChannels = 2
    const numFrames = audioData.numberOfFrames

    const pcmData = [new Float32Array(numFrames), new Float32Array(numFrames)]


    for (let channel = 0; channel < numChannels; channel++) {
      audioData.copyTo(pcmData[channel], { planeIndex: channel })
    }


    // console.debug("pcmData", pcmData)

    this.workletNode.port.postMessage({
      type: 'audio',
      samples: pcmData,
    })
  }

  async startAnalyser(): Promise<void> {
    if (!this.analyser) {
      console.info('analyser not initialized')
      return
    }

    if (this.analyserReading) {
      console.info('analyser already reading')
      return
    }

    this.analyserReading = true
    this.analyserLoop()
  }

  analyserLoop = () => {
    if (!this.analyser || !this.analyserReading) return

    const { l, r } = this.analyser

    const bufferL = new Uint8Array(l.frequencyBinCount)
    const bufferR = new Uint8Array(r.frequencyBinCount)

    l.getByteFrequencyData(bufferL)
    r.getByteFrequencyData(bufferR)

    const avgL = bufferL.reduce((sum, v) => sum + v, 0) / bufferL.length
    const avgR = bufferR.reduce((sum, v) => sum + v, 0) / bufferR.length

    const volumeL = avgL / 255
    const volumeR = avgR / 255

    // const dBFSL = 20 * Math.log10(Math.max(volumeL, 0.00001))
    // const dBFSR = 20 * Math.log10(Math.max(volumeR, 0.00001))

    // console.debug("L:", dBFSL, "R:", dBFSR)

    this.volume.value = {
      l: volumeL,
      r: volumeR,
    }

    requestAnimationFrame(this.analyserLoop)
  }
}

const ediStore = useEDIStore()

const { updateEnsemble, updateDL, updateSLS, selectService, reset: resetStore } = ediStore

const { connected, selectedService } = storeToRefs(ediStore)

const edinburgh = new EDInburgh({
  updateEnsemble,
  updateDL,
  updateSLS,
  selectService,
  //
  connected,
  selectedService,
})

/*
const connect = async (conn: { host: string, port: number }) => {
  await edinburgh.connect(conn)
}
*/

const connect = edinburgh.connect.bind(edinburgh)

// const reset = edinburgh.reset.bind(edinburgh)

const reset = async () => {
  await edinburgh.reset()
  await resetStore()
}

</script>

<template>
  <!--
  <pre v-text="edinburgh" />
  -->
  <main>
    <header>
      <Ensemble />
      <Connection @connect="connect" @reset="reset" />
    </header>

    <section class="service-detail">
      <ServiceDetail :volume="edinburgh.volume.value" />
    </section>

    <section class="service-list">
      <ServiceList @play="edinburgh.selectService" @select="edinburgh.selectService" />
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
