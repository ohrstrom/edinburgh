<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    modelValue: string | number
    type?: 'text' | 'number' | 'url' | 'range'
    disabled?: boolean
    variant?: 'default' | 'primary' | 'danger'
  }>(),
  {
    type: 'text',
    variant: 'default',
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: string | number): void
}>()

function onInput(e: Event) {
  const target = e.target as HTMLInputElement
  const value = 'number' === props.type ? Number(target.value) : target.value
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
  font-family: var(--t-family-mono);
  font-size: var(--t-fs-m);
  align-items: center;
  border-radius: var(--b-r-s);

  &.default {
    background: hsl(var(--c-bg));
    color: currentColor;
    border: 1px solid currentColor;
  }

  &:disabled {
    background: #ccc;
    cursor: not-allowed;
    opacity: 0.7;
  }

  /* type based overrides */
  &[type='range'] {
    // width: 100%;
    border-width: 0;
    padding-left: 0;
    padding-right: 0;
    position: relative;
    background: transparent;

    appearance: none;
    -webkit-appearance: none;
    -moz-appearance: none;

    &::-webkit-slider-thumb {
      -webkit-appearance: none;
      appearance: none;
      width: 12px;
      height: 12px;
      background: currentColor;
      border-radius: var(--b-r-s);
      cursor: pointer;
    }

    &::before {
      content: '';
      position: absolute;
      left: 0px;
      right: 0px;
      height: 12px;
      background: rgba(0, 0, 0, 0);
    }

    &::after {
      content: '';
      position: absolute;
      top: 50%;
      left: 6px;
      right: 6px;
      height: 1px;
      background: currentColor;
      transform: translateY(-50%);
    }

    &::-moz-range-thumb {
      width: 12px;
      height: 12px;
      background: currentColor;
      border-radius: var(--b-r-s);
      cursor: pointer;
      border: none;
    }

    &::-moz-range-track {
      height: 1px;
      background: currentColor;
      border-radius: var(--b-r-s);
    }

    &::-moz-focus-outer {
      border: 0;
    }
  }
}
</style>
