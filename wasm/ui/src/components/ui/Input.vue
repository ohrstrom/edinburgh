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

  /* type based overrides */
  &[type='range'] {
    // width: 100%;
    border-width: 0;
    padding-left: 0;
    padding-right: 0;
    position: relative;

    -webkit-appearance: none;
    appearance: none;

    &::-webkit-slider-thumb {
      -webkit-appearance: none;
      appearance: none;
      width: 14px;
      height: 14px;
      background: currentColor;
      cursor: pointer;
    }

    &::before {
      content: '';
      position: absolute;
      left: 0px;
      right: 0px;
      height: 14px;
      background: rgba(0, 0, 0, 0.1);
    }
    &::after {
      content: '';
      position: absolute;
      top: 50%;
      left: 7px;
      right: 7px;
      height: 4px;
      background: currentColor;
      transform: translateY(-50%);
    }
  }
}
</style>
