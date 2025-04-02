import { ref, computed } from 'vue'
import { useStorage } from '@vueuse/core'
import { defineStore } from 'pinia'

//import type { DL, Ensemble, SLS } from '@/types'

import type * as Types from '@/types'

export const useEDIStore = defineStore('edi', () => {

  const connected = ref(false)

  // const ensemble = ref<Ensemble | null>(null)
  const ensemble = ref<Types.Ensemble>({
    eid: 0,
    services: [],
  })

  const dls = ref<Types.DL[]>([])
  const sls = ref<Types.SLS[]>([])

  const selectedSid = ref(0)
  const selectedScid = ref(0)

  const services = computed(() => {
    if (!ensemble.value) return []
    return ensemble.value.services
      .filter((svc) => svc.label !== undefined)
      .map((svc) => {
        // const dl = dls.value.find(dl => dl.scid === svc.scid)
        return {
          ...svc,
          dl: dls.value.find((v) => v.scid === svc.scid),
          sls: sls.value.find((v) => v.scid === svc.scid),
        }
      })
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

  const ediPort = useStorage('edi/port', '8855', localStorage, {
    serializer: {
      read: (v) => Number(v),
      write: (v) => v,
    },
  })

  const reset = async () => {
    console.debug('STORE: reset')
    ensemble.value = {
      eid: 0,
      services: [],
    }
    dls.value = []
    sls.value = []
    selectedSid.value = 0
    selectedScid.value = 0
  }

  const selectService = async (sid: number) => {
    console.debug('STORE: selectService', sid)
    selectedSid.value = sid
    selectedScid.value = sid
  }

  const updateEnsemble = async (val: Types.Ensemble) => {
    console.debug('STORE: updateEnsemble', val)
    ensemble.value = val
  }

  const updateDL = async (val: Types.DL) => {
    const index = dls.value.findIndex((v) => v.scid === val.scid)
    if (index !== -1) {
      dls.value[index] = val
    } else {
      dls.value.push(val)
    }
  }

  const updateSLS = async (val: Types.SLS) => {
    const index = sls.value.findIndex((v) => v.scid === val.scid)

    // revoke old object URL if it exists
    if (index !== -1 && sls.value[index].url) {
      URL.revokeObjectURL(sls.value[index].url!)
    }

    // create a Blob URL
    if (val.data && val.mimetype) {
      const blob = new Blob([new Uint8Array(val.data)], { type: val.mimetype })
      const { data, ...rest } = val
      val = { ...rest, url: URL.createObjectURL(blob) }
    }

    if (index !== -1) {
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
    // settings
    ediHost,
    ediPort,
    // methods
    reset,
    selectService,
    // state updates
    updateEnsemble,
    updateDL,
    updateSLS,
  }
})
