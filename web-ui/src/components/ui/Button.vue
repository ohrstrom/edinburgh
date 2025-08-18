<script setup lang="ts">
withDefaults(
  defineProps<{
    type?: 'button' | 'submit' | 'reset'
    disabled?: boolean
    variant?: 'default' | 'primary' | 'danger'
  }>(),
  {
    type: 'button',
    variant: 'default',
  },
)

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
  font-family: var(--t-family-mono);
  font-weight: var(--t-fw-b);
  font-size: var(--t-fs-m);
  cursor: pointer;
  border: none;
  border-radius: var(--b-r-s);
  transition: background 0.2s ease;

  &.default {
    background: hsl(var(--c-bg));
    color: currentColor;
    border: 1px solid currentColor;
    box-shadow: 2px 2px hsl(var(--c-shadow));
  }

  &.primary {
    background: hsl(var(--c-cta));
    color: hsl(var(--c-cta-fg));
    border: 1px solid currentColor;
    box-shadow: 2px 2px hsl(var(--c-shadow));
  }

  &.danger {
    background: #f36d57;
    border: 1px solid currentColor;
    box-shadow: 2px 2px hsl(var(--c-shadow));
  }

  &:disabled {
    background: #ccc;
    cursor: not-allowed;
    opacity: 0.7;
  }
}
</style>
