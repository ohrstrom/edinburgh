<script setup lang="ts">
interface Ensemble {
  eid: number
  label?: string
  short_label?: string
}

defineProps<{
  ensemble: Ensemble | undefined
}>()
</script>

<template>
  <div class="ensemble">
    <div>
      <h2>{{ ensemble?.label ?? '-' }}</h2>
      <small>{{ ensemble?.short_label ?? '-' }}</small>
    </div>
    <div>
      <span>{{ ensemble?.eid ?? '-' }}</span>
    </div>
  </div>
</template>
