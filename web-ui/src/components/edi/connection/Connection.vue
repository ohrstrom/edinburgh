<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { storeToRefs } from 'pinia'
import { useEDIStore } from '@/stores/edi'

import Button from '@/components/ui/Button.vue'
import Input from '@/components/ui/Input.vue'

const { connected, ediHost, ediPort } = storeToRefs(useEDIStore())

type EDIHashConfig = {
  host?: string
  port?: number
}

const emit = defineEmits<{
  (event: 'connect', payload: { host: string; port: number }): void
  (event: 'reset'): void
}>()

function parseEDIHash(hash: string): EDIHashConfig | null {
  if (!hash) return null

  const raw = hash.startsWith('#') ? hash.slice(1) : hash
  if (!raw.startsWith('edi://')) return null

  try {
    const url = new URL(raw)

    const host = url.hostname || undefined
    const port = url.port ? Number(url.port) : undefined

    if (!host && !port) return null

    return { host, port }
  } catch {
    return null
  }
}

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

const hash = ref(window.location.hash)

const onHashChange = () => {
  hash.value = window.location.hash
}

onMounted(() => {
  window.addEventListener('hashchange', onHashChange)
  hash.value = window.location.hash
})

onUnmounted(() => {
  window.removeEventListener('hashchange', onHashChange)
})

watch(
  hash,
  (newHash) => {
    console.log('Hash changed:', newHash)

    const { host, port } = parseEDIHash(newHash) ?? {}

    if (!host || !port) return

    if (ediHost.value === host && ediPort.value === port) return

    ediHost.value = host
    ediPort.value = port
  },
  { immediate: true },
)

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
      <Button @click="connect" :disabled="connected" :variant="connected ? 'default' : 'primary'"
        >Connect</Button
      >
      <Button @click="reset" :disabled="!connected">Reset</Button>
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
