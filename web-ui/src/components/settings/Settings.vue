<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { useColorMode } from '@vueuse/core'
import { usePlayerStore } from '@/stores/player'

import IconTheme from '@/components/icons/IconTheme.vue'
import Input from '@/components/ui/Input.vue'

const { volume } = storeToRefs(usePlayerStore())

const { store: theme } = useColorMode({
  attribute: 'data-theme',
  storageKey: 'ui:theme',
})

const toggleTheme = () => {
  theme.value = theme.value === 'dark' ? 'light' : 'dark'
}
</script>

<template>
  <div class="settings">
    <div :class="['theme', theme]">
      <IconTheme @click="toggleTheme()" :size="16" />
    </div>
    <div class="volume">
      <Input type="range" v-model="volume" min="0" max="1" step="0.0025" />
    </div>
    <div class="decoder">
      <span>Decoder:</span>
      <span>FAAD2Decoder</span>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.settings {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  padding: 0 0.5rem;
  > .theme {
    display: flex;
    align-items: center;
    > svg {
      cursor: pointer;
    }
  }
  > .volume {
    display: flex;
    align-items: center;
  }
  > .decoder {
    font-size: var(--t-fs-s);
    font-family: var(--t-family-mono);
    flex-grow: 1;
    gap: 0.25rem;
    display: flex;
    justify-content: flex-end;
  }
}
</style>
