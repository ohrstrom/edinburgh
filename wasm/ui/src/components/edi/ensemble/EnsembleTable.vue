<script setup lang="ts">
import {computed} from 'vue'
import { storeToRefs } from 'pinia'
import { useEDIStore } from '@/stores/edi'

import HexValue from '@/components/ui/HexValue.vue'

const ediStore = useEDIStore()
const { ensemble } = storeToRefs(ediStore)

defineEmits<{
  (event: 'select', sid: number): void
}>()

const services = computed(() => {
  return (ensemble.value?.services ?? []).sort((a, b) => a.scid > b.scid ? 1 : -1)
})

</script>

<template>
  <div class="ensemble-table">
    <!-- 
    <pre v-text="ensemble" />
    -->
    <div class="table">
      <div class="service" v-for="(svc, index) in services ?? []" :key="`table-svc-${index}`" @click.prevent="$emit('select', svc.sid)">
        <span class="scid">{{ svc.scid }}</span>
        <HexValue class="sid" :value="svc.sid" />
        <span class="label">{{ svc?.label ?? '-' }}</span>
        <span class="short-label">{{ svc?.short_label ?? '-' }}</span>
        <span class="language">{{ svc?.language ?? '-' }}</span>
        <span class="language">
          <span v-if="svc?.subchannel">
            Start: {{ String(svc.subchannel.start).padStart(3, '0') }}
            CU: {{ svc.subchannel.size }}
            PL: {{ svc.subchannel.pl }}
          </span>
        </span>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.table {
  font-size: 0.75rem;
  .service {
    display: grid;
    grid-template-columns: 32px 1fr 1fr 1fr 1fr 2fr;
    gap: 8px;
    padding: 2px 8px;
    cursor: pointer;
    &:hover {
      background: hsl(var(--c-muted));
    }

    > .scid {
      text-align: end;
    }
  }
}
</style>