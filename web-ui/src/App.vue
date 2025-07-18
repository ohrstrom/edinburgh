<script setup lang="ts">
import { ComputedRef, Ref, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useStorage } from '@vueuse/core'

// TODO: how to import cross-package?
import { EDI } from '../../wasm/pkg'

import FAAD2Decoder from '@/decoder/faad2'

import type * as Types from '@/types'

import { useEDIStore } from '@/stores/edi'
import { usePlayerStore } from '@/stores/player'

import Panel from '@/components/ui/Panel.vue'
import Connection from '@/components/edi/connection/Connection.vue'
import Settings from '@/components/edi/settings/Settings.vue'
import Ensemble from '@/components/edi/ensemble/Ensemble.vue'
import ServiceTable from '@/components/edi/ensemble/ServiceTable.vue'
import ServiceDetail from '@/components/edi/service-detail/Service.vue'
import ServiceList from '@/components/edi/service-list/Services.vue'

import EnsembleTable from '@/components/directory/EnsembleTable.vue'
import CodecSupport from '@/components/dev/CodecSupport.vue'


const resample = async (
  buffer: Float32Array,
  sourceRate: number,
  targetRate: number
): Promise<Float32Array> => {
  if (sourceRate === targetRate) {
    return buffer; // no resampling needed
  }

  const numFrames = buffer.length;

  // Calculate target length
  const targetLength = Math.ceil(numFrames * targetRate / sourceRate);

  // Create OfflineAudioContext
  const offlineContext = new OfflineAudioContext({
    numberOfChannels: 1,
    length: targetLength,
    sampleRate: targetRate,
  });

  // Create buffer in source rate
  const audioBuffer = offlineContext.createBuffer(1, numFrames, sourceRate);
  audioBuffer.copyToChannel(buffer, 0);

  // Buffer source
  const sourceNode = offlineContext.createBufferSource();
  sourceNode.buffer = audioBuffer;
  sourceNode.connect(offlineContext.destination);
  sourceNode.start();

  // Render
  const resampledBuffer = await offlineContext.startRendering();

  return resampledBuffer.getChannelData(0);
};

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
  gainFadeNode: GainNode | null = null
  // decoder: AudioDecoder | null = null
  decoder: FAAD2Decoder | AudioDecoder | null = null
  useFAAD2Decoder: boolean = true
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

    this.edi = edi

    // Watch for selected SID changes
    watch(
    () => this.selectedService.value?.scid,
      async (newScid, oldScid) => {
        if (newScid !== oldScid) {
          await this.resetAudioDecoder(this.selectedService.value?.audioFormat)
          await this.startAnalyser()
          await this.fadeIn(0.2)
        }
        // this.decodeAudio = true
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

    console.log('EDInburgh: initializeAudioDecoder')

    if (this.decoder) {
      console.info('EDInburgh: decoder already initialized')
      return
    }

    const sampleRate = this.useFAAD2Decoder ? 48_000 : 24_000

    const audioContext = new AudioContext({
      latencyHint: 'balanced',
      sampleRate,
      // sampleRate: 48_000, // when using faad2 decoder
      // sampleRate: 24_000, // when using browser nadive decoder
    })

    await audioContext.audioWorklet.addModule('pcm-processor.js')

    const workletNode = new AudioWorkletNode(audioContext, 'pcm-processor', {
      outputChannelCount: [2],
    })

    // channel splitter
    const splitter = audioContext.createChannelSplitter(2)

    // L/R analysers
    const analyserL = audioContext.createAnalyser()
    const analyserR = audioContext.createAnalyser()

    analyserL.fftSize = 8192
    analyserR.fftSize = 8192

    // connect chain:
    // worklet → splitter
    // splitter → analyserL (channel 0), analyserR (channel 1)
    workletNode.connect(splitter)

    splitter.connect(analyserL, 0)
    splitter.connect(analyserR, 1)

    // user-controlled volume control
    const gainNode = audioContext.createGain()
    gainNode.gain.value = this.volume

    // Fade in/out control
    const gainFadeNode = audioContext.createGain();
    // gainFadeNode.gain.value = 1.0;
    gainFadeNode.gain.setValueAtTime(0.0, audioContext.currentTime);

    workletNode.connect(gainNode)
    gainNode.connect(gainFadeNode)

    gainFadeNode.connect(audioContext.destination)

    // const decoder = new AudioDecoder({
    // const decoder = new FAAD2Decoder({
    //   output: (audioData) => {
    //     this.playDecodedAudio(audioData)
    //   },
    //   error: (e) => console.error('Decoder error:', e),
    // })

    let decoder: FAAD2Decoder | AudioDecoder | null = null

    if (this.useFAAD2Decoder) {
      decoder = new FAAD2Decoder({
        output: (audioData) => {
          this.playDecodedAudio(audioData)
        },
        error: (e) => console.error('Decoder error:', e),
      })
    } else {
      decoder = new AudioDecoder({
        output: (audioData) => {
          this.playDecodedAudio(audioData)
        },
        error: (e) => console.error('Decoder error:', e),
      })
    }

    this.audioContext = audioContext
    this.workletNode = workletNode
    this.gainNode = gainNode
    this.gainFadeNode = gainFadeNode
    this.decoder = decoder
    this.analyser = {
      l: analyserL,
      r: analyserR,
    }
  }
  async resetAudioDecoder(audioFormat): Promise<void> {
    console.log('EDInburgh: resetAudioDecoder', audioFormat)

    if (!this.decoder) {
      console.info('EDInburgh: decoder not initialized')
      return
    }

    if (!this.workletNode) {
      console.info('EDInburgh: worklet node not initialized')
      return
    }

    this.setPlayerState('stopped')

    this.decoder.reset()

    // const asc = new Uint8Array([0x13, 0x14, 0x56, 0xe5, 0x98]) // HE-AAV v1
    // const asc = new Uint8Array([0x13, 0x0C, 0x56, 0xE5, 0x9D, 0x48, 0x80]); // HE-AAV v2 24 kHz
    // const asc = new Uint8Array([0x14, 0x0C, 0x56, 0xE5, 0xAD, 0x48, 0x80]); // HE-AAV v2 16 kHz

    // NOTE: this is just for testing...
    // let asc = new Uint8Array([0x13, 0x14, 0x56, 0xe5, 0x98]) // HE-AAV v1
    // let codec = 'mp4a.40.5'

    // if (audioFormat.codec === "HE-AACv2") {
    //     asc = new Uint8Array([0x13, 0x0C, 0x56, 0xE5, 0x9D, 0x48, 0x80]) // HE-AAV v2 24 kHz
    //     codec = 'mp4a.40.29'
    // }

    let codec = 'mp4a.40.5'
    const asc = new Uint8Array(audioFormat?.asc ?? [])


    await this.decoder.configure({
      codec,
      // codec: 'mp4a.40.5', // HE-AAV v1
      // codec: 'mp4a.40.29', // HE-AAV
      sampleRate: 48_000,
      numberOfChannels: 2,
      description: asc.buffer,
    })

    this.workletNode.port.postMessage(
      {
        type: 'reset',
      }
    )

    // NOTE: is this a good idea?
    await new Promise<void>((resolve) => {
        const timeout = setTimeout(() => {
            resolve()
        }, 10)
    })
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

    if (!this.decodeAudio) {
      console.info('decodeAudio disabled')
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

    // console.debug('EDInburgh: AD', audioData)


    const numChannels = audioData.numberOfChannels
    const numFrames = audioData.numberOfFrames

    let pcmData = [new Float32Array(numFrames), new Float32Array(numFrames)]

    for (let channel = 0; channel < numChannels; channel++) {
      audioData.copyTo(pcmData[channel], { planeIndex: channel })
      if (numChannels === 1) {
        // If mono, duplicate the channel to both L and R
        pcmData[1] = pcmData[0]
      }
    }

    const sampleRate = audioData.sampleRate
    // const sampleRate = 32_000

    if (sampleRate !== this.audioContext.sampleRate) {
      // console.warn('EDInburgh: sample rate mismatch', audioData.sampleRate, this.audioContext.sampleRate)

      pcmData[0] = await resample(
        pcmData[0],
        sampleRate,
        this.audioContext.sampleRate
      )

      pcmData[1] = await resample(
        pcmData[1],
        sampleRate,
        this.audioContext.sampleRate
      )

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

  async fadeTo(value: number = 1.0, time: number = 1.0): Promise<void> {

    if (!this.decoder || !this.gainFadeNode || !this.audioContext) {
      return
    }

    console.debug('EDInburgh: fade in', time)

    const startTime = this.audioContext.currentTime;
    const endTime = startTime + time;

    this.gainFadeNode.gain.cancelScheduledValues(startTime);
    this.gainFadeNode.gain.setValueAtTime(this.gainFadeNode.gain.value, startTime);
    this.gainFadeNode.gain.linearRampToValueAtTime(value, endTime);

    // Wait until the fade completes using wall clock
    const now = this.audioContext.currentTime;
    const remaining = Math.max(0, endTime - now);
    await new Promise<void>((resolve) => {
      setTimeout(resolve, remaining * 1000);
    });
  }

  async fadeIn(time: number = 0.5): Promise<void> {
    return await this.fadeTo(1.0, time)
  }

  async fadeOut(time: number = 0.5): Promise<void> {
    return await this.fadeTo(0.0, time)
  }

  async playService(sid: number): Promise<void> {
    console.debug('EDInburgh: play service', sid)

    if (sid === this.selectedService.value?.sid) {
      console.info('EDInburgh: already playing service', sid)
      return
    }

    if (this.decodeAudio) {
      await this.fadeOut(0.1)
    }
    this.selectService(sid)
    this.decodeAudio = true
  }

  async stopService(): Promise<void> {
    console.debug('EDInburgh: stop service')
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

    const bufferL = new Float32Array(l.fftSize)
    const bufferR = new Float32Array(r.fftSize)

    l.getFloatTimeDomainData(bufferL)
    r.getFloatTimeDomainData(bufferR)

    const rmsLength = 2048 // You can adjust this value

    const sliceL = bufferL.slice(bufferL.length - rmsLength)
    const sliceR = bufferR.slice(bufferR.length - rmsLength)

    const rmsL = Math.hypot(...sliceL) / Math.sqrt(rmsLength)
    const rmsR = Math.hypot(...sliceR) / Math.sqrt(rmsLength)

    this.level.value = {
      l: rmsL,
      r: rmsR,
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


const { ediHost, ediPort } = storeToRefs(useEDIStore())

const selectEnsemble = async (conn: { host: string; port: number }) => {
  await edinburgh.reset()
  await resetStore()
  ediHost.value = conn.host
  ediPort.value = conn.port
  await connect(conn)
}


// ui states - maybe place somewhere else ;)


const serviceTableExpanded = useStorage('edi/ensemble/service-table/expanded', false)
const ensembleTableExpanded = useStorage('edi/ensemble/ensemble-table/expanded', false)

const toggleServiceTable = () => {
  ensembleTableExpanded.value = false
  serviceTableExpanded.value = !serviceTableExpanded.value
}
const toggleEnsembleTable = () => {
  serviceTableExpanded.value = false
  ensembleTableExpanded.value = !ensembleTableExpanded.value
}

</script>

<template>
  <!--
  <pre v-text="edinburgh" />
  -->
  <main>
    <Panel v-if="false">
      <CodecSupport />
    </Panel>
    <Panel class="header">
      <template #header>
        <Settings />
      </template>
      <Ensemble />
      <div>
        <Connection @connect="connect" @reset="reset" />
      </div>
      <template #sub-navigation>
        <div class="sub-navigation">
          <div @click.prevent="toggleServiceTable()" class="toggle">
            <span class="label">Service Table</span>
            <span v-if="serviceTableExpanded" class="icon icon--close">⌃</span>
            <span v-else class="icon icon--open">⌄</span>
          </div>
          <div @click.prevent="toggleEnsembleTable()" class="toggle">
            <span class="label">Ensemble Discovery</span>
            <span v-if="ensembleTableExpanded" class="icon icon--close">⌃</span>
            <span v-else class="icon icon--open">⌄</span>
          </div>
        </div>
      </template>
      <template #sub-content>
        <ServiceTable v-if="serviceTableExpanded" @select="(sid) => edinburgh.playService(sid)" />
        <EnsembleTable v-if="ensembleTableExpanded" @select="selectEnsemble" />
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

<style lang="scss" scoped>
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

    .sub-navigation {
      display: flex;
      gap: 8px;
      padding: 0 8px;
      border-top: 1px solid black;
      justify-content: space-between;
      .toggle {
        display: flex;
        align-items: center;
        gap: 4px;
        height: 24px;
        cursor: pointer;

        > .label {
          font-size: 0.75rem;
        }

        > .icon {
          &--open {
            margin-top: -9px;
          }
          &--close {
            margin-top: 5px;
          }
        }
      }
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
