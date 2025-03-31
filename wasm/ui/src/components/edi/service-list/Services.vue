<script setup lang="ts">

import {storeToRefs} from "pinia";
import {useEDIStore} from "@/stores/edi";

import Service from './Service.vue'

const ediStore = useEDIStore()
const { services } = storeToRefs(ediStore)


defineEmits<{
  (event: 'select', payload: { sid: number, scid: number }): void
  (event: 'play', payload: { scid?: number }): void
}>()

</script>

<template>
  <div class="services">
    <Service
        v-for="service in services"
        :key="service.scid"
        :service="service"
        @select="$emit('select', { sid: service.sid, scid: service.scid })"
        @play="$emit('play', { scid: service.scid })"
    />
  </div>
</template>

<style lang="scss" scoped>
.service {
  padding: 8px;
  &:nth-child(even) {
    background: #fafafa;
  }
}
</style>
