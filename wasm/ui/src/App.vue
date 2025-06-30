<script setup lang="ts">
import { ComputedRef, Ref, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'

import { EDI } from '../../pkg'

import { decodeAAC, initDecoder } from './lib/decoder.js'

import FAAD2Decoder from '@/decoder/faad2'

import type * as Types from '@/types'

import { useEDIStore } from '@/stores/edi'
import { usePlayerStore } from '@/stores/player'

import Panel from '@/components/ui/Panel.vue'
import Connection from '@/components/edi/connection/Connection.vue'
import Scanner from '@/components/edi/connection/Scanner.vue'
import Settings from '@/components/edi/settings/Settings.vue'
import Ensemble from '@/components/edi/ensemble/Ensemble.vue'
import EnsembleTable from '@/components/edi/ensemble/EnsembleTable.vue'
import ServiceDetail from '@/components/edi/service-detail/Service.vue'
import ServiceList from '@/components/edi/service-list/Services.vue'

import CodecSupport from '@/components/dev/CodecSupport.vue'

interface Analyser {
  l: AnalyserNode
  r: AnalyserNode
}

class EDInburgh {
  edi: EDI
  ws: WebSocket | null = null
  audioContext: AudioContext | null = null
  workletNode: AudioWorkletNode | null = null
  gainNode: GainNode | null = null
  // decoder: AudioDecoder | null = null
  decoder: FAAD2Decoder | null = null
  analyser: Analyser | null = null
  analyserReading = false

  // faad decoder
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  faad: any = undefined

  decodeAudio: boolean = false
  volume: number = 0

  // "local" subchannel state
  // not used: at the moment directly calls ediStore - check performance...
  // subchannels: Map<number, Types.Subchannel> = new Map()

  // Store methods
  updateEnsemble: typeof useEDIStore.prototype.updateEnsemble
  updateDL: typeof useEDIStore.prototype.updateDL
  updateSLS: typeof useEDIStore.prototype.updateSLS
  selectService: typeof useEDIStore.prototype.selectService
  setAudioFormat: typeof useEDIStore.prototype.setAudioFormat
  setPlayerState: typeof usePlayerStore.prototype.setState

  // Reactive store state
  connected: Ref<boolean>
  selectedService: ComputedRef<Types.Service | undefined>
  playerVolume: ComputedRef<number>

  level: Ref<Types.Level>

  constructor({
    updateEnsemble,
    updateDL,
    updateSLS,
    selectService,
    setAudioFormat,
    setPlayerState,
    //
    connected,
    selectedService,
    playerVolume,
  }: {
    updateEnsemble: typeof useEDIStore.prototype.updateEnsemble
    updateDL: typeof useEDIStore.prototype.updateDL
    updateSLS: typeof useEDIStore.prototype.updateSLS
    selectService: typeof useEDIStore.prototype.selectService
    setAudioFormat: typeof useEDIStore.prototype.setAudioFormat
    setPlayerState: typeof usePlayerStore.prototype.setState
    //
    connected: Ref<boolean>
    selectedService: ComputedRef<Types.Service | undefined>
    playerVolume: ComputedRef<number>
  }) {
    console.log('EDInburgh:init')

    // pinia store mappings
    this.updateEnsemble = updateEnsemble
    this.updateDL = updateDL
    this.updateSLS = updateSLS
    this.selectService = selectService
    this.setAudioFormat = setAudioFormat
    this.setPlayerState = setPlayerState
    //
    this.connected = connected
    this.selectedService = selectedService
    this.playerVolume = playerVolume

    this.level = ref<Types.Level>({
      l: 0,
      r: 0,
    })

    /******************************************************************
     EDI Events / Callbacks
     ******************************************************************/

    const edi = new EDI()

    edi.addEventListener("ensemble_updated", async (e: CustomEvent) => {
      await this.updateEnsemble(e.detail as Types.Ensemble)
    })

    edi.addEventListener("mot_image", async (e: CustomEvent) => {
      await this.updateSLS(e.detail as Types.SLS)
    })

    edi.addEventListener("dl_object", async (e: CustomEvent) => {
      await this.updateDL(e.detail as Types.DL)
    })

    edi.addEventListener("aac_segment", async (e: CustomEvent) => {
      const aacSegment = e.detail as Types.AACSegment

      this.setAudioFormat(aacSegment.scid, aacSegment.audio_format)

      if (!this.decodeAudio) {
        return
      }

      const selected = this.selectedService.value
      if (!selected) {
        return
      }

      if (aacSegment.scid !== selected.scid) {
        return
      }

      aacSegment.frames.forEach((frame) => {
        this.processAACSegment(new Uint8Array(frame))
      })

    })

    /*
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


      // console.debug('AAC segment:', aacSegment)

      this.setAudioFormat(aacSegment.scid, aacSegment.audio_format)

      if (!this.decodeAudio) {
        return
      }

      const selected = this.selectedService.value
      if (!selected) {
        return
      }

      if (aacSegment.scid !== selected.scid) {
        return
      }

      // console.debug('AAC segment:', aacSegment)

      aacSegment.frames.forEach((frame) => {
        this.processAACSegment(new Uint8Array(frame))
      })
    })
    */

    this.edi = edi

    // Watch for selected service changes
    /*
    watch(
      this.selectedService,
      (newVal, oldVal) => {
        if (newVal?.scid !== oldVal?.scid) {
          console.debug('Service selected:', newVal)
          ;(async () => {
            await this.resetAudioDecoder()
            await this.startAnalyser()
          })()
        }
        console.debug("selectedService", oldVal, newVal)
        this.decodeAudio = true
      },
      { immediate: true },
    )
    */

    // Watch for selected SID changes
    watch(
    () => this.selectedService.value?.scid,
      async (newScid, oldScid) => {
        if (newScid !== oldScid) {
          await this.resetAudioDecoder()
          await this.startAnalyser()
        }
        this.decodeAudio = true
      },
      { immediate: true }
    )

    // Watch settings
    watch(
    () => this.playerVolume.value,
      async (val) => {
        this.volume = val
        if (this.gainNode) {
          this.gainNode.gain.value = val
        }
      },
      { immediate: true }
    )

    // initDecoder(new Uint8Array([0x12, 0x10]))  // Most standard
    // const asc = new Uint8Array([0x13, 0x14, 0x56, 0xe5, 0x98])
    // // const asc = new Uint8Array([0x14, 0x0C, 0x56, 0xE5, 0xAD, 0x48, 0x80]); // HE-AAV v2
    // const dec = initDecoder(asc)
    // this.faad = dec
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
      this.ws = undefined
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
      this.ws.onopen = this.ws.onmessage = this.ws.onerror = undefined

      try {
        this.ws.close(1000, 'Client disconnecting')

        await new Promise<void>((resolve) => {
          this.ws!.onclose = () => {
            resolve()
          }
        })
      } catch (err) {
        console.warn('WebSocket close error:', err)
      }

      this.ws = undefined
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
      sampleRate: 48_000,
      // sampleRate: 24_000,
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

    const gainNode = audioContext.createGain()
    gainNode.gain.value = this.volume

    // NOTE: just connect the context..
    // workletNode.connect(audioContext.destination)

    workletNode.connect(gainNode)
    gainNode.connect(audioContext.destination)

    // const decoder = new AudioDecoder({
    const decoder = new FAAD2Decoder({
      output: (audioData) => {
        // console.debug("decoded", audioData)
        this.playDecodedAudio(audioData)
      },
      error: (e) => console.error('Decoder error:', e),
    })

    this.audioContext = audioContext
    this.workletNode = workletNode
    this.gainNode = gainNode
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

    this.setPlayerState('stopped')

    this.decoder.reset()

    const asc = new Uint8Array([0x13, 0x14, 0x56, 0xe5, 0x98]) // HE-AAV v1

    // const asc = new Uint8Array([0x14, 0x0C, 0x56, 0xE5, 0xAD, 0x48, 0x80]); // HE-AAV v2

    // const asc = new Uint8Array([0x13, 0x14, 0x56, 0xe5, 0x99, 0x00])

    await this.decoder.configure({
      codec: 'mp4a.40.5',
      // codec: 'mp4a.40.29',
      sampleRate: 48_000,
      numberOfChannels: 2,
      description: asc.buffer,
    })

    await new Promise<void>((resolve) => {
        const timeout = setTimeout(() => {
            console.log('ENC: configured')
            resolve()
        }, 100)
    })

    this.workletNode.port.postMessage(
      {
        type: 'reset',
      }
    )
  }

  async __processAACSegment(aacSegment): Promise<void> {
    if (!this.faad) {
      console.info('FAAD not initialized')
      return
    }

    // console.debug("buffer", aacSegment.buffer)

    const pcmData = decodeAAC(new Uint8Array(aacSegment.buffer))

    if (pcmData) {
      this.workletNode?.port.postMessage(
        {
          type: 'audio',
          samples: pcmData,
        }
      )
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
    } catch (err) {
      console.warn('Decoder error:', err)
      // await this.resetAudioDecoder()
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

    this.setPlayerState('playing')

    this.workletNode.port.postMessage(
      {
        type: 'audio',
        samples: pcmData,
      }
    )
  }

  async playService(sid: number): Promise<void> {
    this.selectService(sid)
    this.decodeAudio = true
  }

  async stopService(): Promise<void> {
    console.debug('stopService')
    this.decodeAudio = false
    this.setPlayerState('stopped')
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
    if (!this.analyser || !this.analyserReading) {
      return
    }

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

    this.level.value = {
      l: volumeL,
      r: volumeR,
    }

    requestAnimationFrame(this.analyserLoop)
  }
}

const ediStore = useEDIStore()

const playerStore = usePlayerStore()

const { updateEnsemble, updateDL, updateSLS, selectService, setAudioFormat, reset: resetStore } = ediStore

const { setState: setPlayerState } = playerStore

const { connected, selectedService } = storeToRefs(ediStore)

const { volume: playerVolume } = storeToRefs(playerStore)

const edinburgh = new EDInburgh({
  updateEnsemble,
  updateDL,
  updateSLS,
  selectService,
  setAudioFormat,
  setPlayerState,
  //
  connected,
  selectedService,
  playerVolume,
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
    <Panel>
      <CodecSupport />
    </Panel>
    <Panel class="header">
      <template #header>
        <Settings />
      </template>
      <Ensemble />
      <div>
        <Connection @connect="connect" @reset="reset" />
        <!--
        <Scanner :edi="edinburgh.edi" />
        -->
      </div>
      <template #footer>
        <EnsembleTable @select="(sid) => edinburgh.playService(sid)" />
      </template>
    </Panel>

    <Panel class="service-detail">
      <ServiceDetail :level="edinburgh.level.value" />
    </Panel>

    <Panel class="service-list">
      <ServiceList @play="(sid) => edinburgh.playService(sid)" @select="(sid) => edinburgh.playService(sid)" @stop="() => edinburgh.stopService()" />
    </Panel>
  </main>
</template>

<style scoped>
main {
  width: 100%;
  height: 100vh;
  max-width: 1024px;
  margin-inline: auto;
  display: flex;
  flex-direction: column;
  background: hsl(var(--c-bg-muted));

  > .header {
    display: grid;
    grid-template-columns: 1fr 324px;
    gap: 12px;

    margin-top: 20px;
    margin-bottom: 20px;
    padding: 8px;
    padding-bottom: 0;

    .settings {
      border-bottom: 1px solid black;
    }
  }

  .service-detail {
    margin-bottom: 20px;
    padding: 8px;
  }

  .service-list {
    flex-grow: 1;
    overflow-y: auto;
    margin-bottom: 20px;

    /* scrollbar */
    &::-webkit-scrollbar {
      width: 4px;
      background: hsl(var(--c-muted));
    }

    &::-webkit-scrollbar-thumb {
      background: hsl(var(--c-fg));
      border-radius: 0;
    }
  }
}

button {
  margin: 5px;
  padding: 8px 12px;
  cursor: pointer;
}
</style>
