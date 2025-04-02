<script setup lang="ts">
withDefaults(defineProps<{
  type?: 'button' | 'submit' | 'reset'
  disabled?: boolean
  variant?: 'default' | 'primary' | 'danger'
}>(), {
  type: 'button',
  variant: 'default',
})

defineEmits<{
  (e: 'click', event: MouseEvent): void
}>()
</script>

<template>
  <button
      :type="type ?? 'button'"
      :disabled="disabled"
      :class="['button', variant]"
      @click="$emit('click', $event)"
  >
    <slot />
  </button>
</template>

<style lang="scss" scoped>
.button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 32px;
  padding: 0 1em;
  font-size: 0.875rem;
  cursor: pointer;
  border: none;
  transition: background 0.2s ease;

  &.default {
    background: white;
    color: currentColor;
    border: 1px solid currentColor;
    box-shadow: 2px 2px currentColor;
  }

  &.primary {
    background: #ffeb00;
    border: 1px solid currentColor;
    box-shadow: 2px 2px currentColor;
  }

  &.danger {
    background: #f36d57;
    border: 1px solid currentColor;
    box-shadow: 2px 2px currentColor;
  }

  &:disabled {
    background: #ccc;
    cursor: not-allowed;
    opacity: 0.7;
  }
}
</style>
