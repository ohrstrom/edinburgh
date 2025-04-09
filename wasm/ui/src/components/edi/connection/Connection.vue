<script setup lang="ts">
import { watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useEDIStore } from '@/stores/edi'

import Button from '@/components/ui/Button.vue'
import Input from '@/components/ui/Input.vue'

const { connected, ediHost, ediPort } = storeToRefs(useEDIStore())

const emit = defineEmits<{
  (event: 'connect', payload: { host: host; port: number }): void
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

watch(
  ediPort,
  (newVal, oldVal) => {
    if (connected.value && newVal !== oldVal) {
      console.debug('port changed:', newVal)
      reset()
      setTimeout(() => {
        connect()
      }, 500)
    }
  },
  { immediate: true },
)
</script>

<template>
  <div class="connection">
    <div class="settings">
      <Input type="text" v-model="ediHost" />
      <Input type="number" v-model="ediPort" />
    </div>
    <div class="actions">
      <Button @click="connect" :variant="connected ? 'default' : 'primary'">Connect</Button>
      <Button @click="reset">Reset</Button>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.connection {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 324px;
  .settings {
    display: grid;
    grid-template-columns: 1fr 100px;
    grid-gap: 0.5rem;
  }
  .actions {
    display: grid;
    grid-template-columns: 1fr 100px;
    grid-gap: 0.5rem;
  }
}
</style>
