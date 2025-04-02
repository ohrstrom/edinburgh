<script setup lang="ts">
const props = withDefaults(defineProps<{
  modelValue: string | number
  type?: 'text' | 'number' | 'url'
  disabled?: boolean
  variant?: 'default' | 'primary' | 'danger'
}>(), {
  type: 'text',
  variant: 'default',
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string | number): void
}>()

function onInput(e: Event) {
  const target = e.target as HTMLInputElement
  const value = props.type === 'number' ? Number(target.value) : target.value
  emit('update:modelValue', value)
}
</script>

<template>
  <input
      :type="type"
      :value="modelValue"
      :disabled="disabled"
      :class="['input', variant]"
      @input="onInput"
  />
</template>

<style lang="scss" scoped>
.input {
  display: inline-flex;
  min-height: 32px;
  padding: 0 0 0 1em;
  font-size: 0.875rem;
  align-items: center;

  &.default {
    background: white;
    color: currentColor;
    border: 1px solid currentColor;
  }

  &:disabled {
    background: #ccc;
    cursor: not-allowed;
    opacity: 0.7;
  }
}
</style>
