import { ref } from 'vue'
import { useStorage } from '@vueuse/core'
import { defineStore } from 'pinia'

import type * as Types from '@/types'

export const usePlayerStore = defineStore('player', () => {
  const state = ref<Types.PlayerState>('stopped')

  const volume = useStorage('player/volume', 1, localStorage, {
    serializer: {
      read: (v: string) => Number(v),
      write: (v: number) => v.toString(),
    },
  })

  const setState = async (val: Types.PlayerState) => {
    state.value = val
  }

  return {
    state,
    volume,
    setState,
  }
})
