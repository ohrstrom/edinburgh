<script setup lang="ts">
import {computed} from "vue";

const props = defineProps<{ value: number }>()


const display = computed(() => {
  const val = Number(props.value)
  return '0x' + val.toString(16).toUpperCase()
})

</script>

<template>
  <span class="hex">
    {{ display }}
  </span>
</template>

<style lang="scss" scoped>
.hex {
  display: inline-flex;
}
</style>
