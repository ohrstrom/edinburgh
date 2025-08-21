<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { useEDIStore } from '@/stores/edi'

import HexValue from '@/components/ui/HexValue.vue'

const ediStore = useEDIStore()
const { ensemble } = storeToRefs(ediStore)
</script>

<template>
  <div v-if="ensemble.eid" class="ensemble">
    <div class="info">
      <div class="info-section ens">
        <h2 class="label">{{ ensemble?.label ?? 'probing' }}</h2>
        <div>
          <span>{{ ensemble?.short_label ?? '' }}</span>
          <span v-if="ensemble?.short_label">&nbsp;•&nbsp;</span>
          <HexValue :value="ensemble.eid" />
        </div>
      </div>
      <div class="info-section services">
        <span v-if="ensemble.services.length">Services: {{ ensemble.services.length }}</span>
      </div>
    </div>
  </div>
  <div v-else class="ensemble ensemble--skeleton">
    <div class="info">
      <span class="message">not connected</span>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.ensemble {
  > .info {
    .ens {
      margin-bottom: 16px;
      > .label {
        margin-bottom: 8px;
        font-size: var(--t-fs-l);
      }
    }
    .services {
      display: none;
      font-size: var(--t-fs-s);
      background: transparent;
    }
  }
  &--skeleton {
    font-family: var(--t-family-mono);
    > .info {
      .message {
        display: inline-flex;
        color: hsl(var(--c-fg));
        padding: 2px 4px;
        font-size: var(--t-fs-s);
      }
    }
  }
}
</style>
