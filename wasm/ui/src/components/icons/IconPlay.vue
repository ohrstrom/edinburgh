<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  size?: number
  colorVar?: string
}>(), {
  size: 32,
  colorVar: '--c-fg',
})

const color = computed(() => `hsl(var(${props.colorVar}))`)

const style = computed(() => ({
  fill: 'transparent',
  stroke: color.value,
  strokeWidth: 3,
  strokeMiterlimit: 10,
  width: `${props.size}px`,
  height: `${props.size}px`,
}))
</script>

<template>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    height="48px"
    width="48px"
    viewBox="0 0 48 48"
    :style="style"
  >
    <polygon points="34.7,24 17.3,32.7 17.3,15.3 " />
  </svg>
</template>
