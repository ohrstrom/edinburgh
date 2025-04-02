<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { useEDIStore } from '@/stores/edi'

import Service from './Service.vue'

const ediStore = useEDIStore()
const { services } = storeToRefs(ediStore)

defineEmits<{
  (event: 'select', sid: number): void
  (event: 'play', sid: number): void
}>()
</script>

<template>
  <div class="services">
    <Service
      v-for="service in services"
      :key="`service-${service.scid}`"
      :service="service"
      @select="$emit('select', $event)"
      @play="$emit('play', $event)"
    />
  </div>
</template>

<style lang="scss" scoped>
.services {
  min-height: 80px;
  max-height: 600px;
  overflow-y: scroll;
}
.service {
  padding: 8px;
  &:nth-child(even) {
    background: #fafafa;
  }
}
</style>
