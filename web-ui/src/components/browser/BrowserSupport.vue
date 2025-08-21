<script setup lang="ts">
import type * as Types from '@/types'

const props = defineProps<{ browser: Types.Browser }>()
</script>
<template>
  <div class="browser-support">
    <h2 class="title">Browser not supported</h2>
    <div v-if="props.browser.missing.length" class="capabilities">
      <h3 class="title">Missing capabilities:</h3>
      <div class="missings">
        <div class="missing" v-for="capability in props.browser.missing" :key="`cap-${capability}`">
          {{ capability }}
        </div>
      </div>
    </div>
  </div>
</template>
<style lang="scss" scoped>
.browser-support {
  padding: 1rem 0.5rem 0.5rem;
  > .title {
    font-size: var(--t-fs-xl);
  }
  > .capabilities {
    margin-top: 0.25rem;
    > .title {
      font-size: var(--t-fs-m);
      font-family: var(--t-family-mono);
    }
    > .missings {
      font-size: var(--t-fs-m);
      font-family: var(--t-family-mono);

      display: flex;
      gap: 0.25rem;
      margin-top: 0.5rem;

      > .missing {
        padding: 2px 8px;
        background: hsl(var(--c-bg) / 0.25);
        border-radius: var(--b-r-s);
      }
    }
  }
}
</style>
