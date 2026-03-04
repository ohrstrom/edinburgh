<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  kind: string | Record<string, unknown>
  value: string
}>()

const kindDisplay = computed(() => {
  /*
        example: PROGRAMME.HOMEPAGE
                 ITEM.ARTIST
                 ITEM.TITLE
    */
  if (typeof props.kind === 'string') {
    return props.kind.replace(/_/g, '.')
  }
  if (typeof props.kind === 'object' && props.kind !== null && !Array.isArray(props.kind)) {
    const key = Object.keys(props.kind)[0]
    if (key) {
      const value = props.kind[key]
      return `${key.toUpperCase()}.${value}`
    }
  }
  return 'UNKNOWN'
})

const kindDisplayShort = computed(() => {
  /*
        example: HOMEPAGE
                 ARTIST
                 TITLE
    */
  if (kindDisplay.value.includes('.')) {
    return kindDisplay.value.split('.').slice(1).join('.')
  }
  return kindDisplay.value
})
</script>

<template>
  <div class="dl-plus-display">
    <span class="kind" v-text="kindDisplayShort" :title="kindDisplay" />
    <span class="value" v-text="props.value" />
  </div>
</template>

<style lang="scss" scoped>
.dl-plus-display {
  a.value {
    text-decoration: underline;
  }
}
</style>
