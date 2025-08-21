<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    size?: number
    colorVar?: string
  }>(),
  {
    size: 32,
    colorVar: '--c-fg',
  },
)

const color = computed(() => `hsl(var(${props.colorVar}))`)

const style = computed(() => ({
  fill: color.value,
  width: `${props.size}px`,
  height: `${props.size}px`,
}))
</script>

<template>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    height="24px"
    width="24px"
    viewBox="0 -960 960 960"
    :style="style"
  >
    <!-- eslint-disable max-len -->
    <path
      d="M480-80q-83 0-156-31.5T197-197q-54-54-85.5-127T80-480q0-83 31.5-156T197-763q54-54 127-85.5T480-880q83 0 156 31.5T763-763q54 54 85.5 127T880-480q0 83-31.5 156T763-197q-54 54-127 85.5T480-80Zm40-83q119-15 199.5-104.5T800-480q0-123-80.5-212.5T520-797v634Z"
    />
    <!-- eslint-enable max-len -->
  </svg>
</template>
