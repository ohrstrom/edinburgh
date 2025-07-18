<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { usePlayerStore } from '@/stores/player'

import Input from '@/components/ui/Input.vue'

const { volume } = storeToRefs(usePlayerStore())

/*
const emit = defineEmits<{
  (event: 'connect', payload: { host: host; port: number }): void
  (event: 'reset'): void
}>()

const connect = () => {}
*/
</script>

<template>
  <div class="settings">
    <Input type="range" v-model="volume" min="0" max="1" step="0.0025" />
  </div>
</template>

<style lang="scss" scoped>
.settings {
  display: flex;
}
</style>
