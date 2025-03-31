<script setup lang="ts">

import {storeToRefs} from "pinia";
import {useEDIStore} from "@/stores/edi";

const {ediHost, ediPort} = storeToRefs(useEDIStore())

const emit = defineEmits<{
  (event: 'connect', payload: { port: number }): void
  (event: 'reset'): void
}>()

const connect = () => {
  const host = ediHost.value
  const port = Number(ediPort.value)
  if (port) {
    emit('connect', { host: host, port: port })
  } else {
    console.error('EDI Port is not set')
  }
}

const reset = () => {
  emit('reset')
}

</script>

<template>
  <div class="connection">
    <div>
      <input type="text" v-model="ediHost" />
      <input type="number" v-model="ediPort" />
    </div>
    <div>
      <button @click="connect">Connect</button>
      <button @click="reset">Reset</button>
    </div>
  </div>
</template>
