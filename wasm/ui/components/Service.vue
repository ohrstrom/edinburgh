<script setup lang="ts">
interface Service {
  sid: number
  scid?: number
  label?: string
  short_label?: string
  isCurrent: boolean
}

defineProps<{ service: Service; isCurrent: boolean }>()
defineEmits<{
  (event: 'select', payload: { sid: number }): void
  (event: 'play', payload: { scid?: number }): void
}>()
</script>

<template>
  <div @click="$emit('select', { sid: service.sid })" class="service">
    <div>
      <button @click="$emit('play', { scid: service.scid })">
        {{ isCurrent ? 'S' : 'P' }}
      </button>
    </div>
    <div>
      <div>
        <span>{{ service?.label ?? '-' }}</span>
      </div>
      <div>
        <small>{{ service?.short_label ?? '-' }}</small>
      </div>
    </div>
    <div>
      <span>SID: {{ service.sid }}</span>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.service {
  display: grid;
  grid-template-columns: 32px 1fr 1fr;
  gap: 8px;
  padding: 8px;
  cursor: pointer;
  > div {
    display: flex;
    flex-direction: column;
    justify-content: center;
  }
}

button {
  width: 32px;
  height: 32px;
  cursor: pointer;
}
</style>
