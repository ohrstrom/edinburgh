import { ref, computed } from "vue";
import { useStorage } from "@vueuse/core";
import { defineStore } from "pinia";
import type {DL, Ensemble, SLS} from "@/types";


export const useEDIStore = defineStore("edi", () => {

  // const ensemble = ref<Ensemble | null>(null)
  const ensemble = ref<Ensemble>({
    eid: 0,
    services: [],
  })

  const dls = ref<DL[]>([])
  const sls = ref<SLS[]>([])

  const selectedSid  = ref(0)
  const selectedScid  = ref(0)

  const services = computed(() => {
    if (!ensemble.value) return []
    return ensemble.value.services.filter((svc) => svc.label !== undefined).map((svc) => {
      // const dl = dls.value.find(dl => dl.scid === svc.scid)
      return {
        ...svc,
        dl: dls.value.find(v => v.scid === svc.scid),
        sls: sls.value.find(v => v.scid === svc.scid),
      }
    }).sort((a, b) => a.label!.localeCompare(b.label!))
  })

  const ediHost = useStorage("edi/host", "edi-ch.digris.net", localStorage, {
    serializer: {
      read: (v) => v,
      write: (v) => v,
    },
  });

  const ediPort = useStorage("edi/port", "8855", localStorage, {
    serializer: {
      read: (v) => Number(v),
      write: (v) => v,
    },
  });

  const selectService = async (s) => {
    console.debug("STORE: selectService", s)
    selectedSid.value = s.sid
    selectedScid.value = s.scid
  }

  const updateEnsemble = async (val: Ensemble) => {
    console.debug("STORE: updateEnsemble", val)
    ensemble.value = val
  }

  const updateDL = async (val: DL) => {
    const index = dls.value.findIndex(v => v.scid === val.scid)
    if (index !== -1) {
      dls.value[index] = val
    } else {
      dls.value.push(val)
    }
  }

  const updateSLS = async (val: SLS) => {
    const index = sls.value.findIndex(v => v.scid === val.scid)

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
    ensemble,
    services,
    selectedSid,
    // settings
    ediHost,
    ediPort,
    // methods
    selectService,
    // state updates
    updateEnsemble,
    updateDL,
    updateSLS,
  }
})
