import { computed, shallowRef, ref } from 'vue'
import { useStorage } from '@vueuse/core'
import { useIDBKeyval } from '@vueuse/integrations/useIDBKeyval'
import { defineStore } from 'pinia'

import type * as Types from '@/types'

import { usePlayerStore } from './player'

// SLS cache
const keyFor = (sid: number) => `edi:sls:${sid}`

export function useSLSCache(sid: number) {
  const initial = shallowRef<Types.SLS | null>(null)

  const { data, isFinished, set } = useIDBKeyval<Types.SLS | null>(keyFor(sid), initial)

  const remove = async () => {
    await set(null)
  }

  return {
    data,
    set,
    isFinished,
    delete: remove,
  }
}

export const useEDIStore = defineStore('edi', () => {
  const playerStore = usePlayerStore()

  const connected = ref(false)

  // const ensemble = ref<Ensemble | null>(null)
  const ensemble = ref<Types.Ensemble>({
    eid: 0,
    services: [],
    subchannels: [],
  })

  const audioFormats = ref<Map<number, Types.AudioFormat>>(new Map())

  const dls = ref<Types.DL[]>([])
  const sls = ref<Types.SLS[]>([])

  const selectedSid = ref(0)

  const services = computed(() => {
    if (!ensemble.value) return []

    return ensemble.value.services
      .filter((svc) => svc.label !== undefined)
      .flatMap((svc) =>
        svc.components.map((comp) => ({
          sid: svc.sid,
          scid: comp.scid,
          label: svc.label,
          short_label: svc.short_label,
          language: comp.language,
          user_apps: comp.user_apps ?? [],
          subchannel: ensemble.value.subchannels.find((sc) => sc.id === comp.subchannel_id),
          audioFormat: audioFormats.value.get(comp.scid),
          dl: dls.value.find((v) => v.scid === comp.scid),
          sls: sls.value.find((v) => v.scid === comp.scid),
          isPlaying: svc.sid === selectedSid.value && playerStore.state === 'playing',
          // isPlaying: playerStore.state === 'playing',
        })),
      )
      .sort((a, b) => a.label!.localeCompare(b.label!))
  })

  const selectedService = computed(() => {
    return services.value.find((svc) => svc.sid === selectedSid.value)
  })

  const ediHost = useStorage('edi/host', 'edi-ch.digris.net', localStorage, {
    serializer: {
      read: (v) => v,
      write: (v) => v,
    },
  })

  const ediPort = useStorage('edi/port', 8855, localStorage, {
    serializer: {
      read: (v: string) => Number(v),
      write: (v: number) => v.toString(),
    },
  })

  const reset = async () => {
    console.debug('STORE: reset')
    ensemble.value = {
      eid: 0,
      services: [],
      subchannels: [],
    }
    audioFormats.value.clear()
    dls.value = []
    sls.value = []
    selectedSid.value = 0
  }

  const selectService = async (sid: number) => {
    console.debug('STORE: selectService', sid)
    selectedSid.value = sid
  }

  const updateEnsemble = async (val: Types.Ensemble) => {
    console.debug('STORE: updateEnsemble', val)
    ensemble.value = val

    // read SLS cache NOTE: not working... SIDs are messed up i think..
    /*
    ensemble.value.services.filter((svc) => svc.sid !== undefined).forEach((svc) => {
      const { data, isFinished } = useSLSCache(svc.sid!)
      watch(isFinished, () => {
        if (isFinished.value && data.value) {
          updateSLS(data.value)
        }
      }, { immediate: true })
    })
    */
  }

  const setAudioFormat = async (scid: number, val: Types.AudioFormat) => {
    // console.debug('STORE: setAudioFormat', scid, val)
    // NOTE: i assume this does not change (at least not in the same session)
    if (!audioFormats.value.has(scid)) {
      audioFormats.value.set(scid, val)
    }
  }

  const updateDL = async (val: Types.DL) => {
    const index = dls.value.findIndex((v) => v.scid === val.scid)
    if (-1 !== index) {
      dls.value[index] = val
    } else {
      dls.value.push(val)
    }
  }

  // oxlint-disable-next-line @typescript-eslint/no-unused-vars
  const saveSLS = async (val: Types.SLS) => {
    const svc = services.value.find((s) => s.scid === val.scid)
    if (!svc) {
      console.debug('SLS: service not found', val.scid)
      return
    }
    const { set } = useSLSCache(svc.sid)
    await set(val)
  }

  const updateSLS = async (val: Types.SLS) => {
    const index = sls.value.findIndex((v) => v.scid === val.scid)

    // revoke old object URL if it exists
    if (-1 !== index && sls.value[index].url) {
      URL.revokeObjectURL(sls.value[index].url!)
    }

    // create a Blob URL
    if (val.data && val.mimetype) {
      // store the blob in IndexedDB
      // await saveSLS(val) // TODO: re-enable once fixed

      // saveSLS(val).then(() => {}).catch((err) => {
      //   console.error('Failed to save SLS to IndexedDB', err)
      // })

      const blob = new Blob([new Uint8Array(val.data)], { type: val.mimetype })
      // oxlint-disable-next-line @typescript-eslint/no-unused-vars
      const { data, ...rest } = val
      val = { ...rest, url: URL.createObjectURL(blob) }
    }

    if (-1 !== index) {
      sls.value[index] = val
    } else {
      sls.value.push(val)
    }
  }

  return {
    // state
    connected,
    ensemble,
    services,
    selectedService,
    selectedSid,
    audioFormats,
    // settings
    ediHost,
    ediPort,
    // methods
    reset,
    selectService,
    // state updates
    updateEnsemble,
    setAudioFormat,
    updateDL,
    updateSLS,
  }
})
